use thiserror::Error;

/// Public error type for [`attune-core`]. New error categories are added here,
/// not invented per-module.
#[derive(Error, Debug)]
pub enum AttuneError {
    #[error("no default audio input device available")]
    NoInputDevice,

    #[error("audio device error: {0}")]
    AudioDevice(String),

    #[error("audio stream build failed: {0}")]
    StreamBuild(String),

    #[error("audio stream play failed: {0}")]
    StreamPlay(String),

    #[error("system audio capture requires macOS 13.0 or later")]
    SystemAudioUnsupported,

    #[error("system audio capture failed: {0}")]
    SystemAudio(String),

    #[error("resampler error: {0}")]
    Resampler(String),

    #[error("wav writer error: {0}")]
    WavWriter(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("hound (wav) error: {0}")]
    Hound(#[from] hound::Error),

    #[error("internal: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AttuneError>;
