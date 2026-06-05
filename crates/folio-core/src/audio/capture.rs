use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use ts_rs::TS;

use crate::audio::devices::default_input_sample_rate;
use crate::audio::mic::MicCapture;
use crate::audio::system::SystemCapture;
#[cfg(target_os = "macos")]
use crate::audio::voice_processing_capture::VoiceProcessingMicCapture;
use crate::audio::wav_writer::AudioWavWriter;
use crate::audio::{CaptureConfig, Channel};
use crate::error::Result;

enum MicHandle {
    Cpal(MicCapture),
    #[cfg(target_os = "macos")]
    VoiceProcessing(VoiceProcessingMicCapture),
}

impl MicHandle {
    fn stop(self) -> Result<()> {
        match self {
            MicHandle::Cpal(c) => c.stop(),
            #[cfg(target_os = "macos")]
            MicHandle::VoiceProcessing(v) => v.stop(),
        }
    }

    fn is_silent(&self) -> bool {
        match self {
            MicHandle::Cpal(c) => c.is_silent(),
            #[cfg(target_os = "macos")]
            MicHandle::VoiceProcessing(v) => v.is_silent(),
        }
    }
}

impl std::fmt::Debug for MicHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MicHandle::Cpal(_) => f.write_str("cpal"),
            #[cfg(target_os = "macos")]
            MicHandle::VoiceProcessing(_) => f.write_str("voice-processing-io"),
        }
    }
}

fn start_mic_with_fallback(
    config: &CaptureConfig,
    writer: Arc<AudioWavWriter>,
    mic_rate: u32,
) -> Option<MicHandle> {
    #[cfg(target_os = "macos")]
    {
        if config.voice_processing_enabled {
            match VoiceProcessingMicCapture::start(writer.clone(), mic_rate) {
                Ok(v) => return Some(MicHandle::VoiceProcessing(v)),
                Err(e) => {
                    warn!(error = %e, "VPIO mic capture failed; falling back to cpal");
                }
            }
        }
    }

    match MicCapture::start(writer, mic_rate, config.mic_device_name.as_deref()) {
        Ok(c) => Some(MicHandle::Cpal(c)),
        Err(e) => {
            warn!(error = %e, "cpal mic capture failed to start");
            None
        }
    }
}

const SYSTEM_NATIVE_RATE: u32 = 48_000;

pub struct CaptureSession {
    config: CaptureConfig,
    started_at: DateTime<Utc>,
    session_dir: PathBuf,
    mic: Option<MicHandle>,
    system: Option<SystemCapture>,
    system_started: bool,
}

unsafe impl Send for CaptureSession {}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct CaptureArtifacts {
    pub session_dir: PathBuf,
    pub mic_path: Option<PathBuf>,
    pub system_path: Option<PathBuf>,
    pub started_at: DateTime<Utc>,
    pub stopped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct RecordingStatus {
    pub recording: bool,
    pub elapsed_secs: u64,
    pub channels: Vec<String>,

    pub session_dir: Option<String>,

    pub paused: bool,

    #[serde(default)]
    pub mic_silent: bool,

    #[serde(default)]
    pub needs_segment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct RecordingResult {
    pub artifacts: CaptureArtifacts,
    pub label: String,
}

impl CaptureSession {
    pub fn start(config: CaptureConfig) -> Result<Self> {
        let started_at_dt: DateTime<Utc> = SystemTime::now().into();
        let session_dir = config
            .output_dir
            .join(started_at_dt.format("%Y-%m-%d-%H-%M-%S").to_string());
        Self::start_in(config, session_dir)
    }

    pub fn start_in(config: CaptureConfig, session_dir: PathBuf) -> Result<Self> {
        let started_at_dt: DateTime<Utc> = SystemTime::now().into();
        std::fs::create_dir_all(&session_dir).map_err(|e| {
            crate::error::FolioError::Storage(format!(
                "create session dir {}: {e}",
                session_dir.display()
            ))
        })?;
        info!(dir = %session_dir.display(), "capture session started");

        let mic = if config.mic_enabled {
            let mic_rate = match config.target_sample_rate {
                Some(rate) => rate,
                None => default_input_sample_rate(config.mic_device_name.as_deref())
                    .unwrap_or_else(|e| {
                        warn!(error = %e, "could not query mic native rate; falling back to 48000");
                        48_000
                    }),
            };
            let path = session_dir.join("mic.wav");
            let writer = Arc::new(AudioWavWriter::create(&path, mic_rate)?);
            let handle = start_mic_with_fallback(&config, writer.clone(), mic_rate);
            match handle {
                Some(h) => {
                    info!(path = %path.display(), rate = mic_rate, mode = ?h, "mic capture started");
                    Some(h)
                }
                None => {
                    drop(writer);
                    let _ = std::fs::remove_file(&path);
                    None
                }
            }
        } else {
            None
        };

        let mut system_started = false;
        let system = if config.system_enabled {
            let sys_rate = config.target_sample_rate.unwrap_or(SYSTEM_NATIVE_RATE);
            let path = session_dir.join("system.wav");
            let writer = Arc::new(AudioWavWriter::create(&path, sys_rate)?);
            match SystemCapture::start(writer.clone(), sys_rate) {
                Ok(c) => {
                    info!(path = %path.display(), rate = sys_rate, "system audio capture started");
                    system_started = true;
                    Some(c)
                }
                Err(e) => {
                    warn!(error = %e, "system audio capture unavailable, continuing without it");
                    drop(writer);
                    let _ = std::fs::remove_file(&path);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            started_at: started_at_dt,
            session_dir,
            mic,
            system,
            system_started,
        })
    }

    pub fn session_dir(&self) -> &PathBuf {
        &self.session_dir
    }
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn is_mic_silent(&self) -> bool {
        self.mic.as_ref().map(|h| h.is_silent()).unwrap_or(false)
    }

    pub fn channels_active(&self) -> Vec<Channel> {
        let mut v = Vec::new();
        if self.mic.is_some() {
            v.push(Channel::Microphone);
        }
        if self.system.is_some() {
            v.push(Channel::System);
        }
        v
    }

    pub fn stop(self) -> Result<CaptureArtifacts> {
        if let Some(mic) = self.mic {
            MicHandle::stop(mic)?;
        }
        if let Some(sys) = self.system {
            sys.stop()?;
        }
        let stopped_at: DateTime<Utc> = SystemTime::now().into();
        let mic_path = self
            .config
            .mic_enabled
            .then(|| self.session_dir.join("mic.wav"));
        let system_path = self
            .system_started
            .then(|| self.session_dir.join("system.wav"));
        info!(
            dir = %self.session_dir.display(),
            duration_s = (stopped_at - self.started_at).num_seconds(),
            "capture session stopped",
        );
        Ok(CaptureArtifacts {
            session_dir: self.session_dir,
            mic_path,
            system_path,
            started_at: self.started_at,
            stopped_at,
        })
    }
}
