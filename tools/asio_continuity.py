"""GPL-3.0-only. Native ASIO continuity experiment; aggregate evidence only."""
import argparse
import ctypes as C
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys
import time
import winreg

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from probe import Evo
from continuity import measure, clean_measurement


def run(change, output, seconds=12.5):
    if not math.isfinite(seconds) or not 12.5 <= seconds <= 120:
        raise ValueError('Duration must be 12.5..120 seconds')
    capture_path = ROOT / 'evidence/asio-capture-temporary.bin'
    result = {'mode': 'attenuate-and-restore' if change else 'read-only-baseline',
              'timestamp': time.time(), 'events': [], 'host_events': [], 'state_reads': 0,
              'capture_stored': False, 'requested_seconds': seconds}
    output.parent.mkdir(parents=True, exist_ok=True)
    if change:
        baseline_path = ROOT / 'evidence/asio-baseline.json'
        if not baseline_path.exists() or not json.loads(baseline_path.read_text()).get('passed'):
            raise RuntimeError('A clean ASIO baseline is required before writing')
    touched = False
    process = None
    try:
        with winreg.OpenKey(winreg.HKEY_CLASSES_ROOT,
                            r'CLSID\{E7E92954-EA17-433a-9142-EE419BE4875C}\InprocServer32') as key:
            asio_path = Path(winreg.QueryValueEx(key, None)[0])
        asio_hash = hashlib.sha256(asio_path.read_bytes()).hexdigest()
        result['asio_dll'] = str(asio_path)
        result['asio_dll_sha256'] = asio_hash
        if asio_hash != 'aa893589ec86ba9ec225c98ca4a03ffa964088ee089ea3fd955ad7fb08177acd':
            raise RuntimeError('Untested ASIO driver DLL')
        with Evo() as evo:
            original = evo.snapshot()
            result['before'] = original
            volumes = original['output_raw']
            if change and not all(-89 * 256 <= value <= 0 for value in volumes):
                raise RuntimeError('Volume outside bounded experiment range')
            target = [value - 256 for value in volumes]
            try:
                process = subprocess.Popen([str(ROOT / 'tools/asio_host.exe'), str(capture_path),
                                            str(round(seconds * 1000))],
                                           stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                           text=True, cwd=ROOT)
                while True:
                    line = process.stdout.readline()
                    if not line:
                        raise RuntimeError('ASIO host failed before start: ' + process.stderr.read())
                    event = json.loads(line)
                    result['host_events'].append(event)
                    if event['event'] == 'started':
                        break
                started = time.perf_counter()
                changed = restored = False
                while process.poll() is None:
                    elapsed = time.perf_counter() - started
                    if elapsed > seconds + 6:
                        raise RuntimeError('ASIO host timeout')
                    if elapsed >= 4 and not changed:
                        if change:
                            touched = True
                            for channel, value in enumerate(target, 1):
                                evo.control(10, 2, channel, C.c_int16, value)
                        current = evo.snapshot()
                        result['events'].append({'event': 'attenuate' if change else 'baseline-marker',
                                                 'seconds': elapsed, 'state': current})
                        if change and current['output_raw'] != target:
                            raise RuntimeError('Attenuation readback mismatch')
                        changed = True
                    if elapsed >= 8 and not restored:
                        if change:
                            for channel, value in enumerate(volumes, 1):
                                evo.control(10, 2, channel, C.c_int16, value)
                        current = evo.snapshot()
                        result['events'].append({'event': 'restore' if change else 'baseline-marker',
                                                 'seconds': elapsed, 'state': current})
                        if change and current['output_raw'] != volumes:
                            raise RuntimeError('Restoration readback mismatch')
                        restored = True
                    evo.snapshot()
                    result['state_reads'] += 1
                    time.sleep(.2)
                stdout, stderr = process.communicate(timeout=2)
                result['host_events'].extend(json.loads(line) for line in stdout.splitlines())
                result['host_exit_code'] = process.returncode
                result['host_stderr'] = stderr
                if process.returncode:
                    raise RuntimeError('ASIO host failed: ' + stderr)
            finally:
                if touched:
                    failures = []
                    for channel, value in enumerate(volumes, 1):
                        try:
                            evo.control(10, 2, channel, C.c_int16, value)
                        except Exception as error:
                            failures.append(str(error))
                    if failures:
                        result['restoration_errors'] = failures
                result['after'] = evo.snapshot()
                result['state_restored'] = result['after'] == original
        if capture_path.exists():
            audio = np.fromfile(capture_path, dtype=np.float32).reshape(-1, 2)
            rate = original['sample_rate']
            result['measurement'] = measure(audio[rate:-rate // 4], rate)
            result['capture_frames'] = len(audio)
        completed = [event for event in result['host_events'] if event['event'] == 'complete']
        result['passed'] = bool(len(completed) == 1 and result['state_restored']
                                and not result.get('restoration_errors')
                                and len(result['events']) == 2
                                and result['state_reads'] >= max(40, int(seconds * 4))
                                and all(completed[0][key] == 0 for key in
                                        ['resets', 'resyncs', 'overloads', 'rate_changes', 'position_gaps'])
                                and completed[0]['position_available']
                                and clean_measurement(result['measurement']))
    except BaseException as error:
        result['error'] = str(error)
        result['passed'] = False
    finally:
        if process is not None and process.poll() is None:
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                process.terminate()
                process.wait(timeout=3)
        capture_path.unlink(missing_ok=True)
        output.write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))
    return result['passed']


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--change', action='store_true')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--seconds', type=float, default=12.5)
    arguments = parser.parse_args()
    sys.exit(0 if run(arguments.change, arguments.output, arguments.seconds) else 1)
