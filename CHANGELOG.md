# Changelog

## 0.3.2 — Motion polish

- Short eased hover/focus colors, button press feedback with fixed hitboxes,
  and encoder hover/press highlights.
- Inspectors slide/fade in and retain outgoing content through their exit.
  Rapid navigation resolves to the latest page. Covered and exiting controls
  cannot accept edits.
- Responsive console cards, dials and mixer strips interpolate as space changes;
  the existing dense layout and conditional disclosure thresholds are preserved.
- Respect Windows' client-area animation preference at startup. Reduced motion
  removes decorative transitions and setpoint interpolation while retaining
  the existing audio meter ballistics.
- Offscreen software-renderer checks cover timing, rapid switching, hitboxes,
  all 17 continuous controls, five window sizes and settled animation activity.
- Hardware commands and actual slider values remain immediate. Motion produces
  no intermediate device writes and adds no continuous animation loop.

## 0.3.1 — First public OpenEvo release

- Native Rust + Slint control window for Audient EVO 4 on Windows x64.
- Input gain/mute/48V, Smartgain, stereo link, output volume/mute and monitoring.
- Six-source loopback mixer with level, pan and mute for every source.
- Dense Orbit layout, full controls at portrait half/third snaps, and adaptive
  disclosure only when controls cannot fit; narrow tall windows use a 3 × 2 mixer.
- Live input/playback meters and master activity ring with smooth ballistics;
  eased knob positions and fixed-width numeric readouts.
- Serialized driver commands, bounded queues, reconnect generation checks and
  rollback/readback handling. No startup replay of cached routing.
- Portable Windows ZIP with application source, notices and checksum manifest.

Earlier 0.1–0.3 builds were local development builds under the EVO Control name.
Existing saved loopback intent remains in the legacy EvoControl cache directory.
