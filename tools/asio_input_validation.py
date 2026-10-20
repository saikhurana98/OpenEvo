"""Measure physical-input matrix columns using simultaneous ASIO captures.

Default is a read-only baseline. --change uses the production Rust setter and
restores OEM saved intentions corroborated by the baseline. Raw audio is deleted.
"""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from probe import Evo

parser = argparse.ArgumentParser()
parser.add_argument('--change', action='store_true')
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--original-pan', type=int, nargs=2, choices=[-100,0,100], default=[-100,100])
args = parser.parse_args()
temporary = ROOT/'evidence/input-matrix-temporary.bin'
report = {'kind':'Physical input matrix measured against simultaneous ASIO inputs',
          'raw_audio_retained':False, 'hardware_matrix_readback_available':False,
          'events':[], 'restoration_errors':[], 'passed':False}
originals = {source:(-14.75,args.original_pan[source],False) for source in (0,1)}
baseline_db = [[-128,-128],[-128,-128]]
for source, pan in enumerate(args.original_pan):
    if pan == 0:baseline_db[0][source]=baseline_db[1][source]=-20.75
    else:baseline_db[int(pan>0)][source]=-14.75
touched = set()


def apply(source, route):
    gain, pan, muted = route
    return json.loads(subprocess.check_output(
        [str(ROOT/'target/release/examples/matrix_probe.exe'),str(source),str(gain),
         str(pan),str(int(muted))], text=True, timeout=15))


def capture(expected, selected_lag=None):
    try:
        host = json.loads(subprocess.check_output(
            [str(ROOT/'tools/asio_matrix_host.exe'),str(temporary)],text=True,timeout=15))
        if any(host[k] for k in ['resets','resyncs','overloads','rate_changes']):
            raise RuntimeError(f'ASIO event: {host}')
        audio = np.fromfile(temporary,dtype=np.float32).reshape(-1,4)[96000:-9600].astype(float)
        if len(audio)<96000: raise RuntimeError('Short capture')
        input_rms = np.sqrt(np.mean(audio[:,:2]**2,axis=0))
        if np.any(input_rms<.002): raise RuntimeError(f'Physical signal too quiet: {input_rms}')
        if np.any(np.abs(audio[:,:2])>.999): raise RuntimeError('Physical input clipped')
        positions = np.arange(len(audio))/96000
        tones = np.column_stack([fn(2*np.pi*f*positions) for f in [997,1213,1613,2111]
                                 for fn in (np.sin,np.cos)]+[np.ones(len(audio))])
        y = audio[32:-32,2:]
        candidates = [selected_lag] if selected_lag is not None else [0,1,2,3,4,8,16,32,-1,-2,-4,-8,-16,-32]
        best = None
        for lag in candidates:
            x = np.column_stack([audio[32-lag:len(audio)-32-lag,:2],tones[32:-32]])
            coefficients = np.linalg.lstsq(x,y,rcond=None)[0]
            residual = np.sqrt(np.mean((y-x@coefficients)**2,axis=0))
            candidate = (float(np.sum(residual)),lag,coefficients[:2].T,residual)
            if best is None or candidate[0]<best[0]:best=candidate
            if candidate[0]<2e-6:break
        _, lag, actual, residual = best
        wanted = np.array([[0 if db<=-120 else 10**(db/20) for db in row] for row in expected])
        match = bool(np.all(np.abs(actual-wanted)<=np.maximum(np.abs(wanted)*.03,.002)))
        clean = bool(np.all(residual<=np.maximum(np.sqrt(np.mean(y**2,axis=0))*.03,3e-6)))
        return {'host':host,'input_rms':input_rms.tolist(),'input_peak':np.max(np.abs(audio[:,:2]),axis=0).tolist(),
                'relative_delay_samples':lag,'coefficients_destination_by_source':actual.tolist(),
                'expected_db_destination_by_source':expected,'residual_rms':residual.tolist(),
                'coefficients_match':match,'clean_residual':clean,'passed':match and clean}
    finally:
        temporary.unlink(missing_ok=True)


try:
    asio = Path('C:/Program Files/Audient/USBAudioDriver/x64/audientusbaudioasio_x64.dll')
    if hashlib.sha256(asio.read_bytes()).hexdigest()!='aa893589ec86ba9ec225c98ca4a03ffa964088ee089ea3fd955ad7fb08177acd':
        raise RuntimeError('ASIO DLL changed')
    with Evo() as evo:
        report['before']=evo.snapshot()
        if report['before']['sample_rate']!=96000:raise RuntimeError('Requires current 96 kHz')
        if [evo.control(58,0,c,C.c_uint32) for c in (0,1)]!=[0,0]:raise RuntimeError('48V must be off')
        if not evo.control(59,1,0,C.c_uint32) and max(report['before']['output_raw'])>-60*256:
            raise RuntimeError('Physical outputs must be muted or below -60 dB')
        if evo.control(58,5,0,C.c_uint32) in (2,3):raise RuntimeError('Smartgain is active')
    preset=ET.parse(Path.home()/'AppData/Roaming/Audient/EVO/state.xml')
    saved={int(x.attrib['channel']):x.find('mix').attrib for x in preset.findall('.//mixer/input')}
    if any(float(saved[i]['volume'])!=-14.75 or float(saved[i]['pan'])!=i for i in (0,1)):
        raise RuntimeError('OEM saved input routing differs from known restoration')
    if any(float(saved[i]['volume'])>-120 for i in (2,3)):
        raise RuntimeError('Main playback sources must be muted in the known baseline')
    baseline=capture(baseline_db)
    report['baseline']=baseline
    if not baseline['passed']:raise RuntimeError('Baseline does not corroborate saved routing')
    report['restoration_provenance']='Saved gain plus explicitly selected pan, corroborated by simultaneous baseline audio; actual register readback unavailable'
    report['original_routes']=originals
    report['oem_saved_pan']=[float(saved[i]['pan']) for i in (0,1)]
    if args.change:
        for source in (0,1):
            other=1-source
            touched.add(other)
            apply(other,(0,0,True))
            for name,route,db in [('hard-pan',(-6,(-100 if source==0 else 100),False),(-6,-128) if source==0 else (-128,-6)),
                                  ('center',(-6,0,False),(-12,-12)),('mute',(-6,0,True),(-128,-128))]:
                touched.add(source)
                acknowledgement=apply(source,route)
                wanted=[[-128,-128],[-128,-128]]
                wanted[0][source],wanted[1][source]=db
                measurement=capture(wanted,baseline['relative_delay_samples'])
                report['events'].append({'source':source,'case':name,'write':acknowledgement,'measurement':measurement})
                if not measurement['passed']:raise RuntimeError(f'Source {source} {name} did not match')
            apply(source,originals[source]);apply(other,originals[other])
        report['restored_measurement']=capture(baseline_db,baseline['relative_delay_samples'])
        if not report['restored_measurement']['passed']:raise RuntimeError('Restored signal pattern did not match')
    report['passed']=True
except BaseException as error:
    report['error']=str(error)
finally:
    for source in sorted(touched):
        try:apply(source,originals[source])
        except Exception as error:report['restoration_errors'].append(f'{source}: {error}')
    if report['restoration_errors']:report['passed']=False
    temporary.unlink(missing_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':report['passed'],'cases':len(report['events']),
                      'error':report.get('error'),'restoration_errors':report['restoration_errors'],
                      'baseline':report.get('baseline')},indent=2))
sys.exit(0 if report['passed'] else 1)
