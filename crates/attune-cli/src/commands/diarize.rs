//! `diarize` — run speaker diarization over a recording and print
//! "who spoke when".
//!
//! Wraps `attune_core::diarization::DiarizationRuntime` (sherpa-onnx:
//! pyannote segmentation + WeSpeaker embedding + clustering). Models
//! default to the app model store; pass `--segmentation` / `--embedding`
//! to point at specific ONNX files.
//!
//! ```text
//! attune-cli diarize ~/Documents/Attune/Recordings/<session> \
//!   --segmentation /path/to/model.onnx \
//!   --embedding   /path/to/wespeaker_en_voxceleb_resnet34_LM.onnx
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};

use attune_core::diarization::{
    DiarizationModel, DiarizationModelStore, DiarizationOptions, DiarizationRuntime,
};

use crate::cli::DiarizeArgs;

pub fn run(args: DiarizeArgs) -> Result<()> {
    let wav = resolve_wav(&args.input, &args.channel)?;

    let store = DiarizationModelStore::default_location();
    let segmentation = args
        .segmentation
        .clone()
        .unwrap_or_else(|| store.path_for(DiarizationModel::Segmentation));
    let embedding = args
        .embedding
        .clone()
        .unwrap_or_else(|| store.path_for(DiarizationModel::EmbeddingResnet34Lm));
    for (label, p) in [("segmentation", &segmentation), ("embedding", &embedding)] {
        if !p.is_file() {
            bail!(
                "{label} model not found: {}\n\
                 Pass --{label} <path.onnx> or download the models into {}",
                p.display(),
                store.root().display()
            );
        }
    }

    let opts = DiarizationOptions {
        num_speakers: args.num_speakers,
        threshold: args.threshold,
        ..Default::default()
    };

    let rt = DiarizationRuntime::open(&segmentation, &embedding, &opts)
        .context("creating the diarizer (check the model files)")?;

    let t0 = Instant::now();
    let segments = rt.diarize_wav(&wav).context("diarizing the recording")?;
    let elapsed = t0.elapsed().as_secs_f64();

    if args.json {
        for s in &segments {
            println!(
                "{{\"start\":{:.3},\"end\":{:.3},\"speaker\":{}}}",
                s.start_secs, s.end_secs, s.speaker
            );
        }
        return Ok(());
    }

    let audio_secs = segments.iter().map(|s| s.end_secs).fold(0.0_f32, f32::max);
    let mut per_speaker: BTreeMap<i32, (usize, f32)> = BTreeMap::new();
    for s in &segments {
        let e = per_speaker.entry(s.speaker).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += s.end_secs - s.start_secs;
    }

    println!("input        {}", wav.display());
    println!("model rate   {} Hz", rt.sample_rate());
    println!(
        "processed    {:.1}s audio in {:.1}s  (RTF {:.3})",
        audio_secs,
        elapsed,
        if audio_secs > 0.0 {
            elapsed / audio_secs as f64
        } else {
            0.0
        }
    );
    println!(
        "result       {} speakers, {} segments",
        per_speaker.len(),
        segments.len()
    );
    println!();
    println!("{:<16} {:<8} duration", "time", "speaker");
    println!("{}", "-".repeat(40));
    for s in &segments {
        println!(
            "{:>6.2}-{:<7.2}  S{:<5}  {:.2}s",
            s.start_secs,
            s.end_secs,
            s.speaker,
            s.end_secs - s.start_secs
        );
    }
    println!();
    println!("per speaker:");
    for (spk, (count, dur)) in &per_speaker {
        println!("  S{spk}: {count} segments, {dur:.1}s total");
    }

    Ok(())
}

fn resolve_wav(input: &Path, channel: &str) -> Result<PathBuf> {
    if input.is_dir() {
        let candidate = input.join(format!("{channel}.wav"));
        if !candidate.is_file() {
            bail!("no {channel}.wav in {}", input.display());
        }
        Ok(candidate)
    } else if input.is_file() {
        Ok(input.to_path_buf())
    } else {
        bail!("input not found: {}", input.display())
    }
}
