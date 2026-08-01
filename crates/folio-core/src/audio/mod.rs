pub mod capture;
pub mod devices;
pub mod enhancement;
pub mod level_meter;
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

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub mic_enabled: bool,
    pub system_enabled: bool,

    pub mic_device_name: Option<String>,

    pub target_sample_rate: Option<u32>,

    pub output_dir: std::path::PathBuf,

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
