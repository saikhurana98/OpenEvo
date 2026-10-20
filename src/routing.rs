//! Loopback intentions. Hardware mixer registers cannot be read back.
use crate::driver::Result;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Route {
    pub gain_db: f32,
    pub pan: f32,
    pub muted: bool,
}
impl Default for Route {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            pan: 0.0,
            muted: false,
        }
    }
}

/// Six mono sources map to MU60's six inputs, each with two destinations.
/// OEM MixerNode FUN_1400d76f0 uses this quadratic -6 dB center pan law.
pub fn matrix(source: u8, route: Route) -> Result<Vec<(u8, i16)>> {
    if source > 5
        || !route.gain_db.is_finite()
        || !(-96.0..=6.0).contains(&route.gain_db)
        || !route.pan.is_finite()
        || !(-100.0..=100.0).contains(&route.pan)
    {
        return Err("Loopback level or pan is outside its range".into());
    }
    let coefficient = |linear: f64| -> i16 {
        if route.muted {
            i16::MIN
        } else {
            let pan_db = if linear <= 0.0 {
                -128.0
            } else {
                (20.0 * linear.log10()).max(-128.0)
            };
            // The OEM clamps in dB then truncates Q8.8 toward zero.
            ((route.gain_db as f64 + pan_db).clamp(-128.0, 6.0) * 256.0) as i16
        }
    };
    let pan = ((route.pan + 100.0) / 200.0) as f64;
    // OEM forms k in single precision, then promotes for the polynomial/log.
    let k = (2.0 - 4.0 * 10.0_f32.powf(-0.3)) as f64;
    let amplitude = |x: f64| ((k * x + 1.0) - k) * x;
    Ok(vec![
        (source * 2, coefficient(amplitude(1.0 - pan))),
        (source * 2 + 1, coefficient(amplitude(pan))),
    ])
}

#[derive(Default, Serialize, Deserialize)]
pub struct SavedRouting {
    pub schema: u8,
    pub sources: [Option<Route>; 6],
}
pub fn load() -> SavedRouting {
    path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<SavedRouting>(&b).ok())
        .filter(|s| {
            s.schema == 2
                && s.sources
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.is_none_or(|r| matrix(i as u8, r).is_ok()))
        })
        .unwrap_or(SavedRouting {
            schema: 2,
            ..SavedRouting::default()
        })
}
pub fn save(state: &SavedRouting) -> Result<()> {
    let path = path().ok_or("Local application data directory is unavailable")?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let pending = path.with_extension("json.pending");
    std::fs::write(
        &pending,
        serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // std::fs::rename cannot replace an existing file atomically on every
    // Windows version. MoveFileExW replaces the old name at commit.
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileExW(old: *const u16, new: *const u16, flags: u32) -> i32;
        }
        let old: Vec<u16> = pending.as_os_str().encode_wide().chain(Some(0)).collect();
        let new: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(old.as_ptr(), new.as_ptr(), 0x9) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    std::fs::rename(pending, path).map_err(|e| e.to_string())?;
    Ok(())
}
fn path() -> Option<std::path::PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("EvoControl/loopback.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oem_pan_law_and_source_mapping() {
        let left = matrix(
            0,
            Route {
                pan: -100.0,
                ..Route::default()
            },
        )
        .unwrap();
        assert_eq!(left, vec![(0, 0), (1, i16::MIN)]);
        let center = matrix(1, Route::default()).unwrap();
        assert_eq!(center[0].1, -1536);
        assert_eq!(center[0].1, center[1].1);
        assert_eq!(
            matrix(
                5,
                Route {
                    pan: 100.0,
                    ..Route::default()
                }
            )
            .unwrap(),
            vec![(10, i16::MIN), (11, 0)]
        );
        // At quarter pan OEM's polynomial is approximately -2.488/-12.010 dB.
        let quarter = matrix(
            2,
            Route {
                pan: -50.0,
                ..Route::default()
            },
        )
        .unwrap();
        assert!((quarter[0].1 as f32 / 256.0 + 2.488).abs() < 0.01);
        assert!((quarter[1].1 as f32 / 256.0 + 12.010).abs() < 0.01);
        for source in 0..6 {
            assert_eq!(
                matrix(source, Route::default()).unwrap(),
                vec![(source * 2, -1536), (source * 2 + 1, -1536)]
            );
        }
        assert_eq!(
            matrix(
                0,
                Route {
                    gain_db: 6.0,
                    pan: -100.0,
                    muted: false
                }
            )
            .unwrap(),
            vec![(0, 1536), (1, -31232)]
        );
    }
    #[test]
    fn mute_is_true_silence_and_values_are_bounded() {
        assert!(
            matrix(
                3,
                Route {
                    muted: true,
                    ..Route::default()
                }
            )
            .unwrap()
            .iter()
            .all(|(_, v)| *v == i16::MIN)
        );
        assert!(matrix(6, Route::default()).is_err());
        assert!(
            matrix(
                0,
                Route {
                    gain_db: f32::NAN,
                    ..Route::default()
                }
            )
            .is_err()
        );
    }
}
