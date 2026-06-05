use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "folio-cli")]
#[command(version)]
#[command(about = "Folio CLI test harness", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Record(RecordArgs),

    Devices,

    Transcribe(TranscribeArgs),

    #[cfg(target_os = "macos")]
    VpioSmoke(VpioSmokeArgs),

    Sessions(SessionsArgs),

    Tasks(TasksArgs),

    MemorySearch(MemorySearchArgs),

    EnhanceCompare(EnhanceCompareArgs),

    Diarize(DiarizeArgs),

    DiarizeTranscript(DiarizeTranscriptArgs),
}

#[derive(Parser, Debug)]
pub struct DiarizeTranscriptArgs {
    pub session_dir: PathBuf,

    #[arg(long)]
    pub segmentation: Option<PathBuf>,

    #[arg(long)]
    pub embedding: Option<PathBuf>,

    #[arg(long, default_value_t = 0)]
    pub num_speakers: i32,

    #[arg(long, default_value_t = 0.5, allow_hyphen_values = true)]
    pub threshold: f32,
}

#[derive(Parser, Debug)]
pub struct DiarizeArgs {
    pub input: PathBuf,

    #[arg(long, default_value = "system")]
    pub channel: String,

    #[arg(long)]
    pub segmentation: Option<PathBuf>,

    #[arg(long)]
    pub embedding: Option<PathBuf>,

    #[arg(long, default_value_t = 0)]
    pub num_speakers: i32,

    #[arg(long, default_value_t = 0.5, allow_hyphen_values = true)]
    pub threshold: f32,

    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct EnhanceCompareArgs {
    pub input: PathBuf,

    #[arg(long)]
    pub out: Option<PathBuf>,

    #[arg(long, default_value_t = -20.0, allow_hyphen_values = true)]
    pub atten_lim_db: f32,
}

#[derive(Parser, Debug)]
pub struct SessionsArgs {
    #[arg(long, default_value = "./recordings")]
    pub output: PathBuf,

    #[arg(long, default_value_t = false)]
    pub table: bool,

    #[arg(long, default_value_t = 0)]
    pub limit: usize,
}

#[derive(Parser, Debug)]
pub struct TasksArgs {
    #[arg(long, default_value = "./tasks/tasks.json")]
    pub path: PathBuf,

    #[arg(long)]
    pub status: Option<String>,

    #[arg(long, default_value_t = false)]
    pub table: bool,
}

#[derive(Parser, Debug)]
pub struct MemorySearchArgs {
    #[arg(long, default_value = "./memory")]
    pub dir: PathBuf,

    pub query: String,

    #[arg(long)]
    pub kind: Option<String>,

    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    #[arg(long, default_value_t = false)]
    pub table: bool,
}

#[cfg(target_os = "macos")]
#[derive(Parser, Debug)]
pub struct VpioSmokeArgs {
    #[arg(long, default_value_t = 5)]
    pub seconds: u64,

    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct TranscribeArgs {
    pub audio: PathBuf,

    #[arg(long)]
    pub model: Option<PathBuf>,

    #[arg(long)]
    pub language: Option<String>,

    #[arg(long, default_value_t = false)]
    pub raw: bool,

    #[arg(long)]
    pub no_speech_thold: Option<f32>,

    #[arg(long, default_value_t = false)]
    pub greedy: bool,

    #[arg(long, default_value_t = false)]
    pub allow_non_speech_tokens: bool,

    #[arg(long)]
    pub entropy_thold: Option<f32>,

    #[arg(long)]
    pub logprob_thold: Option<f32>,

    #[arg(long, default_value_t = false)]
    pub no_initial_prompt: bool,

    #[arg(long, default_value_t = false)]
    pub library: bool,
}

#[derive(Parser, Debug)]
pub struct RecordArgs {
    #[arg(long, default_value_t = 10)]
    pub seconds: u64,

    #[arg(long, default_value = "./recordings")]
    pub output: PathBuf,

    #[arg(long, default_value_t = false)]
    pub no_mic: bool,

    #[arg(long, default_value_t = false)]
    pub no_system: bool,

    #[arg(long)]
    pub mic_device: Option<String>,

    #[arg(long)]
    pub sample_rate: Option<u32>,

    #[arg(long, default_value_t = false)]
    pub no_voice_processing: bool,
}
