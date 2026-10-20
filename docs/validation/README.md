# UI validation

These reports contain simulated control values only. The native interactive test
loads no hardware driver. Each control is edited twice: the first edit simulates
a rejected write and rolls back; the second supplies a different authoritative
value. The test checks the actual UI model in both directions.

- dense-ui-readback.json: 18 cases in the compact full Orbit console.
- responsive-ui-readback.json: 18 cases using compact inspectors.
- orbit-ui-readback.json: 34 cases across all 17 continuous controls.

The public OpenEvo rename changes labels and packaging only. Twenty-four regular
Rust tests pass on the release build. See RELEASE_STATUS.md for the boundaries
of historical hardware and audio-continuity measurements.
