//! `attune-cli devices` — enumerate input audio devices.

use anyhow::Result;
use attune_core::audio::list_input_devices;

pub fn run() -> Result<()> {
    let devices = list_input_devices()?;
    if devices.is_empty() {
        println!("No input devices found.");
        return Ok(());
    }
    println!("Input devices:");
    for d in devices {
        let marker = if d.is_default { "*" } else { " " };
        let sr = d
            .default_sample_rate
            .map(|s| format!("{} Hz", s))
            .unwrap_or_else(|| "unknown".into());
        let ch = d
            .default_channels
            .map(|c| format!("{} ch", c))
            .unwrap_or_else(|| "unknown".into());
        println!("  {} {:40}  {:10}  {}", marker, d.name, sr, ch);
    }
    println!();
    println!("* = default. Pass --mic-device \"<name>\" to record from a specific device.");
    Ok(())
}
