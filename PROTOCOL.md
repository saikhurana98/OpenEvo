# Windows control contracts

These notes describe the inspected installed binaries, not a public SDK ABI.
Exact artifact hashes are in FEASIBILITY.md. RELEASE_STATUS.md distinguishes
live control/readback checks from static contracts and remaining signal tests.

## Driver transport

- OEM FUN_1402e77e0 dynamically loads TUSBAUDIO exports.
- OEM FUN_1402edc10 calls EnumerateDevices() without arguments, then Count(),
  OpenDeviceByIndex(index,&u32_token), and CloseDevice(token). This corroborates
  the production zero-argument enumeration call, superseding the conservative
  three-zero-argument invocation in the historical Python probe.
- OEM FUN_1402eba50 dispatches AudioControlRequestGet/Set with unit, request,
  selector, channel, data pointer, length, transferred pointer and timeout.
- API DLL workers validate32-bit tokens and pack wIndex=unit<<8,
  wValue=selector<<8|channel. Interface0. GET length is checked by this app.

## EVO4 controls

| Function | Unit | Request | Selector | Channel | Data |
|---|---:|---:|---:|---:|---|
| Output volume |10|1|2|1/2|signed16 Q8.8dB|
| Input gain |11|1|2|1/2|signed16 Q8.8dB|
| Input48V |58|1|0|0/1|u32 boolean|
| Input mute |58|1|2|0/1|u32 boolean|
| Input stereo link |58|1|3|0|u32 boolean|
| Smartgain selected inputs |58|1|4|0/1|u32 boolean|
| Smartgain state/command |58|1|5|0|u32 enum|
| Output mute |59|1|1|0|u32 boolean|
| Monitor blend |56|1|0|0|u32,0=input,127=playback|
| Monitor pan |56|1|1|0|u32,0=left,50=center,100=right|
| Loopback crosspoint |60|1|1|0..11|signed16 Q8.8dB,write-only|
| Mixer source meters |60|3|0|0|six unsigned16 peaks|

The OEM uses4-byte blend/pan payloads. The earlier probe's2-byte blend GET
succeeded but does not establish the correct SET width. The native adapter
follows the OEM4-byte form.

Output-level writes preserve both originals before touching hardware. A failed
write can already have reached the device; any write/readback failure therefore
attempts both original-channel writes and both readbacks. A restoration failure
on one channel does not skip the other, and unconfirmed restoration is reported.
Fault-injected tests exercise this production transaction without loading the DLL.

Smartgain setup writes state1, changes bit0 of each selector4 arm word while
preserving all other bits, then writes2 to start. OEM compact controls also use
bit1 of the same words. State3 has the explicit OEM label "Smart gain listening";
green/red presentations support the completion/failure interpretation of4/5.
The adapter cancels setup after any uncertain setup/selection/start request and
reports an unconfirmed cancellation. A live DJ-line-signal run reached4 after
16.6 seconds and selected7.5/8.5dB. The test restored1.5/1dB and state0.
See evidence/native-dj-smartgain-production.json and oem-smartgain-findings.md.

Meter samples are unsigned16 little-endian, normalized by65535. OEM
FUN_1402eb780 builds request3; FUN_1400dc3d0 reads source meters and forwards
physical input samples to InputControlModel. FUN_14012e530 converts to dB.
The native implementation reads MU60's six-input layout and disables meters on
failure. A live 470-byte USB configuration descriptor confirms mixer unit60 with
two outputs, fed by extension unit50 with six source channels. The 12-byte meter
GET succeeds. Live line input signals produce nonzero meter values and visible
native bars. Empty/stopped sources return near-zero values. The exact firmware
meter tap relative to all other controls is not established.

Version 0.3 consumes all six entries from this same 12-byte request. Entries
0/1 drive input meters, and entries 2/3 drive the labelled main-playback L/R
meters and master activity ring. Existing OEM source mapping establishes their
order; a fresh live read returns nonzero playback values independently of silent
inputs. This adds no USB transaction or streaming audio client. It does not
establish a post-volume, direct-monitor-sum or analog-output meter.
Display attack/hold/release smoothing runs only on the UI thread.

Loopback crosspoint=input_index*2+destination_index, with six mono source strips.
OEM MixerNode FUN_1400d76f0 uses pan p in0..1 and k=2−4*10^(−0.3):
A(x)=((k*x+1)−k)*x. Left receives A(1−p), right A(p). Each destination dB
is gain+20log10(A), clamped to−128..+6, then Q8.8 truncated toward zero.
The zero-amplitude branch substitutes−128dB before adding gain. Center pan
attenuates each destination by6dB. Explicit mute writes−128dB to both.
FUN_1402ecfd0 supplies the crosspoint packing and quantization. Stereo link in
the OEM pairs level listeners and hard-pans channels; it does not establish
the earlier prototype's four-source balance behavior. The app now exposes all
six sources independently. Live ASIO measurements corroborate gain, center/hard
pan and mute for all four playback sources (12 cases), and both physical-input
sources (6 cases) measured against simultaneous physical ASIO capture. The input
baseline was centered at-14.75dB despite the OEM's saved hard-pan intention;
the measured baseline was restored. Failed partial writes make the source unknown.
Cached routing is never represented as hardware readback; schema2 intentionally
rejects earlier four-source caches.

## Driver settings

GetSupportedSampleRates(token,capacity,u32_array,&count) uses capacity32 and
copies u32 rates. SetSampleRate(token,rate) is followed by rate readback.

GetASIOInstanceInfo(0,buffer) writes0xC4 bytes. OEM reads preferred frames at
offset28 and32 supported-size slots at offset36. FUN_1402ebd60 calls
SetASIOBufferPreferredSize(0,info[0],size,0x10000). The adapter checks the allowed
list and re-reads preference. Native UI checks passed 96→48→96 kHz and
1024→512→1024 frames, including the final inline buffer buttons. The driver
changed the preferred buffer when switching rates; the app re-queries its list.

## Live descriptor API

REA traced these read-only exports, then a bounded process queried the device:

```c
uint32_t TUSBAUDIO_GetUsbConfigDescriptor(uint32_t token, void *buffer,
                                        uint32_t capacity, uint32_t *returned);
uint32_t TUSBAUDIO_GetDeviceProperties(uint32_t token, void *buffer); // 1040 bytes
```

The configuration call does not support a null size query; the probe allocates
65535 bytes and validates the returned descriptor walk. Configuration SHA-256:
`23b86d61d633f241da48df2c5cebd5fba42a5a333554474b829755514c144600`.
These calls have no caller timeout, so the probe runs outside the UI process.
The property block's firmware/version semantics remain unknown. No firmware
version is inferred from opaque fields or the malformed PnP revision string.

## REA ledger

Full records: evidence/oem-controls-rea.json, evidence/driver-settings-rea.json,
and evidence/oem-pan-rea.json. Detailed pan/meter notes are in
evidence/oem-pan-findings.md.
Reusable snapshots sit beside them. Important records:

- Enumeration: ev_329a94256f1460b4bac473e7f5f367ea943a54620b46ea9a93c77b0dc235f4b2.
- Transport: ev_b86ee9d41f5f9690814f002b0e11bac4cfbc7e1eb6eaf25b4ed9d03a04109291.
- Input model/unit58: ev_b5e0471a94493ceb93ff005c1ab2c80d3a53441aaa98cab7811756db0da244a3.
- Property writer: ev_e44fd23f31a6658caed123df663cd8b1e4f29b8d45b86591be0f2aa53a8f2e7e.
- Smartgain: ev_0fb9e1e6f983e2c6ba7b139baaba98a5a3d65b1266a2d01eca0939a35bb4ba61.
- Link: ev_9ee5cd4c2e078fe01adf86eae1eb67c008a05933f2dd52b5b7951032ec4e10e3.
- Blend SET: ev_020c32f1b6eb5ad27e96abbe74a76d773bc0657232a1956676e7513f831eb21f.
- Pan SET: ev_306e18f8e80e77ccc76bd69548eed734001c1754ed64c43330d500f04571aa15.
- Meter request: ev_bbf998ea4307fd1add4c8f0e71dcd67f331a913e7dae40f13a44da8f5498b11b.
- ASIO setting: ev_65943987877581c19b86b9e67a0b102dbe91099dd16ca827bbdad40ef7c3f8cc.
- ASIO DLL length: ev_9e39bb42bc244f345e63f593bf1c7cbbff53d3820a500e22d1fa7f3c2fe43ed8.
- Pan law: ev_55ca2030b6fdae346f92fd6aad5e40c3e4a422eb1c17c05eba7741241079138c.
- Crosspoint quantization: ev_1238b993ce6b5277f6b207a6bcdfa5e5f1e8ea4642d00766876663846d3cc8ac.
- USB configuration ABI: ev_68cdb7a9a2221b2bd4aa6de79c90100906a5f95365724b50a112141d0ac06e64.
- Device properties ABI: ev_dd04ae2b7d67b789ae6f13499e8c19ffaf370b77fbe38d29f1339b1e421ebeac.
- Smartgain arm bits: ev_d6551d960ed71ee2ef08200bfd98f25bae197b66cd663d9a2c8e972d8456cad2.
