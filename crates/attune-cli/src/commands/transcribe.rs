//! `attune-cli transcribe` — feed a WAV file through whisper.cpp and
//! print both the raw segments and the hallucination filter's drops.
//! Mirrors the library path closely enough to investigate empty or
//! short transcripts coming out of the Tauri app.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use anyhow::{anyhow, bail, Result};
use attune_core::transcription::hallucination_filter::filter_segments;
use attune_core::transcription::{LocalWhisperTranscriber, Transcriber, TranscriptSegment};
use hound::WavReader;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::cli::TranscribeArgs;

pub fn run(args: TranscribeArgs) -> Result<()> {
    let model_path = args.model.clone().unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        bail!(
            "whisper model not found at {} — download it from the app's Settings panel first",
            model_path.display()
        );
    }
    if !args.audio.is_file() {
        bail!("audio file not found at {}", args.audio.display());
    }

    if args.library {
        return run_library_mode(&model_path, &args);
    }

    let no_speech_thold = args.no_speech_thold.unwrap_or(0.8);
    let entropy_thold = args.entropy_thold.unwrap_or(2.4);
    let logprob_thold = args.logprob_thold.unwrap_or(-1.0);

    print_param_banner(
        &model_path,
        &args,
        no_speech_thold,
        entropy_thold,
        logprob_thold,
    );

    let pcm = decode_wav_to_16k_mono(&args.audio)?;
    println!(
        "decoded {} samples ({:.1}s at 16kHz)",
        pcm.len(),
        pcm.len() as f32 / 16_000.0
    );
    let peak = pcm.iter().fold(0.0f32, |a, b| a.max(b.abs()));
    let rms = (pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len().max(1) as f32).sqrt();
    println!(
        "audio peak amplitude:  {:.4}  ({:.1} dBFS)",
        peak,
        20.0 * peak.max(1e-9).log10()
    );
    println!(
        "audio rms amplitude:   {:.4}  ({:.1} dBFS)",
        rms,
        20.0 * rms.max(1e-9).log10()
    );

    let ctx = WhisperContext::new_with_params(
        model_path
            .to_str()
            .ok_or_else(|| anyhow!("non-UTF8 model path"))?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| anyhow!("could not load whisper model: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow!("whisper state init: {e}"))?;

    let mut params = if args.greedy {
        FullParams::new(SamplingStrategy::Greedy { best_of: 5 })
    } else {
        FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        })
    };
    params.set_n_threads(default_threads());
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_no_context(true);
    params.set_n_max_text_ctx(0);
    params.set_suppress_blank(true);
    params.set_suppress_non_speech_tokens(!args.allow_non_speech_tokens);
    params.set_temperature(0.0);
    params.set_temperature_inc(0.2);
    params.set_entropy_thold(entropy_thold);
    params.set_logprob_thold(logprob_thold);
    params.set_no_speech_thold(no_speech_thold);
    params.set_max_initial_ts(1.0);
    params.set_token_timestamps(true);
    params.set_split_on_word(true);
    params.set_max_len(120);
    if !args.no_initial_prompt {
        params.set_initial_prompt(
            "Attune meeting glossary: Tahir, Yusuf, İbrahim, Ege, Vusal, Azerbaycan, \
             Chrome extension, Claude, Gemini, MIS, veri tabanı, sistemleri, \
             multidisipliner, agent, startup.",
        );
    }

    let hint = args
        .language
        .as_deref()
        .filter(|l| !l.is_empty() && *l != "auto");
    params.set_language(hint);

    println!("running inference…");
    state
        .full(params, &pcm)
        .map_err(|e| anyhow!("whisper full(): {e}"))?;

    let n = state
        .full_n_segments()
        .map_err(|e| anyhow!("whisper segments: {e}"))?;
    let mut raw_segments = Vec::with_capacity(n as usize);
    for i in 0..n {
        let text = state
            .full_get_segment_text(i)
            .map_err(|e| anyhow!("segment text: {e}"))?;
        let t0 = state
            .full_get_segment_t0(i)
            .map_err(|e| anyhow!("segment t0: {e}"))?;
        let t1 = state
            .full_get_segment_t1(i)
            .map_err(|e| anyhow!("segment t1: {e}"))?;
        raw_segments.push(TranscriptSegment {
            start_seconds: t0 as f64 / 100.0,
            end_seconds: t1 as f64 / 100.0,
            text: text.trim().to_string(),
            speaker: None,
        });
    }

    let detected = state.full_lang_id_from_state().ok();
    println!("detected language id: {:?}", detected);
    println!();

    println!("--- raw whisper segments ({}) ---", raw_segments.len());
    for (i, s) in raw_segments.iter().enumerate() {
        println!(
            "  [{:>3}] {:>7.2}s → {:>7.2}s  |{}|",
            i, s.start_seconds, s.end_seconds, s.text
        );
    }

    if args.raw {
        return Ok(());
    }

    let (kept, dropped) = filter_segments(raw_segments);
    println!();
    println!("--- after hallucination filter ---");
    println!("  kept:    {}", kept.len());
    println!("  dropped: {}", dropped.len());
    for d in &dropped {
        println!("    × |{}|", d);
    }
    println!();
    println!("--- final transcript ({} segments) ---", kept.len());
    for (i, s) in kept.iter().enumerate() {
        println!(
            "  [{:>3}] {:>7.2}s → {:>7.2}s  |{}|",
            i, s.start_seconds, s.end_seconds, s.text
        );
    }
    Ok(())
}

fn run_library_mode(model_path: &Path, args: &TranscribeArgs) -> Result<()> {
    println!("Model:    {}", model_path.display());
    println!("Audio:    {}", args.audio.display());
    println!("Mode:     LIBRARY (real shipping config)");
    println!();
    let transcriber = LocalWhisperTranscriber::new(model_path);
    let transcript = transcriber.transcribe(&args.audio, args.language.as_deref())?;
    println!("--- library output ---");
    println!("language: {:?}", transcript.language);
    println!("segments: {}", transcript.segments.len());
    for (i, s) in transcript.segments.iter().enumerate() {
        println!(
            "  [{:>3}] {:>7.2}s → {:>7.2}s  |{}|",
            i, s.start_seconds, s.end_seconds, s.text
        );
    }
    Ok(())
}

fn print_param_banner(
    model_path: &Path,
    args: &TranscribeArgs,
    no_speech_thold: f32,
    entropy_thold: f32,
    logprob_thold: f32,
) {
    println!("Model:                {}", model_path.display());
    println!("Audio:                {}", args.audio.display());
    println!(
        "Language:             {}",
        args.language.as_deref().unwrap_or("auto")
    );
    println!(
        "Sampling:             {}",
        if args.greedy {
            "Greedy{best_of=5}"
        } else {
            "BeamSearch{beam_size=5}"
        }
    );
    println!("no_speech_thold:      {}", no_speech_thold);
    println!("entropy_thold:        {}", entropy_thold);
    println!("logprob_thold:        {}", logprob_thold);
    println!(
        "non-speech tokens:    {}",
        if args.allow_non_speech_tokens {
            "ALLOW"
        } else {
            "suppress"
        }
    );
    println!(
        "initial_prompt:       {}",
        if args.no_initial_prompt {
            "OFF"
        } else {
            "Attune glossary"
        }
    );
    println!(
        "filter:               {}",
        if args.raw { "OFF (raw)" } else { "ON" }
    );
    println!();
}

fn default_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|p| (p.get() / 2).max(1) as i32)
        .unwrap_or(4)
}

/// Re-encode any input WAV to mono 16 kHz PCM via ffmpeg, then read
/// the result as `f32` samples for direct feeding to whisper.cpp.
/// Whisper requires 16 kHz mono Float32; this is the standard
/// preprocessing step.
fn decode_wav_to_16k_mono(path: &Path) -> Result<Vec<f32>> {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let tmp = std::env::temp_dir().join(format!("attune-cli-{}.wav", nanos));
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-i",
            path.to_str().ok_or_else(|| anyhow!("non-UTF8 path"))?,
            "-ac",
            "1",
            "-ar",
            "16000",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&tmp)
        .status()?;
    if !status.success() {
        bail!("ffmpeg failed with status {:?}", status);
    }
    let mut reader = WavReader::open(&tmp)?;
    let bits = reader.spec().bits_per_sample;
    let max = (1i64 << (bits - 1)) as f32;
    let mut out = Vec::with_capacity(reader.len() as usize);
    for s in reader.samples::<i32>() {
        out.push(s? as f32 / max);
    }
    let _ = std::fs::remove_file(&tmp);
    Ok(out)
}

fn default_model_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Attune")
        .join("models")
        .join("ggml-large-v3.bin")
}
