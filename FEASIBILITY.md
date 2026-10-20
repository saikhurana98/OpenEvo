# Windows feasibility findings

Investigation date: 18 October 2026 (user timezone: Asia/Calcutta).

## Conclusion

**PASS, bounded to the tested configuration.** The installed Audient API can
enumerate/open the EVO4, read state, and perform a reversible output-volume
write while the device's stereo digital loopback stream remains continuous.
The OEM EVO process remained running. This feasibility probe preceded UI work;
the later native preview and its separate verification status are described in
NATIVE_README.md and RELEASE_STATUS.md.

This is not a claim of full OEM parity, production reliability, ASIO coexistence,
or verified analog-output continuity. Those remain separate validation gates.

Later offline hardening preserves the aggregate failure report when restoration,
final readback or capture analysis fails. Nine offline Python tests exercise the
detector and failure paths. The historical source hashes below identify the
original live experiment; current harness hashes are retained separately in
evidence/probe-source-hashes-current.json. No new live proof is implied.

## Target identity

- Device: present Windows PnP device `USB\\VID_2708&PID_0006` (Audient EVO4).
- Installed OEM application: EVO 4.4.0.
- Driver package display name: Audient USB Audio Driver v5.4.0.
- API DLL file version: 5.70.0.0. Package and DLL versions differ; record both.
- DLL: `C:\Program Files\Audient\USBAudioDriver\x64\audientusbaudioapi_x64.dll`.
- DLL SHA-256: `25af306d32bb674796bfe82ba220396254d97e8859e024394fbeda7db242b2e6`.
- `TUSBAUDIO_GetApiVersion()` returned `0x0005000d`.
- Windows x64 ABI. Device handles are 32-bit tokens, not native HANDLE pointers.

## Static verification with REA

REA/Ghidra inspected the actual installed DLL. Export wrappers frequently appear
as `void` in decompiler output, but their tail jumps/return instructions preserve
the worker's 32-bit status return. Worker functions establish output widths,
device-token lookup, request packing, and zero-success/error returns.

The API initialization reads `custom.ini` beside the DLL and obtains the vendor
driver interface GUID. This explains why loading the original installed DLL is
preferable to copying it elsewhere.

Exercised ABI, reconstructed for this exact binary:

```c
uint32_t TUSBAUDIO_GetApiVersion(void);
uint32_t TUSBAUDIO_GetDeviceCount(void);
uint32_t TUSBAUDIO_OpenDeviceByIndex(uint32_t index, uint32_t *handle);
uint32_t TUSBAUDIO_CloseDevice(uint32_t handle);
uint32_t TUSBAUDIO_GetCurrentSampleRate(uint32_t handle, uint32_t *rate);
uint32_t TUSBAUDIO_AudioControlRequestGet(
    uint32_t handle, uint8_t unit, uint8_t request,
    uint8_t selector, uint8_t channel, void *buffer, uint32_t bytes,
    uint32_t *transferred, uint32_t timeout);
// AudioControlRequestSet uses the same argument layout.
```

Enumeration deserves a specific caveat: the export jumps to a worker after
replacing RCX with the singleton object. The worker leaves RDX unused and passes
the high bytes of R8 into enumeration flags while setting the low byte itself.
The harness passes three zero integers to make these values deterministic.
This is an exercised binary-specific invocation, **not a recovered public SDK
prototype**. A production wrapper should resolve the documented API contract
or confirm the OEM call site before supporting other DLL versions. Subsequent
REA inspection of OEM FUN_1402edc10 confirmed a zero-argument call; the native
adapter follows that call site. See PROTOCOL.md for the retained evidence.

The control worker builds `wIndex = unit << 8` and
`wValue = selector << 8 | channel`; the interface byte is zero. Request 1 is
used here. Output volume uses unit 10, selector 2, channels 1 and 2, signed
16-bit values in 1/256 dB. GET byte counts are checked. Writes are allowlisted
to those two output-volume controls.

The high-level GetVolume/SetVolume exports were inspected but not used: their
channel/direction mapping was unnecessary for this narrower verified route.

### Evidence ledger

Complete records are in `evidence/driver-api-rea.json`; an exact-target reusable
analysis snapshot is in `evidence/driver-api-snapshot.json`.

| Finding | REA Evidence ID |
|---|---|
| Relevant exported functions | `ev_0c12e05adab50508737fcc4368ac6aae08ce35134dfe994259cc7c2d45da9e1b` |
| OpenDeviceByIndex writes a 32-bit token | `ev_a5e6abe8efdf698fa9b8ad2c5d9aa11e470cc4a738e75fcdc6601b3a27d742d3` |
| Control GET export argument layout | `ev_cab86ed369a9da51ecbfc29ed99f3918f833c11aa9f162ea8decef96cf633143` |
| Control SET export argument layout | `ev_064f7cc17f602b425352eb1d278043c937fcaf4fce69128116ef232ba38ba509` |
| USB request packing and token lookup | `ev_23d823888636a347d36567339b449b79c516cd42c2e46c0767c3f5930e08414d` |
| GET IOCTL and returned byte count | `ev_cc9a8f09c75058be90297580f59cf3f5f4b1ffccab6d29bfb9bebdd9e8a785ef` |
| SET IOCTL | `ev_641191ec4433cf6feeb2b78d5c42ad7be7bf2110dd874f246ab53c61b48a9de1` |
| custom.ini initialization | `ev_72c83343cdc53796d1f58aec1164fa100286f378e54b744e9f5d0c97e8812a7c` |
| Sample-rate read | `ev_80a7af334ede7cbdcce6fa705fd38f9eb22baaf83991bb432d9a18810a90a6cc` |

These are static observations/reconstructions. Runtime results below corroborate
the narrow calls actually exercised, not every export or inferred parameter.

## Runtime experiments

1. Read-only probe: opened one device and read 96,000 Hz, output raw -4224/-4224
   (-16.5 dB), input gain raw 384/256, monitor mix raw 105.
2. Main Output playback -> hardware loopback capture: pilot absent under the
   existing mix. Stopped before any write. Preserved routing and all read state.
3. Dedicated Loop-back playback -> hardware Loop-back capture: clean baseline.
4. Same dedicated path with output attenuation/restoration: passed.
5. Final repeat after adding identity checks and explicit pass criteria: passed.

The final run used WASAPI shared mode, stereo float32 at the already configured
96 kHz. The source contains 997 Hz and 1613 Hz pilots, each at 0.0025 amplitude
(combined peak at most 0.005, approximately -46 dBFS). It captures the Audient
hardware Loop-back input, not Windows software loopback.

At about 4.01 seconds, both output registers changed to -4480 (-17.5 dB).
At about 8.06 seconds, both returned to -4224 (-16.5 dB). Both transitions were
read back successfully. Input gains, sample rate, and monitor blend matched
the initial snapshot at the end. Mixer routing and phantom power were not written.

| Metric | Baseline | Final write run |
|---|---:|---:|
| Captured stereo frames | 1,158,720 | 1,158,720 |
| Duration represented | 12.07 s | 12.07 s |
| Callback count | 1,207 | 1,207 |
| Callback underflow/overflow flags | 0 | 0 |
| Analyzed 20-ms windows | 541 | 541 |
| Weak pilot windows | 0 | 0 |
| Phase steps above 0.1 rad | 0 | 0 |
| Maximum phase step | 0.000179 rad | 0.000169 rad |
| Maximum residual / signal RMS | 0.000898 | 0.000882 |
| Stream active at end | yes | yes |
| Read state restored | yes | yes |

Analysis excludes the first second and final quarter-second, retaining both
setting transitions. The app reads state approximately five times per second
during the stream. Each 20-ms window is fit against both known sine/cosine pairs
and a DC term. Passing requires pilots in both channels, no windows below half
median pilot amplitude, no phase steps over 0.1 rad, residual RMS below 1% of
signal RMS, no post-warmup callback flags, two completed event markers, an active
stream, and final state restoration.

Four offline tests verify acceptance of the clean pilot and rejection of
silence, a synthetic 5-ms dropout, and a synthetic sample/phase discontinuity.
These thresholds are a bounded detector, not proof that arbitrarily short or
low-amplitude artifacts cannot occur.

## Implications and remaining gates

- Windows transport is feasible through the installed user-mode API without
  replacing the audio driver. Port the exercised calls into a narrow Rust FFI
  module with the same ownership, length, status, and version checks.
- The successful measured path is USB playback 3/4 -> the EVO's digital loopback
  capture. It does not traverse the analog output controlled by the attenuator.
  The register change is proven by readback; its analog magnitude and absence
  of analog clicks need an external recorder or physical loopback cable.
- ASIO/DAW coexistence, low-buffer operation, long-duration stress, hotplug,
  sleep/resume, and other driver versions are untested. The PortAudio runtime
  used for this experiment exposes WASAPI but no ASIO host.
- The OEM process remained alive; conflict resolution for simultaneous edits
  was not exercised and should not be inferred from that coexistence.
- Firmware identity, Smartgain, meters, and the remaining controls are outside
  this first probe. No SDK compatibility or redistribution permission is
  inferred from the presence of DLL exports.
- Before UI integration, resolve the enumeration API contract and add an ASIO
  streaming test. Retain the existing driver dependency explicitly.

Reference for stream callback semantics and status flags:
https://python-sounddevice.readthedocs.io/en/latest/api/streams.html
