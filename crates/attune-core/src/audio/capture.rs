//! Capture orchestrator.
//!
//! Starts mic + system capture, manages the WAV writers, and exposes a single
//! [`CaptureSession::start`] / [`CaptureSession::stop`] interface to callers.
//! v0 week 1 wires mic via cpal; system capture is stubbed (see
//! `system.rs`). Mic-only capture continues if system capture is unavailable.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use tracing::{info, warn};

use crate::audio::mic::MicCapture;
use crate::audio::system::SystemCapture;
use crate::audio::wav_writer::AudioWavWriter;
use crate::audio::{CaptureConfig, Channel};
use crate::error::Result;

pub struct CaptureSession {
    config: CaptureConfig,
    started_at: DateTime<Utc>,
    session_dir: PathBuf,
    mic: Option<MicCapture>,
    system: Option<SystemCapture>,
    system_started: bool,
}

pub struct CaptureArtifacts {
    pub session_dir: PathBuf,
    pub mic_path: Option<PathBuf>,
    pub system_path: Option<PathBuf>,
    pub started_at: DateTime<Utc>,
    pub stopped_at: DateTime<Utc>,
}

impl CaptureSession {
    /// Start a new capture session. Creates the timestamped output directory,
    /// opens WAV writers, and begins streaming audio from the enabled sources.
    pub fn start(config: CaptureConfig) -> Result<Self> {
        let started_at = SystemTime::now();
        let started_at_dt: DateTime<Utc> = started_at.into();
        let session_dir = config
            .output_dir
            .join(started_at_dt.format("%Y-%m-%d-%H-%M-%S").to_string());
        std::fs::create_dir_all(&session_dir)?;
        info!(dir = %session_dir.display(), "capture session started");

        let mic = if config.mic_enabled {
            let path = session_dir.join("mic.wav");
            let writer = Arc::new(AudioWavWriter::create(&path, config.target_sample_rate)?);
            match MicCapture::start(
                writer.clone(),
                config.target_sample_rate,
                config.mic_device_name.as_deref(),
            ) {
                Ok(c) => {
                    info!(path = %path.display(), "mic capture started");
                    Some(c)
                }
                Err(e) => {
                    warn!(error = %e, "mic capture failed to start");
                    None
                }
            }
        } else {
            None
        };

        let mut system_started = false;
        let system = if config.system_enabled {
            let path = session_dir.join("system.wav");
            let writer = Arc::new(AudioWavWriter::create(&path, config.target_sample_rate)?);
            match SystemCapture::start(writer.clone(), config.target_sample_rate) {
                Ok(c) => {
                    info!(path = %path.display(), "system audio capture started");
                    system_started = true;
                    Some(c)
                }
                Err(e) => {
                    warn!(error = %e, "system audio capture unavailable, continuing without it");
                    // Drop the writer so it doesn't keep an orphan header-only
                    // file on disk, then unlink the file.
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

    /// Stop both capture sources, finalize WAVs, return paths to the produced
    /// files.
    pub fn stop(self) -> Result<CaptureArtifacts> {
        if let Some(mic) = self.mic {
            mic.stop()?;
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
