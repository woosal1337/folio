//! Recording lifecycle: start, stop, and poll the current capture
//! session.

use std::time::Instant;

use attune_core::audio::{CaptureConfig, CaptureSession, RecordingResult, RecordingStatus};
use tauri::State;
use tracing::{debug, info};

use crate::app::AppState;

#[tauri::command]
pub fn recording_status(state: State<'_, AppState>) -> RecordingStatus {
    debug!("recording_status");
    state.recording_status()
}

#[tauri::command]
pub fn start_recording(state: State<'_, AppState>) -> Result<RecordingStatus, String> {
    if state.session.lock().is_some() {
        return Err("already recording".into());
    }
    let settings = state.settings.lock().clone();
    let config = CaptureConfig {
        mic_enabled: true,
        system_enabled: settings.system_audio_enabled,
        mic_device_name: settings.mic_device.clone(),
        target_sample_rate: None,
        output_dir: settings.output_dir.clone(),
    };

    info!(
        device = ?config.mic_device_name,
        system = config.system_enabled,
        output = %config.output_dir.display(),
        "starting capture"
    );

    let session = CaptureSession::start(config).map_err(|e| e.to_string())?;
    let channels = session.channels_active();
    if channels.is_empty() {
        return Err(
            "No capture channels available. Check microphone permission in System Settings → Privacy.".into(),
        );
    }

    *state.session.lock() = Some(session);
    *state.recording_started.lock() = Some(Instant::now());

    Ok(state.recording_status())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) -> Result<RecordingResult, String> {
    let session = state
        .session
        .lock()
        .take()
        .ok_or_else(|| "not recording".to_string())?;
    *state.recording_started.lock() = None;

    let artifacts = session.stop().map_err(|e| e.to_string())?;
    let label = artifacts
        .session_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "session".into());

    info!(dir = %artifacts.session_dir.display(), "capture stopped");

    Ok(RecordingResult { artifacts, label })
}
