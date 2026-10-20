# First usable Windows release

Target: a modern, light, single-window open-source replacement for the daily
EVO4 OEM controls and loopback mixer. Keep the installed Audient audio driver.
Initial platform: Windows x64, tested installed DLL, EVO4 only.

## Release acceptance checklist

- [x] Native executable and portable ZIP with source/build instructions.
- [x] One window: two inputs, output/monitor controls, Smartgain, loopback mixer,
      inline driver settings and diagnostics; EVO-inspired charcoal/green palette.
- [x] Real device detection and readable state on startup, with no startup writes.
- [x] Input gain, input mute, 48V, linking, output level/mute, monitoring controls.
- [x] Smartgain request sequence/status and input meters verified from OEM evidence.
- [x] Six-source loopback level/pan/mute routing with honest write-only state handling.
- [x] Sample rate and ASIO buffer settings verified and shown with supported values.
- [x] Worker isolates driver calls from rendering; command queue is bounded and
      continuous edits coalesce without losing ordered discrete actions.
- [x] Disconnect/error/reconnect state, no automatic replay of stale writes.
- [x] Physical knob/readable changes reconcile; failed writes remain errors.
- [x] Windows transport enumeration contract corroborated against OEM call site.
- [x] WASAPI digital-loopback stream survived reversible output write and reads.
- [x] ASIO stream survived concurrent control reads and reversible write.
- [x] Hardware-backed UI interaction, scaling/keyboard checks, packaged launch.
- [x] Local tests cover protocol packing, restore/error handling and relevant
      state behavior; build/release checks pass.
- [x] License, attribution, driver dependency and known limitations documented.

Firmware flashing is a documented OEM handoff for this release, as planned.
Analog-output click measurement needs an external recording path; report that
boundary explicitly rather than treating digital-loopback evidence as analog proof.

## Release 0.1.0 evidence

- EVO4 is connected. Native diagnostics read the expected 96 kHz / 1024-frame
  configuration; optional capabilities respond. Reversible native UI edits and
  independent driver readbacks verify the input/output and driver controls.
- Three 12.5-second ASIO runs passed: baseline, reversible output write, and the
  write repeated with native UI polling active. No reported reset, resync,
  overload, rate change, sample-position gap or missing pilot window.
- A further 60-second ASIO read run passed 300 state reads with the native UI
  open. No detected pilot dropout or ASIO timing event. This is not a long soak.
- All six sources passed eighteen measured matrix cases using the production
  Rust setter. Physical input routes were compared against simultaneous ASIO
  inputs; the actual measured baseline was restored.
- Smartgain completed with a looping DJ signal, selected7.5/8.5dB and restored
  original1.5/1dB afterward. Live input meters respond. Both48V toggles were
  checked with empty inputs and restored off; no48V writes with the console.
- The native Smartgain selection now preserves other selector4 bits, following
  the OEM; four new fault/protocol tests bring Rust's passing total to21.
- Green native sliders passed all 34 UI-only synchronization checks; connected
  150% scaling and 801×600 scrolling were inspected. The physical output knob
  changed-80 to-73dB and the native window matched independent driver readback.
  The same portable process/window survived physical USB unplug/replug; controls
  disabled and then recovered with fresh driver state. Connecting remains
  read-only by source inspection; no app edits occurred during the cycle.
- Connected idle sample: 29.25 MiB working set, 9.48 MiB private memory and about
  1.4% of one CPU core over ten seconds. This is a short sample, not a soak test.
- Updated portable build with live input meters:29.81MiB working set and4.36%
  of one core in a ten-second sample. Source/manifest/connected launch verified.

All applicable first-release gates passed on the inspected device and driver.
The release package includes source, notices and build instructions; the archive
manifest and connected packaged launch are checked before handoff. This does not
extend the claim to analog click measurement, hours-long sessions or other DLLs.
