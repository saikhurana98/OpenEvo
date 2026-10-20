"""Measure four playback-source columns using the production Rust mixer setter.

Requires muted physical outputs and the known OEM saved loopback configuration.
Does not touch microphone routes, preamp gain, 48V, rate or buffer size.
Temporary capture is deleted; aggregate spectral amplitudes are retained.
"""
import ctypes as C
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time
import winreg
import xml.etree.ElementTree as ET
import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from probe import Evo

CAPTURE = ROOT / 'evidence/matrix-capture-temporary.bin'
OUTPUT = ROOT / 'evidence/native-matrix-validation.json'
FREQUENCIES = [997, 1213, 1613, 2111]
AMPLITUDE = .01
report = {'kind': 'Live ASIO playback-source matrix measurement using production Rust setter',
          'pilot_amplitude': AMPLITUDE, 'frequencies_hz':FREQUENCIES,
          'events': [], 'restoration_errors': [], 'microphone_routes_modified': False,
          'hardware_matrix_readback_available': False, 'passed': False}
parser = argparse.ArgumentParser()
parser.add_argument('--sources', type=int, nargs='+', choices=[2, 3, 4, 5], default=[2, 3, 4, 5])
parser.add_argument('--output', type=Path, default=OUTPUT)
arguments = parser.parse_args()
OUTPUT = arguments.output


def apply(source, gain, pan, muted):
    completed = subprocess.run([str(ROOT/'target/release/examples/matrix_probe.exe'),
                                str(source), str(gain), str(pan), str(int(muted))],
                               text=True, capture_output=True, check=True, timeout=15)
    return json.loads(completed.stdout)


def capture():
    try:
        completed = subprocess.run([str(ROOT/'tools/asio_matrix_host.exe'), str(CAPTURE)],
                                   text=True, capture_output=True, check=True, timeout=15)
        host = json.loads(completed.stdout)
        if any(host[key] for key in ['resets', 'resyncs', 'overloads', 'rate_changes']):
            raise RuntimeError(f'ASIO event during measurement: {host}')
        audio = np.fromfile(CAPTURE, dtype=np.float32).reshape(-1, 4)[96000:-9600]
        if len(audio) < 96000:
            raise RuntimeError('Insufficient stable capture')
        positions = np.arange(len(audio)) / 96000
        basis = np.column_stack([fn(2*np.pi*hz*positions) for hz in FREQUENCIES
                                 for fn in (np.sin, np.cos)] + [np.ones(len(audio))])
        coefficients = np.linalg.lstsq(basis, audio[:, 2:], rcond=None)[0]
        amplitudes = np.hypot(coefficients[0:8:2], coefficients[1:8:2]).T
        window_amplitudes = []
        width = 9600
        for offset in range(0, len(audio)-width+1, width):
            local = np.linalg.lstsq(basis[offset:offset+width], audio[offset:offset+width, 2:], rcond=None)[0]
            window_amplitudes.append(np.hypot(local[0:8:2], local[1:8:2]).T.tolist())
        residual = audio[:, 2:] - basis @ coefficients
        return {'host': host, 'amplitudes': amplitudes.tolist(),
                'window_ms':100, 'window_amplitudes':window_amplitudes,
                'residual_rms':np.sqrt(np.mean(residual**2, axis=0)).tolist(),
                'physical_input_rms': np.sqrt(np.mean(audio[:, :2]**2, axis=0)).tolist()}
    finally:
        CAPTURE.unlink(missing_ok=True)


def matches(measurement, source, left_db, right_db):
    column = source - 2
    actual = np.asarray(measurement['amplitudes'])[:, column]
    expected = np.array([0 if db <= -120 else AMPLITUDE*10**(db/20)
                         for db in [left_db, right_db]])
    if not np.all(np.abs(actual-expected) <= np.maximum(expected*.03, .000003)):
        raise RuntimeError(f'Source {source}: actual {actual.tolist()} expected {expected.tolist()}')


touched = set()
# Restore the OEM's saved intended route, verified against the untouched baseline.
originals = {2: (0, -100, True), 3: (0, 100, True),
             4: (0, -100, False), 5: (0, 100, False)}
try:
    with winreg.OpenKey(winreg.HKEY_CLASSES_ROOT,
                        r'CLSID\{E7E92954-EA17-433a-9142-EE419BE4875C}\InprocServer32') as key:
        asio_path = Path(winreg.QueryValueEx(key, None)[0])
    if hashlib.sha256(asio_path.read_bytes()).hexdigest() != 'aa893589ec86ba9ec225c98ca4a03ffa964088ee089ea3fd955ad7fb08177acd':
        raise RuntimeError('ASIO DLL changed')
    with Evo() as evo:
        before = evo.snapshot()
        if before['sample_rate'] != 96000 or evo.control(59, 1, 0, C.c_uint32) != 1:
            raise RuntimeError('Requires current96kHz and already-muted physical output')
    preset = ET.parse(Path.home()/'AppData/Roaming/Audient/EVO/state.xml')
    saved = {int(item.attrib['channel']): item.find('mix').attrib
             for item in preset.findall('.//mixer/input')}
    if any(float(saved[i]['volume']) > -120 for i in [2, 3]):
        raise RuntimeError('Saved main-playback routes differ from the test configuration')
    if any(float(saved[i]['volume']) != 0 or float(saved[i]['pan']) != (i-4) for i in [4, 5]):
        raise RuntimeError('Saved loopback-playback routes differ from test configuration')
    baseline = capture()
    for source in [2, 3]: matches(baseline, source, -128, -128)
    matches(baseline, 4, 0, -128)
    matches(baseline, 5, -128, 0)
    report['baseline'] = baseline
    report['restoration_provenance'] = 'OEM saved intent plus live baseline; write-only matrix cannot be read back'
    for source in arguments.sources:
        hard_pan = -100 if source % 2 == 0 else 100
        for label, gain, pan, mute, expected in [
            ('hard-pan-attenuated', -6, hard_pan, False, (-6, -128) if hard_pan < 0 else (-128, -6)),
            ('center-pan', -6, 0, False, (-12, -12)),
            ('muted', -6, 0, True, (-128, -128)),
        ]:
            touched.add(source) # Includes uncertain/partial requests.
            acknowledgement = apply(source, gain, pan, mute)
            measurement = capture()
            event = {'source': source, 'case': label, 'write': acknowledgement,
                     'measurement': measurement, 'expected_destination_db': expected}
            report['events'].append(event)
            matches(measurement, source, *expected)
            event['passed'] = True
        apply(source, *originals[source])
    restored = capture()
    for source in [2, 3]: matches(restored, source, -128, -128)
    matches(restored, 4, 0, -128)
    matches(restored, 5, -128, 0)
    report['restored_measurement'] = restored
    report['passed'] = True
except BaseException as error:
    report['error'] = str(error)
finally:
    for source in sorted(touched):
        try:
            apply(source, *originals[source])
        except Exception as error:
            report['restoration_errors'].append(f'Source {source}: {error}')
    if report['restoration_errors']:
        report['passed'] = False
    CAPTURE.unlink(missing_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'passed': report['passed'], 'completed_cases':len(report['events']),
                      'error': report.get('error'), 'restoration_errors':report['restoration_errors']}))
sys.exit(0 if report['passed'] else 1)
