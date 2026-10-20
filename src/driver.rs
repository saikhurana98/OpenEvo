//! Windows adapter for the installed Audient API. Every call stays on one worker.
use libloading::Library;
use sha2::{Digest, Sha256};
use std::ffi::c_void;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, String>;
const TESTED_HASH: &str = "25af306d32bb674796bfe82ba220396254d97e8859e024394fbeda7db242b2e6";
// OEM FUN_1402edc10 calls this export without arguments.
type Enumerate = unsafe extern "system" fn() -> u32;
type Count = unsafe extern "system" fn() -> u32;
type Open = unsafe extern "system" fn(u32, *mut u32) -> u32;
type Close = unsafe extern "system" fn(u32) -> u32;
type Rate = unsafe extern "system" fn(u32, *mut u32) -> u32;
type SetRate = unsafe extern "system" fn(u32, u32) -> u32;
type SupportedRates = unsafe extern "system" fn(u32, u32, *mut u32, *mut u32) -> u32;
type AsioInfo = unsafe extern "system" fn(u32, *mut u32) -> u32;
type SetBuffer = unsafe extern "system" fn(u32, u32, u32, u32) -> u32;
type Control =
    unsafe extern "system" fn(u32, u8, u8, u8, u8, *mut c_void, u32, *mut u32, u32) -> u32;

pub struct Device {
    // The library must outlive all function pointers and the open token.
    _library: Library,
    handle: u32,
    close: Close,
    rate: Rate,
    set_rate: SetRate,
    supported_rates: SupportedRates,
    asio_info: AsioInfo,
    set_buffer: SetBuffer,
    get: Control,
    set: Control,
}

fn check(status: u32) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("Audient API returned 0x{status:08X}"))
    }
}

#[cfg(windows)]
fn verify_identity() -> Result<()> {
    #[repr(C)]
    struct Guid {
        a: u32,
        b: u16,
        c: u16,
        d: [u8; 8],
    }
    #[repr(C)]
    struct Info {
        size: u32,
        guid: Guid,
        instance: u32,
        reserved: usize,
    }
    #[link(name = "setupapi")]
    unsafe extern "system" {
        fn SetupDiGetClassDevsW(
            guid: *const Guid,
            enumerator: *const u16,
            parent: *mut c_void,
            flags: u32,
        ) -> *mut c_void;
        fn SetupDiEnumDeviceInfo(list: *mut c_void, index: u32, info: *mut Info) -> i32;
        fn SetupDiGetDeviceInstanceIdW(
            list: *mut c_void,
            info: *mut Info,
            buffer: *mut u16,
            size: u32,
            required: *mut u32,
        ) -> i32;
        fn SetupDiDestroyDeviceInfoList(list: *mut c_void) -> i32;
    }
    let guid = Guid {
        a: 0x572b9a62,
        b: 0x741a,
        c: 0x4a6f,
        d: [0x84, 0xa7, 0x84, 0xa1, 0x0f, 0x18, 0xd6, 0x4f],
    };
    unsafe {
        let list = SetupDiGetClassDevsW(&guid, std::ptr::null(), std::ptr::null_mut(), 0x12);
        if list as isize == -1 {
            return Err("Could not enumerate the installed Audient driver".into());
        }
        let result = (|| {
            let mut found = Vec::new();
            for index in 0..64 {
                let mut info = Info {
                    size: std::mem::size_of::<Info>() as u32,
                    guid: Guid {
                        a: 0,
                        b: 0,
                        c: 0,
                        d: [0; 8],
                    },
                    instance: 0,
                    reserved: 0,
                };
                if SetupDiEnumDeviceInfo(list, index, &mut info) == 0 {
                    break;
                }
                let mut text = [0u16; 512];
                let mut required = 0;
                if SetupDiGetDeviceInstanceIdW(
                    list,
                    &mut info,
                    text.as_mut_ptr(),
                    512,
                    &mut required,
                ) == 0
                {
                    return Err("Could not verify the connected device identity".into());
                }
                let end = text.iter().position(|&x| x == 0).unwrap_or(text.len());
                found.push(String::from_utf16_lossy(&text[..end]).to_uppercase());
            }
            if found.len() != 1 || !found[0].starts_with("USB\\VID_2708&PID_0006\\") {
                return Err(
                    "Connect one EVO 4. Other Audient models are not supported by this build."
                        .into(),
                );
            }
            Ok(())
        })();
        SetupDiDestroyDeviceInfoList(list);
        result
    }
}
#[cfg(not(windows))]
fn verify_identity() -> Result<()> {
    Err("This build requires Windows x64".into())
}

impl Device {
    pub fn open() -> Result<Self> {
        if !cfg!(all(windows, target_arch = "x86_64")) {
            return Err("Windows x64 is required".into());
        }
        verify_identity()?;
        let base = std::env::var_os("ProgramW6432")
            .or_else(|| std::env::var_os("ProgramFiles"))
            .ok_or("Program Files could not be located")?;
        let path =
            PathBuf::from(base).join("Audient/USBAudioDriver/x64/audientusbaudioapi_x64.dll");
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("Install the Audient EVO driver first: {e}"))?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        if hash != TESTED_HASH {
            return Err(
                "This Audient API version has not been validated. See compatibility notes.".into(),
            );
        }
        unsafe {
            let library = Library::new(&path)
                .map_err(|e| format!("Could not load Audient driver API: {e}"))?;
            let enumerate: Enumerate = *library
                .get(b"TUSBAUDIO_EnumerateDevices\0")
                .map_err(|e| e.to_string())?;
            let count: Count = *library
                .get(b"TUSBAUDIO_GetDeviceCount\0")
                .map_err(|e| e.to_string())?;
            let open: Open = *library
                .get(b"TUSBAUDIO_OpenDeviceByIndex\0")
                .map_err(|e| e.to_string())?;
            let close: Close = *library
                .get(b"TUSBAUDIO_CloseDevice\0")
                .map_err(|e| e.to_string())?;
            let rate: Rate = *library
                .get(b"TUSBAUDIO_GetCurrentSampleRate\0")
                .map_err(|e| e.to_string())?;
            let set_rate: SetRate = *library
                .get(b"TUSBAUDIO_SetSampleRate\0")
                .map_err(|e| e.to_string())?;
            let supported_rates: SupportedRates = *library
                .get(b"TUSBAUDIO_GetSupportedSampleRates\0")
                .map_err(|e| e.to_string())?;
            let asio_info: AsioInfo = *library
                .get(b"TUSBAUDIO_GetASIOInstanceInfo\0")
                .map_err(|e| e.to_string())?;
            let set_buffer: SetBuffer = *library
                .get(b"TUSBAUDIO_SetASIOBufferPreferredSize\0")
                .map_err(|e| e.to_string())?;
            let get: Control = *library
                .get(b"TUSBAUDIO_AudioControlRequestGet\0")
                .map_err(|e| e.to_string())?;
            let set: Control = *library
                .get(b"TUSBAUDIO_AudioControlRequestSet\0")
                .map_err(|e| e.to_string())?;
            check(enumerate())?;
            if count() != 1 {
                return Err("Expected exactly one EVO 4 in the Audient driver".into());
            }
            let mut handle = 0;
            check(open(0, &mut handle))?;
            if handle == 0 {
                return Err("Audient API returned an empty device handle".into());
            }
            Ok(Self {
                _library: library,
                handle,
                close,
                rate,
                set_rate,
                supported_rates,
                asio_info,
                set_buffer,
                get,
                set,
            })
        }
    }
    fn read<const N: usize>(&self, unit: u8, selector: u8, channel: u8) -> Result<[u8; N]> {
        self.request_read(unit, 1, selector, channel)
    }
    fn request_read<const N: usize>(
        &self,
        unit: u8,
        request: u8,
        selector: u8,
        channel: u8,
    ) -> Result<[u8; N]> {
        let mut data = [0u8; N];
        let mut transferred = 0;
        unsafe {
            check((self.get)(
                self.handle,
                unit,
                request,
                selector,
                channel,
                data.as_mut_ptr().cast(),
                N as u32,
                &mut transferred,
                500,
            ))?;
        }
        if transferred != N as u32 {
            return Err(format!("Short device response: {transferred}/{N} bytes"));
        }
        Ok(data)
    }
    fn write(&self, unit: u8, selector: u8, channel: u8, data: &mut [u8]) -> Result<()> {
        let mut transferred = 0;
        unsafe {
            check((self.set)(
                self.handle,
                unit,
                1,
                selector,
                channel,
                data.as_mut_ptr().cast(),
                data.len() as u32,
                &mut transferred,
                500,
            ))
        }
    }
    pub fn sample_rate(&self) -> Result<u32> {
        let mut value = 0;
        unsafe {
            check((self.rate)(self.handle, &mut value))?;
        }
        Ok(value)
    }
    pub fn supported_sample_rates(&self) -> Result<Vec<u32>> {
        let mut values = [0u32; 32];
        let mut count = 0;
        unsafe {
            check((self.supported_rates)(
                self.handle,
                32,
                values.as_mut_ptr(),
                &mut count,
            ))?;
        }
        if count > 32 {
            return Err("Invalid supported sample-rate count".into());
        }
        Ok(values[..count as usize].to_vec())
    }
    pub fn set_sample_rate(&self, rate: u32) -> Result<()> {
        if !self.supported_sample_rates()?.contains(&rate) {
            return Err("This sample rate is not supported by the device".into());
        }
        unsafe {
            check((self.set_rate)(self.handle, rate))?;
        }
        if self.sample_rate()? != rate {
            return Err("Sample-rate readback did not match".into());
        }
        Ok(())
    }
    fn raw_buffer_info(&self) -> Result<[u32; 49]> {
        // Driver writes exactly 0xC4 bytes. OEM reads preferred frames at +28
        // and the 32 allowed sizes at +36. Index0 is the driver ASIO instance.
        let mut info = [0u32; 49];
        unsafe {
            check((self.asio_info)(0, info.as_mut_ptr()))?;
        }
        Ok(info)
    }
    pub fn buffer_info(&self) -> Result<(u32, Vec<u32>)> {
        let info = self.raw_buffer_info()?;
        let mut allowed: Vec<u32> = info[9..41]
            .iter()
            .copied()
            .filter(|&x| x > 0 && x <= 65536)
            .collect();
        allowed.sort_unstable();
        allowed.dedup();
        if info[7] == 0 || allowed.is_empty() {
            return Err("ASIO buffer information is unavailable".into());
        }
        Ok((info[7], allowed))
    }
    pub fn set_buffer_size(&self, size: u32) -> Result<()> {
        let info = self.raw_buffer_info()?;
        if !info[9..41].contains(&size) || size == 0 {
            return Err("ASIO buffer size is not supported".into());
        }
        unsafe {
            check((self.set_buffer)(0, info[0], size, 0x10000))?;
        }
        if self.buffer_info()?.0 != size {
            return Err("ASIO buffer preference was not confirmed by the driver".into());
        }
        Ok(())
    }
    pub fn output_db(&self) -> Result<f32> {
        Ok(i16::from_le_bytes(self.read(10, 2, 1)?) as f32 / 256.0)
    }
    pub fn gain(&self, channel: u8) -> Result<f32> {
        check_channel(channel)?;
        Ok(i16::from_le_bytes(self.read(11, 2, channel + 1)?) as f32 / 256.0)
    }
    pub fn input_muted(&self, channel: u8) -> Result<bool> {
        check_channel(channel)?;
        control_bool(self.read(58, 2, channel)?)
    }
    pub fn phantom(&self, channel: u8) -> Result<bool> {
        check_channel(channel)?;
        control_bool(self.read(58, 0, channel)?)
    }
    pub fn output_muted(&self) -> Result<bool> {
        control_bool(self.read(59, 1, 0)?)
    }
    pub fn monitor_blend(&self) -> Result<f32> {
        let raw = u32::from_le_bytes(self.read(56, 0, 0)?);
        if raw > 127 {
            return Err("Unexpected monitor blend value".into());
        }
        Ok(raw as f32 * 100.0 / 127.0)
    }
    pub fn monitor_pan(&self) -> Result<f32> {
        let raw = u32::from_le_bytes(self.read(56, 1, 0)?);
        if raw > 100 {
            return Err("Unexpected monitor pan value".into());
        }
        Ok(raw as f32 * 2.0 - 100.0)
    }
    pub fn linked(&self) -> Result<bool> {
        control_bool(self.read(58, 3, 0)?)
    }
    pub fn smartgain_state(&self) -> Result<u32> {
        let value = u32::from_le_bytes(self.read(58, 5, 0)?);
        if value > 5 {
            return Err("Unexpected Smartgain state".into());
        }
        Ok(value)
    }
    /// OEM GET_MEM on mixer MU60 returns six unsigned 16-bit source peaks.
    pub fn source_peaks(&self) -> Result<[f32; 6]> {
        let data = self.request_read::<12>(60, 3, 0, 0)?;
        Ok(std::array::from_fn(|i| {
            u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]) as f32 / 65535.0
        }))
    }
    pub fn input_peaks(&self) -> Result<[f32; 2]> {
        let peaks = self.source_peaks()?;
        Ok([peaks[0], peaks[1]])
    }
    pub fn set_output_db(&self, db: f32) -> Result<()> {
        let raw = db_raw(db, -96.0, 0.0)?;
        set_stereo_output(raw, |channel, value| match value {
            Some(raw) => self
                .write(10, 2, channel, &mut raw.to_le_bytes())
                .map(|_| raw),
            None => self.read(10, 2, channel).map(i16::from_le_bytes),
        })
    }
    pub fn set_gain(&self, channel: u8, db: f32) -> Result<()> {
        check_channel(channel)?;
        let raw = db_raw(db, -8.0, 50.0)?.to_le_bytes();
        self.write(11, 2, channel + 1, &mut raw.clone())?;
        if self.read::<2>(11, 2, channel + 1)? != raw {
            return Err("Input gain readback did not match".into());
        }
        Ok(())
    }
    pub fn set_input_muted(&self, channel: u8, value: bool) -> Result<()> {
        check_channel(channel)?;
        self.write(58, 2, channel, &mut (value as u32).to_le_bytes())
    }
    pub fn set_phantom(&self, channel: u8, value: bool) -> Result<()> {
        check_channel(channel)?;
        self.write(58, 0, channel, &mut (value as u32).to_le_bytes())
    }
    pub fn set_output_muted(&self, value: bool) -> Result<()> {
        self.write(59, 1, 0, &mut (value as u32).to_le_bytes())
    }
    pub fn set_monitor_blend(&self, value: f32) -> Result<()> {
        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
            return Err("Monitor blend must be between 0 and 100".into());
        }
        let raw = ((value * 127.0 / 100.0).round() as u32).to_le_bytes();
        self.write(56, 0, 0, &mut raw.clone())?;
        if self.read::<4>(56, 0, 0)? != raw {
            return Err("Monitor blend readback did not match".into());
        }
        Ok(())
    }
    pub fn set_monitor_pan(&self, value: f32) -> Result<()> {
        if !value.is_finite() || !(-100.0..=100.0).contains(&value) {
            return Err("Monitor pan is outside its range".into());
        }
        let raw = (((value + 100.0) / 2.0).round() as u32).to_le_bytes();
        self.write(56, 1, 0, &mut raw.clone())?;
        if self.read::<4>(56, 1, 0)? != raw {
            return Err("Monitor pan readback did not match".into());
        }
        Ok(())
    }
    pub fn set_linked(&self, value: bool) -> Result<()> {
        self.write(58, 3, 0, &mut (value as u32).to_le_bytes())
    }
    pub fn start_smartgain(&self, mask: u8) -> Result<()> {
        start_smartgain_control(mask, |selector, channel, value| match value {
            Some(value) => {
                self.write(58, selector, channel, &mut value.to_le_bytes())?;
                Ok(value)
            }
            None => self.read(58, selector, channel).map(u32::from_le_bytes),
        })
    }
    pub fn stop_smartgain(&self) -> Result<()> {
        self.write(58, 5, 0, &mut 0u32.to_le_bytes())
    }
    pub fn set_loopback(&self, source: u8, route: crate::routing::Route) -> Result<()> {
        for (channel, value) in crate::routing::matrix(source, route)? {
            self.write(60, 1, channel, &mut value.to_le_bytes())?;
        }
        Ok(())
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            (self.close)(self.handle);
        }
    }
}

/// The OEM arm control changes only bit0 of CS4. Other bits belong to separate
/// hardware/UI arming controls and must survive a software Smartgain selection.
fn start_smartgain_control(
    mask: u8,
    mut control: impl FnMut(u8, u8, Option<u32>) -> Result<u32>,
) -> Result<()> {
    if mask == 0 || mask > 3 {
        return Err("Select at least one Smartgain input".into());
    }
    match control(5, 0, None)? {
        2 | 3 => return Err("Smartgain is already starting or listening".into()),
        0 | 1 | 4 | 5 => {}
        _ => return Err("Unexpected Smartgain state".into()),
    }
    let result: Result<()> = (|| {
        control(5, 0, Some(1))?;
        let flags = [control(4, 0, None)?, control(4, 1, None)?];
        for channel in 0..2 {
            let selected = ((mask >> channel) & 1) as u32;
            control(4, channel, Some((flags[channel as usize] & !1) | selected))?;
        }
        control(5, 0, Some(2))?;
        Ok(())
    })();
    if let Err(error) = result {
        return Err(match control(5, 0, Some(0)) {
            Ok(_) => format!("{error}; Smartgain setup cancelled"),
            Err(cancel) => format!("{error}; Smartgain cancellation not confirmed: {cancel}"),
        });
    }
    Ok(())
}

/// Output writes are not atomic. A failed request may already have reached the
/// device, so restore both original channels after any write/readback failure.
/// The small transport seam also permits fault tests without loading a driver.
fn set_stereo_output(
    target: i16,
    mut control: impl FnMut(u8, Option<i16>) -> Result<i16>,
) -> Result<()> {
    let originals = [control(1, None)?, control(2, None)?];
    let applied = (|| {
        for channel in 1..=2 {
            control(channel, Some(target))?;
        }
        for channel in 1..=2 {
            if control(channel, None)? != target {
                return Err(format!("Output channel {channel} readback did not match"));
            }
        }
        Ok(())
    })();
    if let Err(error) = applied {
        let mut failures = Vec::new();
        for (index, original) in originals.iter().enumerate() {
            if let Err(error) = control(index as u8 + 1, Some(*original)) {
                failures.push(format!("channel {} restore: {error}", index + 1));
            }
        }
        for (index, original) in originals.iter().enumerate() {
            match control(index as u8 + 1, None) {
                Ok(actual) if actual == *original => {}
                Ok(_) => failures.push(format!("channel {} restore readback mismatch", index + 1)),
                Err(error) => {
                    failures.push(format!("channel {} restore readback: {error}", index + 1))
                }
            }
        }
        return Err(if failures.is_empty() {
            format!("{error}; original output levels restored")
        } else {
            format!(
                "{error}; restoration not confirmed: {}",
                failures.join("; ")
            )
        });
    }
    Ok(())
}

fn check_channel(channel: u8) -> Result<()> {
    if channel < 2 {
        Ok(())
    } else {
        Err("Invalid EVO 4 input channel".into())
    }
}
fn control_bool(bytes: [u8; 4]) -> Result<bool> {
    match u32::from_le_bytes(bytes) {
        0 => Ok(false),
        1 => Ok(true),
        value => Err(format!("Unexpected boolean control response: {value:#x}")),
    }
}
pub fn db_raw(db: f32, min: f32, max: f32) -> Result<i16> {
    if !db.is_finite() || !(min..=max).contains(&db) {
        return Err(format!("Level must be between {min} and {max} dB"));
    }
    Ok((db * 256.0).round() as i16)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smartgain_preserves_hardware_arm_bits() {
        let mut flags = [0b1010u32, 0b1101];
        let mut state = 0;
        start_smartgain_control(1, |selector, channel, value| {
            let target = if selector == 5 {
                &mut state
            } else {
                &mut flags[channel as usize]
            };
            if let Some(value) = value {
                *target = value;
            }
            Ok(*target)
        })
        .unwrap();
        assert_eq!(flags, [0b1011, 0b1100]);
        assert_eq!(state, 2);
    }

    #[test]
    fn smartgain_cancels_after_uncertain_setup_selection_or_start() {
        for fail_at in 1..=6 {
            let mut call = 0;
            let mut state = 0;
            let error = start_smartgain_control(3, |selector, _, value| {
                let current = call;
                call += 1;
                if selector == 5
                    && let Some(value) = value
                {
                    state = value;
                }
                if current == fail_at {
                    return Err("uncertain transport".into());
                }
                Ok(if selector == 5 { state } else { 0b1110 })
            })
            .unwrap_err();
            assert_eq!(state, 0);
            assert!(error.contains("uncertain transport"));
            assert!(error.contains("setup cancelled"));
        }
    }

    #[test]
    fn smartgain_refuses_running_state_without_writes() {
        for running in [2, 3] {
            let mut writes = 0;
            assert!(
                start_smartgain_control(3, |_, _, value| {
                    if value.is_some() {
                        writes += 1;
                    }
                    Ok(running)
                })
                .is_err()
            );
            assert_eq!(writes, 0);
        }
    }

    #[test]
    fn smartgain_reports_uncertain_cancellation() {
        let error = start_smartgain_control(3, |_, _, value| match value {
            None => Ok(0),
            Some(_) => Err("device disappeared".into()),
        })
        .unwrap_err();
        assert!(error.contains("cancellation not confirmed"));
    }

    #[test]
    fn stereo_output_reads_both_originals_before_writing_and_confirms_both() {
        let mut levels = [-4224, -4096];
        let mut calls = Vec::new();
        set_stereo_output(-4480, |channel, value| {
            calls.push((channel, value));
            let level = &mut levels[channel as usize - 1];
            if let Some(value) = value {
                *level = value;
            }
            Ok(*level)
        })
        .unwrap();
        assert_eq!(levels, [-4480, -4480]);
        assert_eq!(
            calls,
            [
                (1, None),
                (2, None),
                (1, Some(-4480)),
                (2, Some(-4480)),
                (1, None),
                (2, None)
            ]
        );
    }

    #[test]
    fn stereo_output_never_writes_without_both_originals() {
        for failed_channel in 1..=2 {
            let mut writes = 0;
            assert!(
                set_stereo_output(-4480, |channel, value| {
                    if value.is_some() {
                        writes += 1;
                    }
                    if channel == failed_channel {
                        Err("read unavailable".into())
                    } else {
                        Ok(-4224)
                    }
                })
                .is_err()
            );
            assert_eq!(writes, 0);
        }
    }

    #[test]
    fn stereo_output_restores_both_after_uncertain_write_on_either_channel() {
        for failed_channel in 1..=2 {
            let original = [-4224, -4096];
            let mut levels = original;
            let mut failed = false;
            let error = set_stereo_output(-4480, |channel, value| {
                let level = &mut levels[channel as usize - 1];
                if let Some(value) = value {
                    *level = value; // USB write can reach hardware before its error is reported.
                    if channel == failed_channel && !failed {
                        failed = true;
                        return Err("uncertain write".into());
                    }
                }
                Ok(*level)
            })
            .unwrap_err();
            assert!(error.contains("uncertain write"));
            assert!(error.contains("original output levels restored"));
            assert_eq!(levels, original);
        }
    }

    #[test]
    fn stereo_output_restores_after_failed_or_mismatched_readback_on_either_channel() {
        for failed_channel in 1..=2 {
            for mismatch in [false, true] {
                let original = [-4224, -4096];
                let mut levels = original;
                let mut writes = 0;
                let mut failed = false;
                let error = set_stereo_output(-4480, |channel, value| {
                    let level = &mut levels[channel as usize - 1];
                    if let Some(value) = value {
                        *level = value;
                        writes += 1;
                    } else if writes == 2 && channel == failed_channel && !failed {
                        failed = true;
                        return if mismatch {
                            Ok(-4000)
                        } else {
                            Err("readback unavailable".into())
                        };
                    }
                    Ok(*level)
                })
                .unwrap_err();
                assert!(error.contains("original output levels restored"));
                assert_eq!(levels, original);
            }
        }
    }

    #[test]
    fn stereo_output_attempts_right_restore_even_when_left_restore_fails() {
        let mut levels = [-4224, -4096];
        let mut step = 0;
        let error = set_stereo_output(-4480, |channel, value| {
            step += 1;
            if step == 5 {
                return Err("left restore failed".into());
            }
            let level = &mut levels[channel as usize - 1];
            if let Some(value) = value {
                *level = value;
            }
            if step == 4 {
                return Err("second write uncertain".into());
            }
            Ok(*level)
        })
        .unwrap_err();
        assert_eq!(step, 8); // Both restore writes and both readbacks were attempted.
        assert_eq!(levels, [-4480, -4096]);
        assert!(error.contains("second write uncertain"));
        assert!(error.contains("restoration not confirmed"));
        assert!(error.contains("left restore failed"));
        assert!(error.contains("channel 1 restore readback mismatch"));
    }

    #[test]
    fn stereo_output_reports_restore_readback_failure() {
        let mut levels = [-4224, -4096];
        let mut step = 0;
        let error = set_stereo_output(-4480, |channel, value| {
            step += 1;
            let level = &mut levels[channel as usize - 1];
            if let Some(value) = value {
                *level = value;
            }
            if step == 4 {
                return Err("write uncertain".into());
            }
            if step == 7 {
                return Err("restore readback unavailable".into());
            }
            Ok(*level)
        })
        .unwrap_err();
        assert_eq!(step, 8);
        assert_eq!(levels, [-4224, -4096]);
        assert!(error.contains("restoration not confirmed"));
        assert!(error.contains("restore readback unavailable"));
    }

    #[test]
    fn volume_encoding_and_bounds() {
        assert_eq!(db_raw(-16.5, -96.0, 0.0).unwrap(), -4224);
        assert_eq!(db_raw(-17.5, -96.0, 0.0).unwrap(), -4480);
        assert!(db_raw(f32::NAN, -96.0, 0.0).is_err());
        assert!(db_raw(1.0, -96.0, 0.0).is_err());
        assert!(db_raw(-97.0, -96.0, 0.0).is_err());
    }
    #[test]
    fn unknown_boolean_is_not_presented_as_enabled() {
        assert_eq!(control_bool([0, 0, 0, 0]).unwrap(), false);
        assert_eq!(control_bool([1, 0, 0, 0]).unwrap(), true);
        assert!(control_bool([255; 4]).is_err());
    }
}
