//! `clap` argument definitions for `attune-cli`.
//!
//! All argument structs and the top-level [`Command`] enum live here so
//! `main.rs` stays a thin dispatch table. Per-subcommand business logic
//! lives next to the subcommand in `crate::commands::*`.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "attune-cli")]
#[command(version)]
#[command(about = "Attune CLI test harness", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Record audio for a fixed duration and write WAV files to disk.
    Record(RecordArgs),
    /// List available input audio devices.
    Devices,
    /// Transcribe a WAV file with the local Whisper backend and print
    /// the raw segments AND the hallucination-filter drops. Used to
    /// investigate empty/short transcripts from the app.
    Transcribe(TranscribeArgs),
    /// macOS only. Record from the default mic through Apple's Voice
    /// Processing IO AudioUnit (AEC + noise suppression + AGC) for a
    /// fixed duration. Writes the captured audio to a WAV so you can
    /// listen to it and compare against a non-VPIO recording.
    ///
    /// Phase 1 smoke test for projects/attune/plan/voice-processing-io.md
    /// in the vault. The production path is wired into `CaptureSession`
    /// behind the `voice_processing_enabled` setting.
    #[cfg(target_os = "macos")]
    VpioSmoke(VpioSmokeArgs),
    /// List recording sessions under a directory. JSON by default so
    /// the output pipes cleanly into jq / Hammerspoon / RTK.
    /// v2 finding 072 / GET-74.
    Sessions(SessionsArgs),
    /// List tasks from a tasks.json file. JSON by default.
    /// v2 finding 072 / GET-74.
    Tasks(TasksArgs),
    /// Search the local memory store. JSON by default.
    /// v2 finding 072 / GET-74.
    MemorySearch(MemorySearchArgs),
    /// GET-188. Enhance a recording's system audio (RNNoise) and report
    /// before/after metrics, for A/B evaluation. Writes
    /// `<stem>.enhanced.wav` next to the input so you can listen to both
    /// and transcribe each with `attune-cli transcribe`.
    EnhanceCompare(EnhanceCompareArgs),
    /// Speaker diarization (GET-189 substrate). Runs the sherpa-onnx
    /// pyannote-segmentation + WeSpeaker-embedding + clustering pipeline
    /// over a recording and prints speaker-labelled time segments
    /// ("who spoke when").
    Diarize(DiarizeArgs),
    /// GET-189. Diarize an EXISTING recording's transcript in place:
    /// loads its transcript, tags each system-channel segment with a
    /// speaker, and rewrites it. Re-open the note to see Speaker 1/2/3…
    /// (no Whisper re-run).
    DiarizeTranscript(DiarizeTranscriptArgs),
}

#[derive(Parser, Debug)]
pub struct DiarizeTranscriptArgs {
    /// Session directory containing `transcript.json(.zst)` + `system.wav`.
    pub session_dir: PathBuf,

    /// Pyannote segmentation ONNX model. Defaults to the app model store.
    #[arg(long)]
    pub segmentation: Option<PathBuf>,

    /// WeSpeaker embedding ONNX model. Defaults to the app model store.
    #[arg(long)]
    pub embedding: Option<PathBuf>,

    /// Fixed speaker count. 0 (default) auto-estimates.
    #[arg(long, default_value_t = 0)]
    pub num_speakers: i32,

    /// Clustering threshold when auto-estimating.
    #[arg(long, default_value_t = 0.5, allow_hyphen_values = true)]
    pub threshold: f32,
}

#[derive(Parser, Debug)]
pub struct DiarizeArgs {
    /// A WAV file, or a session directory (uses `<channel>.wav`).
    pub input: PathBuf,

    /// Which channel to diarize when `input` is a directory.
    #[arg(long, default_value = "system")]
    pub channel: String,

    /// Pyannote segmentation ONNX model. Defaults to the app model
    /// store (`~/Library/Application Support/Attune/models/diarization/`).
    #[arg(long)]
    pub segmentation: Option<PathBuf>,

    /// WeSpeaker embedding ONNX model. Defaults to the app model store.
    #[arg(long)]
    pub embedding: Option<PathBuf>,

    /// Fixed speaker count. 0 (default) auto-estimates via `--threshold`.
    #[arg(long, default_value_t = 0)]
    pub num_speakers: i32,

    /// Clustering merge threshold when auto-estimating (higher = fewer
    /// speakers). sherpa-onnx default ~0.5.
    #[arg(long, default_value_t = 0.5, allow_hyphen_values = true)]
    pub threshold: f32,

    /// Print newline-delimited JSON segments instead of a table.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct EnhanceCompareArgs {
    /// Path to a `system.wav` (or any WAV), or a session directory
    /// containing `system.wav`.
    pub input: PathBuf,

    /// Output path for the enhanced WAV. Defaults to
    /// `<stem>.enhanced.wav` next to the input.
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Maximum attenuation in dB (negative; more negative = more
    /// aggressive suppression). Defaults to the app's conservative
    /// -20 dB cap.
    #[arg(long, default_value_t = -20.0, allow_hyphen_values = true)]
    pub atten_lim_db: f32,
}

#[derive(Parser, Debug)]
pub struct SessionsArgs {
    /// Recordings directory to scan. Defaults to ./recordings to match
    /// the `record` subcommand's default.
    #[arg(long, default_value = "./recordings")]
    pub output: PathBuf,

    /// Print a table to stdout instead of newline-delimited JSON.
    #[arg(long, default_value_t = false)]
    pub table: bool,

    /// Limit to N most-recent sessions. 0 = no limit.
    #[arg(long, default_value_t = 0)]
    pub limit: usize,
}

#[derive(Parser, Debug)]
pub struct TasksArgs {
    /// Path to the tasks.json file. Defaults to ./tasks/tasks.json.
    #[arg(long, default_value = "./tasks/tasks.json")]
    pub path: PathBuf,

    /// Filter to only this status. Empty = all.
    #[arg(long)]
    pub status: Option<String>,

    /// Print a one-line-per-task table instead of JSON.
    #[arg(long, default_value_t = false)]
    pub table: bool,
}

#[derive(Parser, Debug)]
pub struct MemorySearchArgs {
    /// Memory directory to scan. Defaults to ./memory.
    #[arg(long, default_value = "./memory")]
    pub dir: PathBuf,

    /// Free-text query. Matches against the memory's content + key,
    /// case-insensitive substring.
    pub query: String,

    /// Restrict by memory kind (observe / claim / pref / person).
    #[arg(long)]
    pub kind: Option<String>,

    /// Max rows returned. 0 = no limit.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    /// Print a table instead of JSON.
    #[arg(long, default_value_t = false)]
    pub table: bool,
}

#[cfg(target_os = "macos")]
#[derive(Parser, Debug)]
pub struct VpioSmokeArgs {
    /// Recording duration in seconds.
    #[arg(long, default_value_t = 5)]
    pub seconds: u64,

    /// Output WAV path. Defaults to a unique timestamped file under
    /// /tmp so successive runs don't clobber each other.
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct TranscribeArgs {
    /// Path to the WAV file to transcribe.
    pub audio: PathBuf,

    /// Path to the GGML Whisper model. Defaults to the app's installed
    /// location at ~/Library/Application Support/Attune/models/ggml-large-v3.bin.
    #[arg(long)]
    pub model: Option<PathBuf>,

    /// Language hint (ISO 639-1). Pass `auto` or omit for auto-detect.
    #[arg(long)]
    pub language: Option<String>,

    /// Show the segments WITHOUT applying the hallucination filter.
    /// Reveals what Whisper actually produced versus what got dropped.
    #[arg(long, default_value_t = false)]
    pub raw: bool,

    /// Override no_speech_thold (default 0.8, whisper.cpp default 0.6,
    /// lower = more permissive on quiet/music chunks).
    #[arg(long)]
    pub no_speech_thold: Option<f32>,

    /// Switch to greedy sampling (default in this CLI is BeamSearch{5}).
    /// Music sometimes transcribes better with greedy because beam search
    /// converges on the most-memorised lyric ("Altyazı M.K.").
    #[arg(long, default_value_t = false)]
    pub greedy: bool,

    /// Allow non-speech tokens through. By default Attune strips them,
    /// but for music we may want to see if Whisper emits "[Music]"
    /// instead of an "Altyazı M.K." hallucination.
    #[arg(long, default_value_t = false)]
    pub allow_non_speech_tokens: bool,

    /// Override entropy threshold (default 2.4). Lower = more permissive
    /// (lets repetitive output through). For music, sometimes raising
    /// this helps.
    #[arg(long)]
    pub entropy_thold: Option<f32>,

    /// Override logprob threshold (default -1.0). Lower = more permissive.
    #[arg(long)]
    pub logprob_thold: Option<f32>,

    /// Drop the Attune initial-prompt glossary for this run. Useful
    /// when you suspect the glossary is biasing the output (it should
    /// not according to the OpenAI cookbook, but we want to verify).
    #[arg(long, default_value_t = false)]
    pub no_initial_prompt: bool,

    /// Run through the real `LocalWhisperTranscriber` library path
    /// instead of the CLI's own params block. Use this to verify the
    /// shipping configuration on a specific audio file. All the per-
    /// param overrides above are ignored when `--library` is set.
    #[arg(long, default_value_t = false)]
    pub library: bool,
}

#[derive(Parser, Debug)]
pub struct RecordArgs {
    /// Recording duration in seconds.
    #[arg(long, default_value_t = 10)]
    pub seconds: u64,

    /// Output directory. A timestamped subdirectory is created within.
    #[arg(long, default_value = "./recordings")]
    pub output: PathBuf,

    /// Disable microphone capture.
    #[arg(long, default_value_t = false)]
    pub no_mic: bool,

    /// Disable system audio capture.
    #[arg(long, default_value_t = false)]
    pub no_system: bool,

    /// Microphone device by exact name. Use `attune-cli devices` to list.
    #[arg(long)]
    pub mic_device: Option<String>,

    /// Override the on-disk sample rate. Default: native per source
    /// (device's reported rate for the mic, 48 kHz for ScreenCaptureKit).
    /// Pass 16000 to write Whisper-ready files instead.
    #[arg(long)]
    pub sample_rate: Option<u32>,

    /// macOS only. Disable Apple Voice Processing IO on the mic path
    /// (AEC + noise suppression + AGC). Default behaviour is VPIO ON;
    /// this flag forces the plain cpal path for A/B testing.
    #[arg(long, default_value_t = false)]
    pub no_voice_processing: bool,
}
