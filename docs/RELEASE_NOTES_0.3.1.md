# OpenEvo v0.3.1

The first public release of OpenEvo: a modern native Windows control app for
Audient EVO 4, built with Rust and Slint.

## Included

- Input gain, mute, two-step 48V activation, Smartgain and stereo link.
- Output volume/mute, monitor mix and monitor pan.
- Six-source loopback mixer with individual level, pan and mute controls.
- Smooth input/playback meters, master activity ring and numeric transitions.
- Dense responsive Orbit layout, tested at portrait half/third snap sizes.
- Sample-rate/buffer settings, physical knob sync and USB reconnect handling.

## Download

Extract **OpenEvo-0.3.1-windows-x64.zip** and run **OpenEvo.exe**.
The ZIP contains the Windows x64 executable, application source, build
instructions, notices and an internal file checksum manifest. The separate
**SHA256SUMS.txt** verifies the ZIP itself. GitHub's source archives contain
the full public repository, including research/test harnesses and documentation.

Requires the installed Audient Windows driver. The supported API DLL is version
5.70.0.0 with SHA-256
`25af306d32bb674796bfe82ba220396254d97e8859e024394fbeda7db242b2e6`.
This early release supports one connected EVO 4 and rejects unverified driver
DLLs or other Audient models. The executable is unsigned.

## Validation and limits

The branded release passes 24 automated Rust tests and nine offline Python
probe regressions. Earlier native UI regressions cover 18 compact-console
state-sync cases, 18 inspector cases and 34 full-console cases. Those UI tests
use simulated state and make no hardware calls. The renamed application was
also inspected connected to the EVO 4, with live playback activity.

Earlier hardware tests covered control readback, Smartgain, all six mixer
sources, physical knob sync, USB reconnect and bounded ASIO/WASAPI coexistence.
Those measurements were not repeated for the rename. Analog click-free output,
sleep/resume and hours-long streaming remain unverified.

Loopback routes are write-only: saved intent is shown honestly and is not
replayed on startup/reconnect. Playback meters are not post-volume analog
meters. Driver/firmware binaries are not included; use Audient's official
updater for firmware. Existing EvoControl cache data is retained.

GPL-3.0-only. Independent software, not affiliated with or endorsed by Audient.
