"""Bounded Windows EVO4 feasibility probe; defaults to read-only.

ABI recovered from installed Audient API 5.70.0.0 with REA/Ghidra.
This is an investigative harness, not the eventual Rust application.
"""
import argparse
import ctypes as C
import hashlib
import json
import time
import subprocess
from pathlib import Path

DLL = Path(r'C:\Program Files\Audient\USBAudioDriver\x64\audientusbaudioapi_x64.dll')
DLL_SHA256 = '25af306d32bb674796bfe82ba220396254d97e8859e024394fbeda7db242b2e6'
U32, U8 = C.c_uint32, C.c_uint8

class Evo:
    def __init__(self):
        if hashlib.sha256(DLL.read_bytes()).hexdigest() != DLL_SHA256:
            raise RuntimeError('Untested driver DLL: revalidate ABI before use')
        self.dll = C.WinDLL(str(DLL))
        self.handle = U32()
        self.api = self.bind('GetApiVersion', [])
        # The export ignores RCX/RDX; R8 high bytes flow into enumeration flags.
        # Passing three zero arguments makes those unused/flag bytes deterministic.
        self.enumerate = self.bind('EnumerateDevices', [U32, U32, U32])
        self.count = self.bind('GetDeviceCount', [])
        self.open = self.bind('OpenDeviceByIndex', [U32, C.POINTER(U32)])
        self.close = self.bind('CloseDevice', [U32])
        self.rate = self.bind('GetCurrentSampleRate', [U32, C.POINTER(U32)])
        args = [U32, U8, U8, U8, U8, C.c_void_p, U32, C.POINTER(U32), U32]
        self.get = self.bind('AudioControlRequestGet', args)
        self.set = self.bind('AudioControlRequestSet', args)

    def bind(self, name, args):
        f = getattr(self.dll, 'TUSBAUDIO_' + name)
        f.argtypes, f.restype = args, U32
        return f

    @staticmethod
    def check(status):
        if status:
            raise RuntimeError(f'Audient API error 0x{status:08x}')

    def __enter__(self):
        # Match this narrowly validated probe to EVO4, rather than any device
        # that happens to be installed under the same vendor driver.
        command = "Get-PnpDevice -PresentOnly | Where-Object { $_.Class -eq 'audientusbaudio_sc' } | ForEach-Object { $_.InstanceId }"
        found = subprocess.run(['powershell.exe','-NoProfile','-NonInteractive','-Command',command],
                               check=True,capture_output=True,text=True,timeout=15)
        identities = [s.strip().upper() for s in found.stdout.splitlines() if s.strip()]
        if len(identities) != 1 or not identities[0].startswith('USB\\VID_2708&PID_0006\\'):
            raise RuntimeError('Exactly one present EVO4 is required')
        self.check(self.enumerate(0, 0, 0))
        if self.count() != 1:
            raise RuntimeError('Probe requires exactly one Audient device')
        self.check(self.open(0, C.byref(self.handle)))
        return self

    def __exit__(self, *args):
        if self.handle.value:
            self.check(self.close(self.handle))
            self.handle.value = 0

    def control(self, unit, selector, channel, data_type, value=None):
        data, transferred = data_type(), U32()
        if value is not None:
            # Only output attenuation is writable in this harness.
            if (unit, selector, channel) not in [(10, 2, 1), (10, 2, 2)]:
                raise ValueError('Write is outside the probe allowlist')
            data.value = value
        operation = self.get if value is None else self.set
        self.check(operation(self.handle, unit, 1, selector, channel,
                             C.byref(data), C.sizeof(data), C.byref(transferred), 1000))
        if value is None and transferred.value != C.sizeof(data):
            raise RuntimeError(f'Short read: {transferred.value}')
        return data.value

    def snapshot(self):
        rate = U32()
        self.check(self.rate(self.handle, C.byref(rate)))
        return {
            'sample_rate': rate.value,
            'output_raw': [self.control(10, 2, c, C.c_int16) for c in (1, 2)],
            'input_gain_raw': [self.control(11, 2, c, C.c_int16) for c in (1, 2)],
            'monitor_mix_raw': self.control(56, 0, 0, C.c_uint16),
        }

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', default='evidence/read-state.json')
    args = parser.parse_args()
    with Evo() as evo:
        result = {'timestamp': time.time(), 'dll_sha256': DLL_SHA256,
                  'api_version': hex(evo.api()), 'state': evo.snapshot()}
    path = Path(args.output)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))

if __name__ == '__main__':
    main()
