// SPDX-License-Identifier: GPL-3.0-only
// Test-only ASIO host: four distinct playback pilots, four captured inputs.
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
static ASIOBufferInfo buffers[8]{};
static long frames=0;
static double rate=0;
static std::vector<float> capture;
static std::atomic<size_t> cursor{0};
static std::atomic<long> resets{0},resyncs{0},overloads{0},rate_changes{0};
static unsigned long long generated=0;
static bool ready_supported=false;
static constexpr double tones[4]={997,1213,1613,2111};
static void process(long index){
    const auto begin=cursor.load();
    for(long n=0;n<frames;n++){
        for(int ch=0;ch<4;ch++){
            if((begin+n)*4+ch<capture.size())
                capture[(begin+n)*4+ch]=(float)((double)((long*)buffers[ch].buffers[index])[n]/2147483648.0);
            double pilot=.01*sin(2*3.141592653589793*tones[ch]*(generated+n)/rate);
            if(generated+n<rate*.1)pilot*=(generated+n)/(rate*.1);
            ((long*)buffers[ch+4].buffers[index])[n]=(long)(pilot*2147483647.0);
        }
    }
    generated+=frames;cursor.store(begin+frames);
    if(ready_supported)driver->outputReady();
}
static void buffer_switch(long index,ASIOBool){process(index);}
static ASIOTime* time_switch(ASIOTime* time,long index,ASIOBool){process(index);return time;}
static void rate_changed(ASIOSampleRate){rate_changes++;}
static long message(long selector,long value,void*,double*){
    if(selector==kAsioSelectorSupported)return value==kAsioEngineVersion||value==kAsioSupportsTimeInfo||value==kAsioResetRequest||value==kAsioResyncRequest||value==kAsioOverload;
    if(selector==kAsioEngineVersion)return 2;
    if(selector==kAsioSupportsTimeInfo)return 1;
    if(selector==kAsioResetRequest){resets++;return 0;}
    if(selector==kAsioResyncRequest){resyncs++;return 1;}
    if(selector==kAsioOverload){overloads++;return 1;}
    return 0;
}
static void check(ASIOError code,const char* operation){if(code!=ASE_OK&&code!=ASE_SUCCESS)throw std::runtime_error(operation);}
int main(int argc,char** argv){
    if(argc!=2){fprintf(stderr,"Provide a temporary capture path\n");return 2;}
    bool created=false,started=false;int result=0;
    CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED);
    try{
        CLSID clsid{};CLSIDFromString(L"{E7E92954-EA17-433a-9142-EE419BE4875C}",&clsid);
        if(FAILED(CoCreateInstance(clsid,nullptr,CLSCTX_INPROC_SERVER,clsid,(void**)&driver)))throw std::runtime_error("ASIO COM creation failed");
        if(!driver->init(GetDesktopWindow()))throw std::runtime_error("ASIO init failed");
        long in=0,out=0,min=0,max=0,gran=0;
        check(driver->getChannels(&in,&out),"getChannels");check(driver->getSampleRate(&rate),"getSampleRate");
        check(driver->getBufferSize(&min,&max,&frames,&gran),"getBufferSize");
        if(in!=4||out!=4||rate!=96000||frames<16||frames>8192)throw std::runtime_error("Unexpected configuration");
        for(int i=0;i<8;i++){
            ASIOChannelInfo info{};info.channel=i%4;info.isInput=i<4;
            check(driver->getChannelInfo(&info),"getChannelInfo");
            if(info.type!=ASIOSTInt32LSB)throw std::runtime_error("Unsupported sample type");
            buffers[i].channelNum=i%4;buffers[i].isInput=i<4;
        }
        capture.resize((size_t)rate*5*4);
        ASIOCallbacks callbacks{buffer_switch,rate_changed,message,time_switch};
        check(driver->createBuffers(buffers,8,frames,&callbacks),"createBuffers");created=true;
        ready_supported=driver->outputReady()==ASE_OK;
        for(auto& buffer:buffers)for(auto ptr:buffer.buffers)ZeroMemory(ptr,frames*sizeof(long));
        check(driver->start(),"start");started=true;
        const auto start=GetTickCount64();
        while(GetTickCount64()-start<3000){MSG msg;while(PeekMessage(&msg,nullptr,0,0,PM_REMOVE)){TranslateMessage(&msg);DispatchMessage(&msg);}Sleep(5);}
        check(driver->stop(),"stop");started=false;
        const auto count=cursor.load();if(count*4>capture.size())throw std::runtime_error("Capture overrun");
        FILE* file=nullptr;fopen_s(&file,argv[1],"wb");if(!file)throw std::runtime_error("Capture open failed");
        const auto written=fwrite(capture.data(),sizeof(float),count*4,file);fclose(file);
        if(written!=count*4)throw std::runtime_error("Capture write failed");
        printf("{\"frames\":%zu,\"rate\":%.0f,\"buffer_frames\":%ld,\"resets\":%ld,\"resyncs\":%ld,\"overloads\":%ld,\"rate_changes\":%ld}\n",count,rate,frames,resets.load(),resyncs.load(),overloads.load(),rate_changes.load());
    }catch(const std::exception& error){fprintf(stderr,"%s\n",error.what());result=1;}
    if(started)driver->stop();if(created)driver->disposeBuffers();if(driver)driver->Release();CoUninitialize();return result;
}
