"""WASAPI playback -> EVO hardware loopback continuity experiment.

Only aggregate measurements are saved; captured audio stays in memory.
"""
import argparse
import ctypes as C
import json
import time
from pathlib import Path
import numpy as np
import sounddevice as sd
from probe import Evo

def endpoint(name, direction):
    matches = [d for d in sd.query_devices()
               if d['name'] == name and d['max_' + direction + '_channels'] >= 2
               and sd.query_hostapis(d['hostapi'])['name'] == 'Windows WASAPI']
    if len(matches) != 1:
        raise RuntimeError(f'Expected one {direction} endpoint: {name}')
    return matches[0]

def measure(audio, rate):
    # Fixed-frequency least-squares fit in 20-ms windows. A dropout or clock
    # discontinuity changes amplitude, phase, or residual error.
    width = rate // 50
    blocks = len(audio) // width
    if blocks < 10:
        raise RuntimeError('Insufficient capture')
    n = np.arange(width)
    basis = np.column_stack([f(2*np.pi*hz*n/rate)
                             for hz in (997, 1613) for f in (np.sin, np.cos)]
                            + [np.ones(width)])
    inv = np.linalg.pinv(basis)
    data = audio[:blocks*width].reshape(blocks, width, 2).astype(np.float64)
    coefficients = np.einsum('ij,bjk->bik', inv, data)
    fitted = np.einsum('ij,bjk->bik', basis, coefficients)
    amplitudes = np.sqrt(coefficients[:, 0::2][:, :2]**2 + coefficients[:, 1::2]**2)
    phases = np.angle(coefficients[:, [0, 2]] + 1j*coefficients[:, [1, 3]])
    expected = np.arange(blocks)[:, None, None] * (2*np.pi*np.array([997,1613])[None,:,None]*width/rate)
    phase = np.angle(np.exp(1j*(phases-expected)))
    jumps = np.abs(np.angle(np.exp(1j*np.diff(phase, axis=0))))
    residual = np.sqrt(np.mean((data-fitted)**2, axis=1))
    rms = np.sqrt(np.mean(data**2, axis=1))
    median_amp = np.median(amplitudes, axis=0)
    pilot_present = bool(np.all(median_amp > 0.0001))
    weak = np.any(amplitudes < median_amp[None,:,:]*0.5, axis=(1,2))
    return {'window_ms':20, 'windows':blocks, 'pilot_present':pilot_present,
            'median_pilot_amplitudes':median_amp.tolist(),
            'weak_pilot_windows':int(np.count_nonzero(weak)),
            'max_phase_step_radians':float(np.max(jumps)),
            'phase_steps_above_0_1_rad':int(np.count_nonzero(np.any(jumps>0.1,axis=(1,2)))),
            'max_residual_rms':np.max(residual,axis=0).tolist(),
            'median_rms':np.median(rms,axis=0).tolist(),
            'max_residual_fraction':float(np.max(residual/np.maximum(rms,1e-10)))}

def clean_measurement(measurement):
    return (measurement['pilot_present']
            and measurement['weak_pilot_windows'] == 0
            and measurement['phase_steps_above_0_1_rad'] == 0
            and measurement['max_residual_fraction'] < 0.01)

def finish_capture(evo, original, touched, result, captured, rate, counters, callback_count):
    """Attempt all restoration and evidence steps even when an earlier step fails."""
    if touched:
        failures = []
        for channel, value in enumerate(original['output_raw'], 1):
            try:
                evo.control(10, 2, channel, C.c_int16, value)
            except Exception as error:
                failures.append(f'Channel {channel}: {error}')
        if failures:
            result['restoration_errors'] = failures
    try:
        result['after'] = evo.snapshot()
        result['state_restored'] = result['after'] == original
    except Exception as error:
        result['state_restored'] = False
        result['after_error'] = str(error)
    result['capture_frames'] = counters[0]
    result['callbacks'] = callback_count[0]
    try:
        result['measurement'] = measure(captured[rate:counters[0]-rate//4], rate)
    except Exception as error:
        result['measurement_error'] = str(error)
    result['callback_flags_after_warmup'] = [f for f in result['callback_flags'] if f['frame'] >= rate]
    result['passed'] = bool('error' not in result and not result.get('restoration_errors')
                            and result.get('stream_active_at_end') and result['state_restored']
                            and len(result['events']) == 2 and not result['callback_flags_after_warmup']
                            and 'measurement' in result and clean_measurement(result['measurement']))

def run(change, destination, playback='Main Output 1/2 (Audient EVO4)'):
    inp = endpoint('Loop-back 1/2 (Audient EVO4)', 'input')
    out = endpoint(playback, 'output')
    result = {'mode':'attenuate-and-restore' if change else 'read-only-baseline',
              'input':inp, 'output':out, 'events':[], 'callback_flags':[]}
    with Evo() as evo:
        original = evo.snapshot()
        result['before'] = original
        rate = original['sample_rate']
        if inp['default_samplerate'] != rate or out['default_samplerate'] != rate:
            raise RuntimeError('Endpoint/device rates differ; will not reconfigure audio')
        volumes = original['output_raw']
        if change and not all(-90*256 <= x <= 0 for x in volumes):
            raise RuntimeError('Output volume outside bounded experiment range')
        target = [x-256 for x in volumes]  # Exactly 1 dB lower.
        capacity = int(20*rate)
        captured = np.zeros((capacity,2),dtype='float32')
        n = np.arange(capacity,dtype=np.float64)
        tone = (0.0025*(np.sin(2*np.pi*997*n/rate)+np.sin(2*np.pi*1613*n/rate))).astype('float32')
        tone[:rate//10] *= np.linspace(0,1,rate//10,dtype='float32')
        counters = [0,0]
        callback_count = [0]
        def callback(indata,outdata,frames,clock,status):
            i,o = counters
            if max(i,o)+frames > capacity:
                outdata.fill(0)
                raise sd.CallbackAbort
            captured[i:i+frames] = indata
            outdata[:,0] = tone[o:o+frames]
            outdata[:,1] = tone[o:o+frames]
            callback_count[0] += 1
            counters[:] = [i+frames,o+frames]
            if status:
                result['callback_flags'].append({'frame':i,'flags':str(status)})
        touched = False
        settings = sd.WasapiSettings(exclusive=False, auto_convert=False)
        try:
            with sd.Stream(device=(inp['index'],out['index']),samplerate=rate,
                           channels=2,dtype='float32',blocksize=0,latency='high',
                           extra_settings=(settings,settings),callback=callback) as stream:
                start = time.perf_counter()
                changed = restored = False
                while time.perf_counter()-start < 12:
                    elapsed = time.perf_counter()-start
                    if not stream.active:
                        raise RuntimeError('Stream stopped unexpectedly')
                    if elapsed >= 4 and not changed:
                        check = measure(captured[rate:counters[0]-rate//4],rate)
                        if not clean_measurement(check):
                            raise RuntimeError('Baseline pilot is absent or discontinuous; no write performed')
                        if change:
                            touched = True
                            for channel,value in enumerate(target,1):
                                evo.control(10,2,channel,C.c_int16,value)
                        changed = True
                        result['events'].append({'event':'attenuate' if change else 'baseline-marker',
                                                 'seconds':elapsed,'capture_frame':counters[0],
                                                 'state':evo.snapshot()})
                        if change and result['events'][-1]['state']['output_raw'] != target:
                            raise RuntimeError('Attenuation readback mismatch')
                    if elapsed >= 8 and not restored:
                        if change:
                            for channel,value in enumerate(volumes,1):
                                evo.control(10,2,channel,C.c_int16,value)
                        restored = True
                        result['events'].append({'event':'restore' if change else 'baseline-marker',
                                                 'seconds':elapsed,'capture_frame':counters[0],
                                                 'state':evo.snapshot()})
                        if change and result['events'][-1]['state']['output_raw'] != volumes:
                            raise RuntimeError('Restoration readback mismatch')
                    # Bounded 5 Hz read load, representative of a controller.
                    evo.snapshot()
                    time.sleep(0.2)
                result['stream_active_at_end'] = bool(stream.active)
                result['latency'] = stream.latency
        except BaseException as error:
            result['error'] = str(error)
            raise
        finally:
            finish_capture(evo, original, touched, result, captured, rate, counters, callback_count)
            Path(destination).parent.mkdir(parents=True,exist_ok=True)
            Path(destination).write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
    if not result['passed']:
        raise RuntimeError('Continuity criteria failed; inspect saved evidence')

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--change',action='store_true')
    parser.add_argument('--output',required=True)
    parser.add_argument('--loopback-playback',action='store_true')
    args = parser.parse_args()
    run(args.change,args.output, 'Loop-back 1/2 (Audient EVO4)' if args.loopback_playback else 'Main Output 1/2 (Audient EVO4)')
