# ASIO coexistence test

`asio_host.cpp` is a minimal native Windows x64 ASIO host. It instantiates the
installed Audient COM driver and queries current configuration. Without an
argument it exits after printing inventory. With a temporary capture path it
streams a quiet 997+1613 Hz pilot for 12.5 seconds through ASIO output channels
3/4 while capturing ASIO input channels 3/4. On EVO4 these are the loopback
channels. There are no sample-rate, buffer-size, mixer, phantom-power, or input
gain writes.

The native callback copies captured audio and generates output without Python
or a GIL. Timing messages track ASIO sample positions, reset/resync requests,
overloads, and sample-rate changes. The test refuses unknown selected sample
types, a current rate other than 96 kHz, fewer than four channels per
direction, or an unexpected preferred buffer size. This narrowly scoped
restriction is deliberate; the application must eventually support the full
device capability range.

Build from the repository root using the installed VS Build Tools:

```powershell
cmd /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul && cl /nologo /EHsc /std:c++17 /O2 tools\asio_host.cpp /Fetools\asio_host.exe /Fotools\asio_host.obj ole32.lib user32.lib'
./tools/asio_host.exe
.venv/Scripts/python.exe tools/asio_continuity.py --output evidence/asio-baseline.json
.venv/Scripts/python.exe tools/asio_continuity.py --change --output evidence/asio-write.json
```

`asio_continuity.py` opens the verified Audient control API and queries state
at approximately 5 Hz while the separate native process streams. In `--change`
mode, after a previously passing baseline is present, it attenuates both
output channels by exactly 1 dB at approximately four seconds, verifies
readback, and restores the original values at eight seconds. A `finally`
block attempts restoration on exceptions as well. Do not turn the hardware
knob or use another controller during this bounded experiment.

The host temporarily saves raw captured loopback audio solely for analysis;
the orchestrator deletes it in `finally`. Only aggregate measurements are
retained. As in the prior WASAPI probe, the test signal must reach the
existing loopback routing; no route is created automatically.

Pass criteria require two expected control events, at least 40 successful
concurrent state reads, restored device state, continuous ASIO sample
positions, no ASIO reset/resync/overload/rate-change notifications, and a
clean pilot measurement after one second of warmup. The analyzer fits both
pilots in 20 ms windows and checks amplitude loss, phase discontinuities,
and residual error. This verifies digital streaming continuity; it does not
measure physical headphone or speaker continuity.

Current evidence: baseline and attenuate/restore runs passed at 96 kHz,
1024 frames, with 4 inputs/4 outputs and signed 32-bit little-endian samples.
The write experiment was repeated with native GUI polling active. Each run
completed 63 state reads with zero reset/resync/overload/rate-change/position-gap
notifications and no missing pilot windows. See `evidence/asio-baseline.json`,
`evidence/asio-write.json` and `evidence/asio-native-window-write.json`.
The subsequent `--seconds 60` read run also passed with the native window open:
300 state reads, 2937 analyzed pilot windows, no missing pilots or ASIO timing
events. See `evidence/asio-native-window-60s.json`. `--seconds` defaults to12.5
and is bounded to12.5..120; longer sessions remain untested.

`asio_matrix_host.cpp` and `asio_matrix_validation.py` are a separate routing
experiment. They require physical output already muted, the known OEM saved
routing intention, and the current 96 kHz configuration. Four distinct pilots
are sent to the four playback channels. A development-only Rust example calls
the production matrix setter. All four playback sources passed attenuated hard
pan, center pan and mute, then their original intended routes were restored and
the baseline digital signal pattern verified. The matrix cannot be read back.
Physical input routes are untouched. Aggregate measurements are retained; raw
audio is deleted. The authoritative result is
`evidence/native-matrix-validation-high-snr.json`.

Earlier quiet-pilot main-playback trials failed because unrelated PC playback
dominated the capture. Those reports are retained. The final trial increased the
pilot from 0.001 to 0.01 while physical outputs remained muted; the acceptance
tolerance stayed at 3% relative / 0.000003 absolute amplitude. Dedicated
loopback-source measurements were clean even with the quieter pilot.
