#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod controller;
mod driver;
mod model;
mod meters;
mod motion;
mod routing;
mod ui_bridge;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--diagnostics") {
        let destination = args.get(2).ok_or("Provide a diagnostics output path")?;
        let (output, success) = match controller::diagnostic() {
            Ok(json) => (json, true),
            Err(error) => (
                serde_json::json!({"connected":false,"error":error}).to_string(),
                false,
            ),
        };
        std::fs::write(destination, output)?;
        if !success {
            std::process::exit(2);
        }
        return Ok(());
    }
    let (commands_tx, commands_rx) = std::sync::mpsc::sync_channel(64);
    let (snapshots_tx, snapshots_rx) = std::sync::mpsc::sync_channel(2);
    std::thread::Builder::new()
        .name("evo-device".into())
        .spawn(move || controller::run(commands_rx, snapshots_tx))?;
    ui_bridge::run(commands_tx, snapshots_rx)?;
    Ok(())
}
