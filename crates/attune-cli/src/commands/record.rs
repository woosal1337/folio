//! `attune-cli record` — capture mic + system audio for a fixed
//! duration and write WAV files to disk.

use std::time::Duration;

use anyhow::Result;
use attune_core::audio::{CaptureConfig, CaptureSession};

use crate::cli::RecordArgs;

pub fn run(args: RecordArgs) -> Result<()> {
    let config = CaptureConfig {
        mic_enabled: !args.no_mic,
        system_enabled: !args.no_system,
        mic_device_name: args.mic_device,
        target_sample_rate: args.sample_rate,
        output_dir: args.output,
        voice_processing_enabled: !args.no_voice_processing,
    };

    tracing::info!(
        mic = config.mic_enabled,
        system = config.system_enabled,
        device = ?config.mic_device_name,
        sample_rate = config.target_sample_rate,
        seconds = args.seconds,
        output = %config.output_dir.display(),
        "starting capture",
    );

    let session = CaptureSession::start(config)?;
    let channels = session.channels_active();
    tracing::info!(?channels, "channels active");
    if channels.is_empty() {
        anyhow::bail!("no capture channels active — both mic and system audio failed to start");
    }

    std::thread::sleep(Duration::from_secs(args.seconds));

    let artifacts = session.stop()?;
    println!();
    println!("Recording complete.");
    println!("  Session dir: {}", artifacts.session_dir.display());
    if let Some(p) = &artifacts.mic_path {
        if p.exists() {
            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            println!("  Mic:         {} ({} bytes)", p.display(), size);
        }
    }
    if let Some(p) = &artifacts.system_path {
        if p.exists() {
            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            println!("  System:      {} ({} bytes)", p.display(), size);
        } else {
            println!("  System:      <not captured — see logs above>");
        }
    }
    println!(
        "  Duration:    {} seconds",
        (artifacts.stopped_at - artifacts.started_at).num_seconds()
    );
    Ok(())
}
