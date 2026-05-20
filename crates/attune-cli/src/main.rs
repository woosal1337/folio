//! Attune CLI. Test harness for the `attune-core` library during development.
//!
//! Subcommands:
//!   * `record`  — capture mic + system audio to WAV files for a fixed duration.
//!   * `devices` — list available input devices.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use attune_core::audio::{list_input_devices, CaptureConfig, CaptureSession};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "attune-cli")]
#[command(version)]
#[command(about = "Attune CLI test harness", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Record audio for a fixed duration and write WAV files to disk.
    Record(RecordArgs),
    /// List available input audio devices.
    Devices,
}

#[derive(Parser, Debug)]
struct RecordArgs {
    /// Recording duration in seconds.
    #[arg(long, default_value_t = 10)]
    seconds: u64,

    /// Output directory. A timestamped subdirectory is created within.
    #[arg(long, default_value = "./recordings")]
    output: PathBuf,

    /// Disable microphone capture.
    #[arg(long, default_value_t = false)]
    no_mic: bool,

    /// Disable system audio capture.
    #[arg(long, default_value_t = false)]
    no_system: bool,

    /// Microphone device by exact name. Use `attune-cli devices` to list.
    #[arg(long)]
    mic_device: Option<String>,

    /// Target sample rate for the output WAV files.
    #[arg(long, default_value_t = 16_000)]
    sample_rate: u32,
}

fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        Command::Record(args) => run_record(args),
        Command::Devices => run_devices(),
    }
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cpal=warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn run_devices() -> Result<()> {
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

fn run_record(args: RecordArgs) -> Result<()> {
    let config = CaptureConfig {
        mic_enabled: !args.no_mic,
        system_enabled: !args.no_system,
        mic_device_name: args.mic_device,
        target_sample_rate: args.sample_rate,
        output_dir: args.output,
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
