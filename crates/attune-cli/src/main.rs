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
    /// Transcribe a WAV file with the local Whisper backend and print
    /// the raw segments AND the hallucination-filter drops. Used to
    /// investigate empty/short transcripts from the app.
    Transcribe(TranscribeArgs),
}

#[derive(Parser, Debug)]
struct TranscribeArgs {
    /// Path to the WAV file to transcribe.
    audio: PathBuf,

    /// Path to the GGML Whisper model. Defaults to the app's installed
    /// location at ~/Library/Application Support/Attune/models/ggml-large-v3.bin.
    #[arg(long)]
    model: Option<PathBuf>,

    /// Language hint (ISO 639-1). Pass `auto` or omit for auto-detect.
    #[arg(long)]
    language: Option<String>,

    /// Show the segments WITHOUT applying the hallucination filter.
    /// Reveals what Whisper actually produced versus what got dropped.
    #[arg(long, default_value_t = false)]
    raw: bool,

    /// Override no_speech_thold (default 0.8, whisper.cpp default 0.6,
    /// lower = more permissive on quiet/music chunks).
    #[arg(long)]
    no_speech_thold: Option<f32>,

    /// Switch to greedy sampling (default in this CLI is BeamSearch{5}).
    /// Music sometimes transcribes better with greedy because beam search
    /// converges on the most-memorised lyric ("Altyazı M.K.").
    #[arg(long, default_value_t = false)]
    greedy: bool,

    /// Allow non-speech tokens through. By default Attune strips them,
    /// but for music we may want to see if Whisper emits "[Music]"
    /// instead of an "Altyazı M.K." hallucination.
    #[arg(long, default_value_t = false)]
    allow_non_speech_tokens: bool,

    /// Override entropy threshold (default 2.4). Lower = more permissive
    /// (lets repetitive output through). For music, sometimes raising
    /// this helps.
    #[arg(long)]
    entropy_thold: Option<f32>,

    /// Override logprob threshold (default -1.0). Lower = more permissive.
    #[arg(long)]
    logprob_thold: Option<f32>,

    /// Drop the Attune initial-prompt glossary for this run. Useful
    /// when you suspect the glossary is biasing the output (it should
    /// not according to the OpenAI cookbook, but we want to verify).
    #[arg(long, default_value_t = false)]
    no_initial_prompt: bool,

    /// Run through the real `LocalWhisperTranscriber` library path
    /// instead of the CLI's own params block. Use this to verify the
    /// shipping configuration on a specific audio file. All the per-
    /// param overrides above are ignored when `--library` is set.
    #[arg(long, default_value_t = false)]
    library: bool,
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

    /// Override the on-disk sample rate. Default: native per source (device's
    /// reported rate for the mic, 48 kHz for ScreenCaptureKit). Pass 16000
    /// to write Whisper-ready files instead.
    #[arg(long)]
    sample_rate: Option<u32>,
}

fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        Command::Record(args) => run_record(args),
        Command::Devices => run_devices(),
        Command::Transcribe(args) => run_transcribe(args),
    }
}

fn run_transcribe(args: TranscribeArgs) -> Result<()> {
    use attune_core::transcription::hallucination_filter::filter_segments;
    use attune_core::transcription::{LocalWhisperTranscriber, Transcriber, TranscriptSegment};
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    let model_path = args.model.clone().unwrap_or_else(default_model_path);
    if !model_path.is_file() {
        anyhow::bail!(
            "whisper model not found at {} — download it from the app's Settings panel first",
            model_path.display()
        );
    }
    if !args.audio.is_file() {
        anyhow::bail!("audio file not found at {}", args.audio.display());
    }

    if args.library {
        println!("Model:    {}", model_path.display());
        println!("Audio:    {}", args.audio.display());
        println!("Mode:     LIBRARY (real shipping config)");
        println!();
        let transcriber = LocalWhisperTranscriber::new(&model_path);
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
        return Ok(());
    }

    let no_speech_thold = args.no_speech_thold.unwrap_or(0.8);
    let entropy_thold = args.entropy_thold.unwrap_or(2.4);
    let logprob_thold = args.logprob_thold.unwrap_or(-1.0);

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
            .ok_or_else(|| anyhow::anyhow!("non-UTF8 model path"))?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| anyhow::anyhow!("could not load whisper model: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow::anyhow!("whisper state init: {e}"))?;

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
        .map_err(|e| anyhow::anyhow!("whisper full(): {e}"))?;

    let n = state
        .full_n_segments()
        .map_err(|e| anyhow::anyhow!("whisper segments: {e}"))?;
    let mut raw_segments = Vec::with_capacity(n as usize);
    for i in 0..n {
        let text = state
            .full_get_segment_text(i)
            .map_err(|e| anyhow::anyhow!("segment text: {e}"))?;
        let t0 = state
            .full_get_segment_t0(i)
            .map_err(|e| anyhow::anyhow!("segment t0: {e}"))?;
        let t1 = state
            .full_get_segment_t1(i)
            .map_err(|e| anyhow::anyhow!("segment t1: {e}"))?;
        raw_segments.push(TranscriptSegment {
            start_seconds: t0 as f64 / 100.0,
            end_seconds: t1 as f64 / 100.0,
            text: text.trim().to_string(),
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

fn default_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|p| (p.get() / 2).max(1) as i32)
        .unwrap_or(4)
}

fn decode_wav_to_16k_mono(path: &std::path::Path) -> Result<Vec<f32>> {
    use std::process::Command;
    use std::time::SystemTime;
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
            path.to_str()
                .ok_or_else(|| anyhow::anyhow!("non-UTF8 path"))?,
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
        anyhow::bail!("ffmpeg failed with status {:?}", status);
    }
    let mut reader = hound::WavReader::open(&tmp)?;
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
