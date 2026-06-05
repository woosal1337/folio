use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use attune_core::audio::enhancement::{enhance_wav_file, EnhancementConfig};

use crate::cli::EnhanceCompareArgs;

pub fn run(args: EnhanceCompareArgs) -> Result<()> {
    let input = resolve_input(&args.input)?;
    let out = args.out.clone().unwrap_or_else(|| default_out(&input));
    let cfg = EnhancementConfig {
        atten_lim_db: args.atten_lim_db,
    };

    let stats = enhance_wav_file(&input, &out, &cfg)
        .with_context(|| format!("enhancing {}", input.display()))?;

    let dbfs = |rms: f32| {
        if rms > 0.0 {
            20.0 * rms.log10()
        } else {
            f32::NEG_INFINITY
        }
    };
    let level_delta = dbfs(stats.output_rms) - dbfs(stats.input_rms);

    println!("input        {}", input.display());
    println!("output       {}", out.display());
    println!("audio        {:.1}s", stats.audio_secs);
    println!(
        "processing   {:.2}s   (RTF {:.3}, lower is faster than real-time)",
        stats.processing_secs,
        stats.rtf()
    );
    println!(
        "input level  {:.6} RMS  ({:.1} dBFS)",
        stats.input_rms,
        dbfs(stats.input_rms)
    );
    println!(
        "output level {:.6} RMS  ({:.1} dBFS)",
        stats.output_rms,
        dbfs(stats.output_rms)
    );
    println!(
        "Δ level      {level_delta:+.1} dB   (negative = energy removed, i.e. noise suppressed)"
    );
    println!("atten cap    {:.1} dB", cfg.atten_lim_db);
    println!();
    println!("Compare transcripts (raw vs enhanced) to judge the WER impact:");
    println!("  attune-cli transcribe {} --library", input.display());
    println!("  attune-cli transcribe {} --library", out.display());

    Ok(())
}

fn resolve_input(p: &Path) -> Result<PathBuf> {
    if p.is_dir() {
        let candidate = p.join("system.wav");
        if !candidate.is_file() {
            bail!("no system.wav found in {}", p.display());
        }
        Ok(candidate)
    } else if p.is_file() {
        Ok(p.to_path_buf())
    } else {
        bail!("input not found: {}", p.display())
    }
}

fn default_out(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}.enhanced.wav"))
}
