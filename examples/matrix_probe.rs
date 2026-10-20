//! Development-only access to the production mixer setter; not packaged as an app.
#[path = "../src/driver.rs"]
mod driver;
#[path = "../src/routing.rs"]
mod routing;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        return Err("Usage: matrix_probe SOURCE GAIN_DB PAN MUTED(0|1)".into());
    }
    let source: u8 = args[1].parse().map_err(|_| "Invalid source")?;
    let route = routing::Route {
        gain_db: args[2].parse().map_err(|_| "Invalid level")?,
        pan: args[3].parse().map_err(|_| "Invalid pan")?,
        muted: match args[4].as_str() {
            "0" => false,
            "1" => true,
            _ => return Err("Invalid mute".into()),
        },
    };
    let coefficients = routing::matrix(source, route)?;
    let device = driver::Device::open()?;
    device.set_loopback(source, route)?;
    println!(
        "{}",
        serde_json::json!({"source":source,"route":route,"coefficients":coefficients,
        "write_acknowledged":true,"hardware_readback_available":false})
    );
    Ok(())
}
