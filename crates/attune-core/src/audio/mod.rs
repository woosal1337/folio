//! Audio capture pipeline.
//!
//! Two independent capture sources feed two independent WAV writers in v0
//! week 1. The transcription pipeline (week 2-3) consumes the same captured
//! buffers via ring buffers. See `architecture/audio-capture.md` in the design
//! vault for the full pipeline diagram.

pub mod capture;
pub mod devices;
pub mod enhancement;
pub mod inflight;
pub mod mic;
pub mod mic_monitor;
#[cfg(target_os = "macos")]
pub mod process_tap;
pub mod resampler;
pub mod system;
pub mod vad;
pub mod vad_filter;
#[cfg(target_os = "macos")]
pub mod voice_processing_capture;
pub mod wav_writer;

pub use capture::{CaptureArtifacts, CaptureSession, RecordingResult, RecordingStatus};
pub use devices::{list_input_devices, DeviceInfo};
pub use wav_writer::concat_wavs;

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
    /// Microphone device by name. `None` selects the default input device.
    pub mic_device_name: Option<String>,
    /// Optional override for the on-disk sample rate. When `None` (the
    /// default), each source writes at its own native rate: the device's
    /// reported default rate for the mic (typically 44.1 or 48 kHz) and
    /// 48 kHz for ScreenCaptureKit system audio. Set this when you need a
    /// specific rate (e.g. 16 kHz for direct Whisper input).
    pub target_sample_rate: Option<u32>,
    /// Output directory for WAV files. A timestamped subdirectory is created
    /// per session.
    pub output_dir: std::path::PathBuf,
    /// macOS only. When true, mic capture goes through Apple's
    /// Voice Processing IO AudioUnit (AEC + noise suppression + AGC)
    /// instead of the plain cpal path. Stops the mic from picking up
    /// system audio when the user is not wearing headphones. The
    /// session falls back to the cpal path automatically if VPIO
    /// fails to initialise on the bound device (aggregate devices,
    /// certain USB interfaces, etc.). No-op on non-macOS targets.
    pub voice_processing_enabled: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mic_enabled: true,
            system_enabled: true,
            mic_device_name: None,
            target_sample_rate: None,
            output_dir: std::path::PathBuf::from("./recordings"),
            voice_processing_enabled: true,
        }
    }
}
