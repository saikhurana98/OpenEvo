//! Bounded development-only Smartgain test through the production driver.
#[allow(dead_code)]
#[path = "../src/driver.rs"]
mod driver;
#[allow(dead_code)]
#[path = "../src/routing.rs"]
mod routing;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("Provide an evidence path")?;
    let device = driver::Device::open()?;
    if device.phantom(0)? || device.phantom(1)? {
        return Err("48V must be off on both line inputs".into());
    }
    if !device.output_muted()? && device.output_db()? > -60.0 {
        return Err("Physical output must be muted or below -60 dB".into());
    }
    let original = [device.gain(0)?, device.gain(1)?];
    let mut observations = Vec::new();
    let result = (|| -> driver::Result<()> {
        device.start_smartgain(3)?;
        let started = std::time::Instant::now();
        while started.elapsed().as_secs_f32() < 30.0 {
            let state = device.smartgain_state()?;
            observations.push(serde_json::json!({
                "seconds":started.elapsed().as_secs_f32(), "state":state,
                "gains_db":[device.gain(0)?,device.gain(1)?],
                "input_peaks":device.input_peaks()?,
                "phantom":[device.phantom(0)?,device.phantom(1)?]
            }));
            match state {
                4 => return Ok(()),
                5 => return Err("Firmware reported Smartgain failure".into()),
                _ => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
        Err("Smartgain did not complete within 30 seconds".into())
    })();
    let mut restoration_errors = Vec::new();
    if let Err(error) = device.stop_smartgain() {
        restoration_errors.push(format!("stop: {error}"));
    }
    for channel in 0..2 {
        if let Err(error) = device.set_gain(channel, original[channel as usize]) {
            restoration_errors.push(format!("gain {channel}: {error}"));
        }
    }
    let restored = [device.gain(0), device.gain(1)];
    for channel in 0..2 {
        if restored[channel].as_ref().copied() != Ok(original[channel]) {
            restoration_errors.push(format!("gain {channel} restoration not confirmed"));
        }
    }
    let passed = result.is_ok() && restoration_errors.is_empty();
    let report = serde_json::json!({
        "kind":"Live Smartgain using production Rust driver; DJ console loop",
        "passed":passed,"original_gains_db":original,"observations":observations,
        "error":result.err(),"restoration_errors":restoration_errors,
        "restored_gains_db":restored,"state_after_cleanup":device.smartgain_state()?
    });
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({"passed":passed,"observations":observations.len(),
        "last":observations.last(),"restoration_errors":restoration_errors})
    );
    if passed {
        Ok(())
    } else {
        Err("Smartgain validation did not pass; see evidence".into())
    }
}
