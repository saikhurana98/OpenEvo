# Historical development notes

Raw evidence files referenced here are retained locally and are not part of the public repository.

# EVO Control

The native Rust + Slint app is available as version 0.3.0 (responsive Orbit) for Windows x64. See
[Native application instructions](../NATIVE_README.md) and the current
[release acceptance audit](../RELEASE_STATUS.md). Live control edits, ASIO
coexistence, all six mixer sources, physical input meters, Smartgain calibration,
physical knob sync and USB disconnect/reconnect passed on the inspected EVO4 in
the first release. Orbit now reflows without scrolling for portrait-display
snaps, with compact disclosure panels for secondary controls. Main playback
meters animate the master ring, using fast attack and smooth release. The
driver's existing six-source meter packet supplies both input and playback
levels; the control-write paths remain unchanged.

## Historical Windows feasibility probe

The first feasibility gate passed on the installed driver: read device state,
lower output volume by 1 dB, confirm readback, and restore it while a measured
96 kHz stereo WASAPI hardware-loopback stream continued without detected gaps.
The repository now contains the native app and the separate investigative
Python/C++ harnesses used to verify it.

## Reproduce

Run from this directory in PowerShell with 64-bit Python 3.12:

```powershell
python -m venv .venv
.venv/Scripts/python.exe -m pip install -r requirements.txt
.venv/Scripts/python.exe probe.py
.venv/Scripts/python.exe -m unittest -v
.venv/Scripts/python.exe continuity.py --loopback-playback --output evidence/new-baseline.json
.venv/Scripts/python.exe continuity.py --loopback-playback --change --output evidence/new-write.json
```

`probe.py` only reads. `continuity.py` plays a quiet two-tone pilot and captures
the EVO's loopback input. Only `--change` writes: it temporarily lowers both
output channels by 1 dB, then restores the original raw values. It does not set
phantom power, input gain, routing, sample rate, or buffer size. No mixer route
is created automatically. The current route must carry the pilot cleanly or
the experiment stops before any write. Raw captured audio is not saved.

The harness requires exactly one present EVO4 and pins the installed DLL hash.
It loads that DLL from its absolute installed path, leaving its adjacent
`custom.ini` in place. No driver replacement, registration change, or elevated
permissions were needed. The venv is local to this project.

Restoration runs on normal completion and Python exceptions. A forced process
termination, power failure, or physical unplug can prevent restoration; the
saved `before.output_raw` values document the starting state. Do not operate
the hardware knob or another controller during a repeat, since this short
experiment restores its starting values rather than arbitrating user changes.

## Result and limits

See [FEASIBILITY.md](../FEASIBILITY.md) for the ABI, evidence, measurements, and
original measurement limits. Later ASIO evidence is described in
[tools/asio-validation.md](../tools/asio-validation.md).
`evidence/continuity-write-validated.json`
is the final run with explicit pass/fail criteria. Earlier experiments are
retained, including the Main Output route that did not carry the pilot.

The production app uses Rust + Slint. Python was chosen for the probe
to exercise the verified DLL ABI with a small auditable harness and the already
installed interpreter; this is not a change to the product stack.
