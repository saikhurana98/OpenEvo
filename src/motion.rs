//! Presentation-only preferences. No device API is used here.
pub fn prefers_reduced_motion() -> bool {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn SystemParametersInfoW(
                action: u32,
                parameter: u32,
                value: *mut std::ffi::c_void,
                flags: u32,
            ) -> i32;
        }
        let mut enabled: i32 = 1;
        // SPI_GETCLIENTAREAANIMATION returns a Win32 BOOL, not a Rust bool.
        let read = unsafe {
            SystemParametersInfoW(0x1042, 0, (&mut enabled as *mut i32).cast(), 0)
        };
        read != 0 && enabled == 0
    }
    #[cfg(not(windows))]
    false
}
