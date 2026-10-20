"""Verify the portable archive against its manifest and current app source."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('archive', type=Path)
parser.add_argument('--state-before', type=Path, default=ROOT/'evidence/native-session-restored.json')
parser.add_argument('--state-after', type=Path, default=ROOT/'evidence/packaged-connected-state.json')
parser.add_argument('--ui-evidence', type=Path, default=ROOT/'evidence/packaged-native-ui-smoke.json')
parser.add_argument('--report', type=Path, default=ROOT/'evidence/package-verification.json')
args = parser.parse_args()
archive = args.archive.resolve()
digest = lambda data: hashlib.sha256(data).hexdigest()

with zipfile.ZipFile(archive) as package:
    names = [name for name in package.namelist() if not name.endswith('/')]
    roots = {name.split('/')[0] for name in names}
    assert len(roots) == 1, 'Unexpected archive layout'
    prefix = roots.pop() + '/'
    manifest_name = prefix + 'SHA256SUMS.txt'
    manifest = {}
    for line in package.read(manifest_name).decode('utf-8-sig').splitlines():
        expected, relative = line.split('  ', 1)
        assert relative not in manifest, 'Duplicate manifest entry'
        manifest[relative] = expected
    assert set(names) == {prefix + name for name in manifest} | {manifest_name}, 'Unlisted or missing file'
    for relative, expected in manifest.items():
        data = package.read(prefix + relative)
        assert digest(data) == expected, f'Manifest mismatch: {relative}'
        if relative.startswith('source/'):
            local = ROOT / relative.removeprefix('source/')
            assert data == local.read_bytes(), f'Stale source: {relative}'
        elif relative.startswith('docs/') or relative.endswith('.md') or relative == 'LICENSE':
            assert data == (ROOT / relative).read_bytes(), f'Stale document: {relative}'
    assert not any(name.lower().endswith('.dll') for name in names), 'Unexpected bundled DLL'
    exe = package.read(prefix + 'OpenEvo.exe')
    assert exe == (ROOT / 'target/release/openevo.exe').read_bytes(), 'Stale executable'
    assert 'source/.cargo/config.toml' in manifest, 'Missing build configuration'

before = json.loads(args.state_before.read_text())
packaged = json.loads(args.state_after.read_text())
state_keys = ['sample_rate', 'buffer_size', 'input_gain_db', 'input_muted',
              'phantom', 'linked', 'monitor_blend', 'monitor_pan', 'output_db', 'output_muted']
assert before['connected'] and packaged['connected'], 'Connected CLI smoke did not pass'
assert all(before[key] == packaged[key] for key in state_keys), 'Packaged startup state changed'
ui = json.loads(args.ui_evidence.read_text())
assert ui['connected_visible'] and ui['hardware_actions'] == 0, 'Missing UI smoke'
report = {'archive':str(archive.relative_to(ROOT)), 'sha256':digest(archive.read_bytes()),
          'verified_manifest_files':len(manifest), 'source_matches_workspace':True,
          'contains_vendor_dll':False, 'executable_sha256':digest(exe),
          'executable_bytes':len(exe), 'connected_cli_smoke':True,
          'connected_ui_smoke':True, 'startup_readable_state_unchanged':True,
          'state_before_evidence':str(args.state_before),'state_after_evidence':str(args.state_after),
          'ui_evidence':str(args.ui_evidence),
          'passed':True}
args.report.write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report, indent=2))
