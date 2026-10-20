// SPDX-License-Identifier: GPL-3.0-only
// Bounded native ASIO experiment. Uses current driver rate/buffer configuration.
#include <windows.h>
#include <objbase.h>
#include <cstdio>
#include <cstdlib>
#include <cmath>
#include <vector>
#include <atomic>
#include <stdexcept>
#include "asio/iasiodrv.h"

static IASIO* driver=nullptr;
static ASIOBufferInfo buffers[4]{};
static long frames=0;
static double rate=0;
static std::vector<float> capture;
static std::atomic<size_t> cursor{0};
static std::atomic<long> callbacks{0}, resets{0}, resyncs{0}, overloads{0}, rate_changes{0}, position_gaps{0};
static unsigned long long output_position=0, previous_position=0;
static bool previous_valid=false;
static bool ready_supported=false;
static unsigned long long sample_position(ASIOSamples value){return ((unsigned long long)value.hi<<32)|value.lo;}
static void process(long index,ASIOTime* timing){
    size_t begin=cursor.load();
    if(timing && (timing->timeInfo.flags & kSamplePositionValid)){
        auto now=sample_position(timing->timeInfo.samplePosition);
        if(previous_valid && now!=previous_position+(unsigned long long)frames) position_gaps++;
        previous_valid=true; previous_position=now;
    }
    auto* left=(long*)buffers[0].buffers[index];
    auto* right=(long*)buffers[1].buffers[index];
    auto* outleft=(long*)buffers[2].buffers[index];
    auto* outright=(long*)buffers[3].buffers[index];
    for(long n=0;n<frames;n++){
        if((begin+n)*2+1<capture.size()){
            capture[(begin+n)*2]=(float)((double)left[n]/2147483648.0);
            capture[(begin+n)*2+1]=(float)((double)right[n]/2147483648.0);
        }
        const double pos=(double)(output_position+n);
        double pilot=.0025*(sin(2*3.141592653589793*997*pos/rate)+sin(2*3.141592653589793*1613*pos/rate));
        if(pos<rate*.1) pilot*=pos/(rate*.1);
        outleft[n]=outright[n]=(long)(pilot*2147483647.0);
    }
    output_position+=frames;
    cursor.store(begin+frames);
    callbacks++;
    if(ready_supported) driver->outputReady();
}
static void switch_buffer(long index,ASIOBool){process(index,nullptr);}
static ASIOTime* time_buffer(ASIOTime* time,long index,ASIOBool){process(index,time);return time;}
static void changed_rate(ASIOSampleRate){rate_changes++;}
static long message(long selector,long value,void*,double*){
    if(selector==kAsioSelectorSupported) return value==kAsioEngineVersion||value==kAsioSupportsTimeInfo||value==kAsioResetRequest||value==kAsioResyncRequest||value==kAsioOverload;
    if(selector==kAsioEngineVersion) return 2;
    if(selector==kAsioSupportsTimeInfo) return 1;
    if(selector==kAsioResetRequest) {resets++; return 0;}
    if(selector==kAsioResyncRequest) {resyncs++; return 1;}
    if(selector==kAsioOverload) {overloads++; return 1;}
    return 0;
}
static void check(ASIOError error,const char* name){if(error!=ASE_OK && error!=ASE_SUCCESS){char msg[256];sprintf_s(msg,"%s failed: %ld",name,error);throw std::runtime_error(msg);}}
int main(int argc,char** argv){
    bool buffers_created=false,started=false;
    int result=0;
    CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED);
    try{
        unsigned long duration_ms=12500;
        if(argc>2){
            char* end=nullptr;
            duration_ms=strtoul(argv[2],&end,10);
            if(!end||*end||duration_ms<10000||duration_ms>120000)throw std::runtime_error("Duration must be 10000..120000 milliseconds");
        }
        CLSID clsid{};
        if(FAILED(CLSIDFromString(L"{E7E92954-EA17-433a-9142-EE419BE4875C}",&clsid)))throw std::runtime_error("CLSID parsing failed");
        HRESULT status=CoCreateInstance(clsid,nullptr,CLSCTX_INPROC_SERVER,clsid,(void**)&driver);
        if(FAILED(status)){char msg[128];sprintf_s(msg,"CoCreateInstance failed: 0x%08lx",status);throw std::runtime_error(msg);}
        if(!driver->init(GetDesktopWindow()))throw std::runtime_error("ASIO init failed");
        long inputs=0,outputs=0,min=0,max=0,granularity=0,inlat=0,outlat=0;
        check(driver->getChannels(&inputs,&outputs),"getChannels");
        check(driver->getSampleRate(&rate),"getSampleRate");
        check(driver->getBufferSize(&min,&max,&frames,&granularity),"getBufferSize");
        driver->getLatencies(&inlat,&outlat);
        char name[64]{};driver->getDriverName(name);
        printf("{\"event\":\"inventory\",\"driver\":\"%s\",\"version\":%ld,\"rate\":%.0f,\"inputs\":%ld,\"outputs\":%ld,\"buffer_frames\":%ld,\"input_latency\":%ld,\"output_latency\":%ld}\n",name,driver->getDriverVersion(),rate,inputs,outputs,frames,inlat,outlat);
        for(int input=0;input<2;input++)for(long channel=0;channel<(input?inputs:outputs);channel++){
            ASIOChannelInfo info{}; info.channel=channel;info.isInput=input;
            check(driver->getChannelInfo(&info),"getChannelInfo");
            printf("{\"event\":\"channel\",\"input\":%d,\"index\":%ld,\"type\":%ld,\"name\":\"%s\"}\n",input,channel,info.type,info.name);
            if(argc>1 && channel>=2 && channel<=3 && info.type!=ASIOSTInt32LSB)throw std::runtime_error("Selected channel sample type unsupported");
        }
        fflush(stdout);
        if(argc>1){
            if(inputs<4||outputs<4||rate!=96000||frames<16||frames>8192)throw std::runtime_error("Unexpected configuration; no stream started");
            capture.resize((size_t)(rate*(duration_ms/1000.0+2))*2);
            for(int i=0;i<4;i++){buffers[i].isInput=i<2;buffers[i].channelNum=2+(i%2);}
            ASIOCallbacks cb{switch_buffer,changed_rate,message,time_buffer};
            check(driver->createBuffers(buffers,4,frames,&cb),"createBuffers");buffers_created=true;
            ready_supported=driver->outputReady()==ASE_OK;
            for(auto& b:buffers)for(auto ptr:b.buffers)ZeroMemory(ptr,frames*sizeof(long));
            check(driver->start(),"start");started=true;
            printf("{\"event\":\"started\"}\n");fflush(stdout);
            ULONGLONG start=GetTickCount64();
            while(GetTickCount64()-start<duration_ms){MSG msg;while(PeekMessage(&msg,nullptr,0,0,PM_REMOVE)){TranslateMessage(&msg);DispatchMessage(&msg);}Sleep(5);}
            check(driver->stop(),"stop");started=false;
            FILE* file=nullptr;fopen_s(&file,argv[1],"wb");if(!file)throw std::runtime_error("Cannot open temporary capture");
            size_t count=cursor.load();if(count*2>capture.size())throw std::runtime_error("Capture capacity exceeded");
            fwrite(capture.data(),sizeof(float),count*2,file);fclose(file);
            printf("{\"event\":\"complete\",\"frames\":%zu,\"callbacks\":%ld,\"resets\":%ld,\"resyncs\":%ld,\"overloads\":%ld,\"rate_changes\":%ld,\"position_gaps\":%ld,\"position_available\":%s}\n",count,callbacks.load(),resets.load(),resyncs.load(),overloads.load(),rate_changes.load(),position_gaps.load(),previous_valid?"true":"false");
        }
    }catch(const std::exception& e){fprintf(stderr,"%s\n",e.what());result=1;}
    if(started)driver->stop();
    if(buffers_created)driver->disposeBuffers();
    if(driver)driver->Release();
    CoUninitialize();return result;
}
