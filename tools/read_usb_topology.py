"""Read-only descriptor capture for the exact REA-verified Audient API DLL."""
import ctypes as C
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from probe import Evo, U32

with Evo() as evo:
    get_config = evo.bind('GetUsbConfigDescriptor', [U32, C.c_void_p, U32, C.POINTER(U32)])
    buffer = C.create_string_buffer(65535)
    returned = U32()
    evo.check(get_config(evo.handle, buffer, len(buffer), C.byref(returned)))
    if not 9 <= returned.value <= len(buffer):
        raise RuntimeError(f'Invalid descriptor byte count: {returned.value}')
    raw = bytes(buffer[:returned.value])
    total = int.from_bytes(raw[2:4], 'little')
    if raw[0] < 9 or raw[1] != 2 or not raw[0] <= total <= len(raw):
        raise RuntimeError('Invalid configuration descriptor header')
    get_properties = evo.bind('GetDeviceProperties', [U32, C.c_void_p])
    properties = C.create_string_buffer(1040)
    evo.check(get_properties(evo.handle, properties))

items = []
offset = 0
while offset < total:
    length = raw[offset]
    if length < 2 or offset + length > total:
        raise RuntimeError(f'Invalid USB descriptor at {offset}')
    data = raw[offset:offset+length]
    items.append({'offset': offset, 'length': length, 'type': data[1], 'hex': data.hex()})
    offset += length

(ROOT / 'evidence/usb-config-live.bin').write_bytes(raw)
(ROOT / 'evidence/usb-device-properties-live.bin').write_bytes(properties.raw)
report = {'kind': 'Live read-only driver descriptor query; no control writes',
          'returned_bytes': len(raw), 'configuration_total_bytes': total,
          'sha256': hashlib.sha256(raw).hexdigest(), 'descriptors': items,
          'properties_bytes': len(properties.raw),
          'properties_sha256': hashlib.sha256(properties.raw).hexdigest(),
          'properties_semantics': 'Opaque; firmware fields not inferred'}
(ROOT / 'evidence/usb-topology-live.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report, indent=2))
