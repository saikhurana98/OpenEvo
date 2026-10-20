# Changelog

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
