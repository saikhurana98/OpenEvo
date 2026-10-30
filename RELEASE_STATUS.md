# OpenEvo 0.3.2 release

Decorative feedback uses 60/100 ms easing, layout/entrance transitions 160 ms,
and inspector exit 70 ms. Panel switching preserves outgoing content until
its fade completes, always resolves the newest requested page, and disables
covered/exiting controls. Windows client-area animation preference is read at
startup; reduced motion skips decoration and setpoint interpolation. Audio
meter ballistics and all driver/route/worker code remain unchanged.

Motion validation uses Slint's in-memory software renderer and a mocked clock.
No desktop window, physical input or device API is used. The headless regression
checks all 17 continuous controls (34 rollback/readback assertions), actual
interpolation, stable button hitboxes, rapid switch/close/reopen, overlay input
blocking, reduced motion and five logical window sizes. Its frames and report
are in evidence/motion/. The ordinary Rust suite is also run separately.

The release passes 24 ordinary Rust tests and the offscreen motion regression.
Its source and internal checksum manifest are checked against the packaged
executable. Native visual review and live hardware tests were not repeated for
this release; the earlier hardware/audio measurements below remain historical.

# OpenEvo 0.3.1 public release

The accepted Orbit build is published as OpenEvo. Branding, executable/package
names, build documentation and repository metadata changed; device control and
routing code did not. The legacy local cache location is retained for existing
users. Driver/firmware binaries, local recordings and raw investigation dumps
are not redistributed. Raw evidence paths below refer to the development audit;
reviewed UI-only evidence and app screenshots are under docs/.

# Version 0.3.1 adaptive dense Orbit acceptance status

The original console organization is restored: output/monitoring on the left,
two inputs and Smartgain/stereo link on the right, and six fader/pan/mute strips
below. Controls compress before sections collapse. At 150% scale, the complete
console was visually inspected at 1080 x 1217 and 1080 x 810 physical pixels,
without scrolling. Evidence: dense-half-portrait.jpg and dense-third-portrait.jpg.

A wide window keeps the full primary/Smartgain/monitoring area; the mixer only
collapses below the space needed for six useful strips. Narrow, short windows
use the existing compact inspectors. Narrow, tall windows reflow the strips to
two rows of three. Expanding restores visible controls automatically. Settings
remains user-opened, matching the original console's semantics.

Twenty-four automated tests pass. Eighteen fresh simulated native UI cases at
the third-height size verify failed-write rollback and external authoritative
updates for both input gains, output, monitor mix/pan, and first/last loopback
levels/pans. Evidence: evidence/dense-ui-readback.json. The simulated test never
opens the hardware. Production driver, controller, routing, meter ballistics and
command handling are unchanged from 0.3.0. Prior hardware/audio measurements
below remain historical and were not repeated for this layout revision.

# Version 0.3.0 responsive Orbit acceptance status

The main canvas reflows instead of preserving the old 960 logical-pixel minimum
content width. Output, stereo playback activity, blend and input gain/mute/meters
are primary. Smartgain/link, monitor pan, loopback and driver settings are
disclosure panels. Only one detail panel opens at once; the output control stays
visible. The UI contains no scroll view.

## Responsive and animation checks

- The user-snap observation was 1080 x 1217 physical pixels at 150% application
  scale. Both that size and 1080 x 810 show all primary controls without scrolling.
  Disclosure panels and the first/last loopback sources were inspected.
- The driver returns all six meter samples in the existing 12-byte read. Live
  main-playback values were nonzero with zero input values. The master ring now
  visualizes playback activity; the white position dot retains the volume value.
  Meter labels describe playback sources, not a measured post-volume output.
- Presentation ballistics use 12 ms attack, 65 ms hold, 220 ms release, and a
  16 ms UI timer. Tests cover transient response, monotonic decay, frame-rate
  independence, invalid samples and immediate reset. Numeric readouts animate
  for 90 ms and use fixed-width digits; knob/rail geometry eases for 80 ms.
  Actual control values and commands are not interpolated.
- 24 Rust release tests pass. The GUI invariant test additionally checks all
  six source selections and authoritative source readback without route writes.
- Eighteen fresh native UI-only cases cover failed-write rollback and external
  authoritative updates on output, blend, both gains, pan and first/last source
  gain/pan. This separate simulated test invokes no hardware API.
  Evidence: evidence/responsive-ui-readback.json.
- Fresh startup/package results and inspected sizes are recorded in
  evidence/responsive-package-verification.json and
  evidence/responsive-layout-qa.json. All live inspection actions were navigation
  or window sizing. No device controls were edited.

Earlier audio continuity and physical reconnect measurements below are historical
and were not repeated for this display update. Hardware write paths are unchanged;
meter reads use the same packet, and smoothing is entirely visual.

# Historical version 0.2.0 Orbit acceptance status

Orbit implements the selected third image concept as a native Slint window:
a large output encoder on the left, stacked inputs and Smartgain on the right,
and six loopback fader/pan strips below. Charcoal surfaces, soft curves and
discrete lime LED dots follow the hardware-inspired design direction.

## Orbit checks (2026-10-20)

- The release build passes 21 Rust tests. The separately run interactive native
  UI test passes all 34 cases across the 17 continuous controls, covering
  simulated failed-write rollback and authoritative external changes. Evidence:
  evidence/orbit-ui-readback.json. No hardware API is called by that test.
- Pointer interaction confirms that clicking the master encoder leaves its value
  unchanged, relative dragging changes it, and keyboard Right increments it.
  A vertical loopback fader was also dragged. All three control forms expose
  accessible values and retain keyboard editing.
- The rendered UI was inspected at 100% (1082 x 822 physical pixels) and with
  the real EVO4 connected at 150% (1622 x 1217). Rounded controls render without
  square gradient artifacts. Live segmented input meters were observed.
- A 793 x 599 physical-pixel window at 150% keeps its header and footer visible.
  Horizontal scrolling reaches 96 kHz, buffer 4096 and Refresh; scrolling down
  reaches Loopback R's level, pan and mute. Settings inspection made no rate,
  buffer or hardware control changes.
- main.rs, controller.rs, driver.rs, model.rs and routing.rs match the
  0.1.0 source byte for byte. Production ui_bridge.rs is unchanged; only its
  interactive test's evidence filename and timeout changed. This release adds no
  new driver API or audio-stream behavior.

Fresh connected startup and package checks are recorded separately in
evidence/orbit-package-verification.json and evidence/orbit-layout-qa.json.
The initial state for this session was output -10.5 dB, playback blend 100%,
input gains 1.5/1 dB, both phantom switches off, 96 kHz and buffer 1024.
The earlier -16.5 dB values below are historical test restoration values.

The bounded audio-continuity, Smartgain, routing and reconnect measurements
below belong to 0.1.0 and were not repeated for this visual update.

# Historical version 0.1.0 acceptance status

The first usable Windows x64 release passed its applicable acceptance gates on
the inspected EVO4 and Audient API5.70. This audit is dated2026-10-20. Measurements
are bounded to this configuration; they do not establish hours-long reliability.

## Live evidence

- Native startup diagnostics detect exactly one physical EVO4, read 96 kHz /
  1024-frame configuration and report working optional capabilities. Connecting
  and polling issue no application control writes; this is corroborated by source
  review and unchanged readable state, not a complete USB traffic capture.
- Actual native UI edits independently read back correctly: both input gains and
  mutes, both 48V switches, stereo link, output level/mute, monitor blend and pan.
  Both 48V tests used empty inputs. Each setting was restored; speaker output is
  temporarily muted during input/routing experiments, then restored unmuted.
  Individual results are `evidence/native-ui-*.json`; final restoration is
  `evidence/native-session-restored.json`.
- Smartgain reached completion state4 after16.6seconds with a looping DJ console
  signal, selecting7.5/8.5dB. The production Rust driver test then restored the
  original1.5/1dB and stopped Smartgain. Both48V states remained off. See
  `evidence/native-dj-smartgain-production.json`. Earlier start/cancel and failed
  no-signal observations remain available. A stopped/restarted source confounded
  the earlier output-mute comparison; no causal mute/failure claim is made.
- Native UI rate changes 96 -> 48 -> 96 kHz and buffer changes 1024 -> 512 -> 1024
  passed. The final inline buffer buttons also passed a fresh change/restore.
  Supported choices are queried again after changes.
- The user turned the physical output knob from-80 to-73dB. The already-running
  native window's number and fader matched independent driver readback without
  any intervening app edit. See `evidence/native-knob-after-human.json` and
  `evidence/native-physical-knob-ui.json`.
- Physical USB disconnect/reconnect passed in the same portable process16040 and
  window8916676. All device controls disabled while absent and re-enabled after
  reconnection. Independent native diagnostics returned disconnected exit2, then
  connected exit0 with96kHz/1024frames, original input gains and48Voff. The native
  window matched the fresh driver values, including-73dB from the knob test. No
  app control edit occurred during the cycle. See `evidence/native-hotplug-disconnected-ui.json`,
  `native-hotplug-reconnected-ui.json` and `native-hotplug-reconnected-state.json`.
  Startup/reconnect writes remain absent by source review; no complete USB write
  trace was captured, and the simultaneously running OEM process is an external
  controller. This is not proof of every possible concurrent-edit ordering.
- Final cleanup restored output-16.5dB and unmuted state. Input gains are1.5/1dB,
  both48Voff, linkoff, monitor pan centered, blend105/127 and96kHz/1024frames.
  See `evidence/native-final-restored-state.json`.
- Three 12.5-second ASIO trials passed at 96 kHz / 1024 frames: baseline,
  attenuate/restore and attenuate/restore with native GUI meter/state polling.
  Each performed 63 state reads. There were zero reset, resync, overload,
  rate-change or sample-position-gap reports, zero missing pilot windows and
  matching restored readable state. See `evidence/asio-baseline.json`,
  `evidence/asio-write.json`, `evidence/asio-native-window-write.json`.
- A subsequent 60-second ASIO run with the native window open passed 300 state
  reads and 2,937 analyzed pilot windows with zero detected dropouts, phase
  discontinuities or ASIO timing/reset events. This remains a bounded trial,
  not an hours-long recording test. See `evidence/asio-native-window-60s.json`.
- The original WASAPI digital-loopback read/change/restore probe also passed.
  Neither digital test measures physical speaker/headphone continuity.
- The production Rust matrix setter passed 12 measured cases across Main L,
  Main R, Loopback L and Loopback R: attenuated hard pan, center pan and mute.
  Baseline signal pattern was verified before and after, with no restoration
  errors. See `evidence/native-matrix-validation-high-snr.json`.
- Both physical-input sources passed six further gain/pan/mute cases against
  simultaneous ASIO physical capture. Baseline and restoration matched, with no
  ASIO event or restoration error. The actual baseline mixed both inputs to both
  destinations at-20.75dB, equivalent to-14.75dB centered source routes. This
  differed from OEM saved pan intention; the first baseline guard correctly
  refused to write. The explicit measured configuration was then tested and
  restored. See `evidence/native-dj-input-matrix.json`. Hardware matrix registers
  remain unreadable; audio equivalence is the restoration evidence.
- Earlier quiet-pilot main-playback trials failed amid unrelated PC playback.
  These failures remain in the evidence directory. A stronger pilot with outputs
  muted passed the unchanged amplitude tolerance. This is a bounded measurement,
  not a long-running stress test.
- A read-only 470-byte USB configuration descriptor confirms a six-source,
  two-destination mixer at unit60. Live line signals produce nonzero input
  meters and visible native bars; their fill now starts at the left edge.
  Descriptor ABI and evidence are documented in PROTOCOL.md;
  firmware version semantics remain unknown.

## UI, implementation and local checks

- Native Rust + Slint Windows x64 executable; one charcoal/green window; no
  webview/server, Python runtime or replacement audio driver needed to run it.
- Green custom sliders, inline rate/buffer buttons and two-step 48V activation.
  Pointer dragging, keyboard editing and accessibility were inspected in the
  actual native window. At 150% application scaling and 801x600 resizing,
  scrolling reaches all settings and all six loopback strips.
  See `evidence/ui-connected-layout-qa.json` and adjacent screenshots.
- All 34 UI-only synchronization cases across 17 sliders pass, including edit
  rejection rollback and external authoritative changes. These simulated checks
  are separate from the live driver readbacks.
- Serialized worker, bounded/coalescing queues, connection generations, no cached
  writes on startup/reconnect and explicit write-only routing provenance.
  The physical reconnect test now corroborates recovery; stale-generation and
  bounded-queue behavior are also checked locally.
- 21 Rust release tests pass. Six fault cases exercise the production output
  transaction: preserve both originals before writes, attempt both restorations
  after uncertain writes/readbacks and report unconfirmed restoration.
- Four Smartgain tests verify preserved hardware arming bits, cancellation after
  uncertain setup/read/selection/start requests, refusal to restart active
  calibration and explicit cancellation failures. OEM REA evidence established
  that selecting inputs changes only bit0 of selector4, preserving other bits.
- Nine Python tests pass for pilot faults, short captures, device loss, partial
  writes and restoration errors. Harnesses retain errors and reject incomplete
  restoration rather than claiming success from eventual matching state.
- Ten-second connected idle sample with polling: 29.25 MiB working set,
  9.48 MiB private memory and about 1.4% of one CPU core. A further live-input
  sample of preview3 measured29.81MiB working set,9.29MiB private memory and
  4.36% of one core. Release executable13,905,408bytes. See
  `evidence/native-connected-performance.json` and `native-live-input-performance.json`. These short samples
  do not establish long-run memory use or every machine's performance.
- Portable release includes source, locked dependencies, build instructions,
  GPL-3.0-only license and third-party notices. It loads the installed verified
  Audient API DLL; no Audient DLL is redistributed. Final package smoke results
  are retained in `evidence/package-verification.json`.
- The portable executable launched from its package directory and displayed the
  connected device with the restored input/output values. Its read-only CLI also
  returned success and matching state. Earlier preview3 evidence:
  `evidence/packaged-preview3-ui-smoke.json` and `evidence/packaged-preview3-state.json`.
  Final archive/launch checks are recorded separately in `evidence/package-verification.json`.

## Known limits

Sleep/resume and hours-long streaming have not been exercised.

Physical analog continuity needs an external recording path and remains an
explicit measurement boundary, not a claim of click-free output. Firmware
flashing is an OEM handoff for version1. The installed Audient driver remains
required. Firmware version decoding, other devices/DLL versions and other
Windows configurations are outside the current compatibility claim.
