use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Result};
use folio_core::audio::voice_processing_capture::{VoiceProcessingCapture, VPIO_SAMPLE_RATE_HZ};

use crate::cli::VpioSmokeArgs;

pub fn run(args: VpioSmokeArgs) -> Result<()> {
    let output = args.output.unwrap_or_else(default_output_path);

    println!("VoiceProcessingIO smoke test");
    println!("  duration: {} s", args.seconds);
    println!("  rate:     {} Hz", VPIO_SAMPLE_RATE_HZ);
    println!("  output:   {}", output.display());
    println!();
    println!("Talk into the mic now. Play something through the speakers");
    println!("for the bleed-cancellation test (e.g. a YouTube video).");
    println!();

    let mut capture = VoiceProcessingCapture::new()?;
    capture.start()?;

    for remaining in (1..=args.seconds).rev() {
        print!("\r  recording… {remaining:>3}s ");
        std::io::stdout().flush().ok();
        std::thread::sleep(Duration::from_secs(1));
    }
    println!("\r  recording… done   ");

    let actual_rate = capture.sample_rate() as u32;
    let samples = capture.stop()?;
    let duration_s = samples.len() as f64 / actual_rate as f64;
    println!();
    println!(
        "Captured {} samples ({:.2} s at {} Hz)",
        samples.len(),
        duration_s,
        actual_rate
    );

    write_wav(&output, &samples, actual_rate)?;
    let bytes = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
    println!("Wrote {} ({} bytes)", output.display(), bytes);
    println!();
    println!("Verify with:");
    println!("  afinfo {}", output.display());
    println!(
        "  open {}    # opens in QuickTime to listen",
        output.display()
    );
    Ok(())
}

fn write_wav(path: &std::path::Path, samples: &[f32], sample_rate: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| anyhow!("could not create WAV at {}: {e}", path.display()))?;
    for s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let sample = (clamped * i16::MAX as f32) as i16;
        writer
            .write_sample(sample)
            .map_err(|e| anyhow!("WAV write: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| anyhow!("WAV finalize: {e}"))?;
    Ok(())
}

fn default_output_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("vpio-smoke-{nanos}.wav"))
}
