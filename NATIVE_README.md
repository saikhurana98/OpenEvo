# OpenEvo

A native Rust + Slint Windows control window for Audient EVO4, with an
EVO-inspired charcoal/green interface. Uses the installed Audient audio driver.

**Version 0.3.2: motion polish.** Buttons have eased hover/press
feedback; inspectors slide/fade in and out; console geometry moves smoothly as
space changes. Motion is short (60–160 ms) and respects Windows' client-area
animation preference read at startup. Closing or covered controls cannot accept
edits. Actual control values and hardware commands stay immediate.

The existing layout is preserved from 0.3.1: adaptive, dense Orbit. The original full console stays visible
when its controls fit. Output, monitor mix/pan, two inputs, Smartgain/stereo link
and all six loopback fader/pan/mute strips fit both 1080 x 1217 and 1080 x 810
physical-pixel windows at 150% scale, without scrolling. Dials, cards and spacing
shrink with the window. Device settings remain an explicit Settings inspector.

At shorter sizes the loopback mixer collapses first. Narrow, short windows use
compact disclosure panels; expanding the window restores the full console
automatically. A narrow, tall window can show the mixer in two rows of three.
The compact source selector loads saved intent without applying routing.
Meter ballistics and smooth numeric/knob transitions from 0.3.0 are retained.
Twenty-four Rust tests and eighteen fresh UI-only state-sync cases pass.

The existing hardware control writes are unchanged from 0.1.0, whose live checks
covered input/output controls, driver settings, ASIO coexistence, all six mixer
sources, Smartgain, physical knob sync and USB reconnect. See RELEASE_STATUS.md
for the distinction between the Orbit checks and that earlier hardware evidence.

## Run

Unzip `OpenEvo-0.3.2-windows-x64.zip` and run `OpenEvo.exe`. The source-workspace
executable is `target/release/openevo.exe`. No Python, webview, server, account, or replacement audio
driver is needed to run the native app. Connect exactly one EVO4.

This build supports the inspected Windows x64 API DLL only:
`Audient/USBAudioDriver/x64/audientusbaudioapi_x64.dll`, file version5.70.0.0,
SHA-256 `25af306d32bb674796bfe82ba220396254d97e8859e024394fbeda7db242b2e6`.
An unrecognized DLL or another Audient model produces a disconnected state.
The DLL is loaded from its installed location and is not redistributed.

## Controls and behavior

- Inputs: gain, mute, two-step48V activation, stereo link, Smartgain, meters.
- Monitor: output volume/mute, direct-monitor blend and input pan.
- Loopback: six mono sources (Mic1, Mic2, MainL, MainR, LoopbackL, LoopbackR),
  each with level, OEM-derived pan law and mute. The hardware matrix is write-only.
  The editor labels saved intent rather than hardware readback. Editing a source applies that source's complete
  route; cached values are never automatically replayed after launch/reconnect.
  Routing schema2 discards the earlier prototype's four-source cache rather
  than reinterpret those values using different coefficients.
- Inline device settings: sample rate and ASIO buffer choices queried from the
  driver. Some capabilities may be unavailable when not exposed by the driver.
- Firmware updates: use the official Audient EVO application/updater. Firmware
  flashing is outside this release.

All control calls are serialized on a worker. Both command queues are bounded;
continuous changes coalesce. Connection generations reject stale requests after
reconnect. Startup only reads, and failed writes are followed by state readback.
The GUI does not stream audio to obtain meters: it uses the OEM meter request.
Output-level changes preserve both original channel levels first. Any write or
readback failure triggers restoration attempts for both channels and verification
of both originals; unconfirmed restoration remains an error.

Knobs and horizontal rails support dragging, arrow keys and Home/End.
Drag knobs up/right to increase and down/left to decrease; clicking a knob does
not jump its value. Tab moves focus between controls. Input rings show gain
position. The master ring and stereo bars show measured main-playback source
activity; its white dot still shows the volume setting. The playback meter tap
is not established as a post-volume or analog-output measurement.

Meters have a 12 ms attack, 65 ms hold and 220 ms exponential release, presented
on a 16 ms UI timer. These are display ballistics only and do not process audio.
LED segments fade fractionally, and setpoint geometry/readouts transition over
80/90 ms. Numeric readouts use fixed-width digits. Editing still immediately
updates the actual command value; animations never send intermediate commands.

The main UI has no scroll view. Device settings use compact dropdowns.
Rate/buffer changes can restart audio.
The app does not promise click-free physical output: current continuity evidence
measures the digital loopback path.

## Build

Requires Rust and MSVC x64 build tools on Windows. The development workspace
also supports its local Rust installation under `.tools/`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build.ps1 -Configuration release
powershell -NoProfile -ExecutionPolicy Bypass -File tools/build.ps1 -Configuration release -Test
./target/release/openevo.exe --diagnostics evidence/native-state.json
```

When building the portable ZIP's included source, run these commands from its
`source/` directory. Cargo.lock pins dependencies. The build script selects local Rust when
available, otherwise Cargo on PATH. In the development workspace,
`tools/package.ps1` builds a portable
ZIP containing the executable, application source and notices.

The earlier Python harness and native ASIO test host remain available in the
source workspace. See FEASIBILITY.md and tools/asio-validation.md. Restoration
in the bounded write experiments is attempted on normal completion and Python
exceptions; forced termination or physical disconnect can prevent restoration.

To validate the new motion without a physical window or device, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/test-motion.ps1
```

The test uses an in-memory software renderer, simulated input and a mocked
clock. It saves PNG frames and a report under evidence/motion/. Native mouse/
keyboard interaction and a live device were not re-tested for this release.

## Evidence

- FEASIBILITY.md: original successful WASAPI feasibility experiment.
- PROTOCOL.md: OEM-derived API/control contracts and remaining verification.
- RELEASE_STATUS.md: current acceptance audit.
- EXECUTION.md: full first-release checklist.
- evidence/: raw REA records, analysis snapshots and measured runtime results.

Raw investigation files remain in the development workspace. The portable ZIP
includes the acceptance summary, application source and build instructions.

Existing installations retain their saved loopback intent in the legacy
`%LOCALAPPDATA%/EvoControl/loopback.json` location. The rename does not replay
those routes automatically.

GPL-3.0-only. Independent software; see LICENSE and THIRD_PARTY_NOTICES.md.
