//! Audio capture pipeline.
//!
//! Two independent capture sources feed two independent WAV writers in v0
//! week 1. The transcription pipeline (week 2-3) consumes the same captured
//! buffers via ring buffers. See `architecture/audio-capture.md` in the design
//! vault for the full pipeline diagram.

pub mod capture;
pub mod mic;
pub mod resampler;
pub mod system;
pub mod wav_writer;

pub use capture::{CaptureArtifacts, CaptureSession};

/// Capture source channel. Used for routing and labeling downstream
/// (`me` for [`Channel::Microphone`], `others` for [`Channel::System`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    System,
    Microphone,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::System => "system",
            Channel::Microphone => "mic",
        }
    }
}

/// Capture parameters shared across both channels.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub mic_enabled: bool,
    pub system_enabled: bool,
    /// Target sample rate fed to the transcription pipeline. Whisper expects
    /// 16 kHz mono.
    pub target_sample_rate: u32,
    /// Output directory for WAV files. A timestamped subdirectory is created
    /// per session.
    pub output_dir: std::path::PathBuf,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mic_enabled: true,
            system_enabled: true,
            target_sample_rate: 16_000,
            output_dir: std::path::PathBuf::from("./recordings"),
        }
    }
}
