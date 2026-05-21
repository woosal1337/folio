//! Tauri command handlers. Each `#[tauri::command]` function is callable
//! from the React frontend via `invoke('command_name', args)`.

use std::path::PathBuf;
use std::time::Instant;

use attune_core::audio::{
    list_input_devices as core_list_input_devices, CaptureArtifacts, CaptureConfig,
    CaptureSession, DeviceInfo,
};
use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use tauri::State;
use tracing::info;
#[cfg(not(target_os = "macos"))]
use tracing::warn;

use crate::state::{AppState, RecordingStatus, Settings};

/// Health-check command used during scaffolding to verify the IPC bridge.
#[tauri::command]
pub fn ping(name: Option<String>) -> String {
    match name {
        Some(n) => format!("pong, {n}"),
        None => "pong".into(),
    }
}

/// Enumerate input audio devices visible to the system.
#[tauri::command]
pub fn list_input_devices() -> Result<Vec<DeviceInfo>, String> {
    core_list_input_devices().map_err(|e| e.to_string())
}

/// Read the current settings.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().clone()
}

/// Persist new settings.
#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    *state.settings.lock() = settings;
    Ok(())
}

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn recording_status(state: State<'_, AppState>) -> RecordingStatus {
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

#[derive(Debug, Serialize)]
pub struct RecordingResult {
    pub artifacts: CaptureArtifacts,
    pub label: String,
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

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct RecordingSummary {
    pub session_dir: PathBuf,
    pub label: String,
    pub duration_seconds: i64,
    pub mic_bytes: Option<u64>,
    pub system_bytes: Option<u64>,
    pub mic_sample_rate: Option<u32>,
    pub system_sample_rate: Option<u32>,
    pub created_at: Option<DateTime<Utc>>,
}

#[tauri::command]
pub fn list_recordings(state: State<'_, AppState>) -> Vec<RecordingSummary> {
    let output_dir = state.settings.lock().output_dir.clone();
    scan_recordings(&output_dir)
}

fn scan_recordings(output_dir: &std::path::Path) -> Vec<RecordingSummary> {
    let Ok(entries) = std::fs::read_dir(output_dir) else {
        return Vec::new();
    };
    let mut out: Vec<RecordingSummary> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let label = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "session".into());
        let mic_path = path.join("mic.wav");
        let system_path = path.join("system.wav");
        let mic_bytes = std::fs::metadata(&mic_path).ok().map(|m| m.len());
        let system_bytes = std::fs::metadata(&system_path).ok().map(|m| m.len());
        if mic_bytes.is_none() && system_bytes.is_none() {
            continue;
        }
        let mic_sample_rate = wav_sample_rate(&mic_path);
        let system_sample_rate = wav_sample_rate(&system_path);
        let duration_seconds = wav_duration_seconds(&mic_path)
            .or_else(|| wav_duration_seconds(&system_path))
            .unwrap_or(0);
        let created_at = entry
            .metadata()
            .ok()
            .and_then(|m| m.created().ok())
            .map(|t| {
                let dt: DateTime<Local> = t.into();
                dt.with_timezone(&Utc)
            });
        out.push(RecordingSummary {
            session_dir: path,
            label,
            duration_seconds,
            mic_bytes,
            system_bytes,
            mic_sample_rate,
            system_sample_rate,
            created_at,
        });
    }
    out.sort_by(|a, b| b.label.cmp(&a.label));
    out
}

fn wav_sample_rate(path: &std::path::Path) -> Option<u32> {
    Some(hound::WavReader::open(path).ok()?.spec().sample_rate)
}

fn wav_duration_seconds(path: &std::path::Path) -> Option<i64> {
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let frames = reader.duration() as u64;
    if spec.sample_rate == 0 {
        return None;
    }
    Some((frames / spec.sample_rate as u64) as i64)
}

/// Delete a recording session directory.
///
/// Refuses to delete unless the path lies under the user's configured
/// recordings folder — defence in depth so a bug in the frontend can't
/// trigger an `rm -rf /` situation.
#[tauri::command]
pub fn delete_recording(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let canon_root = std::fs::canonicalize(&output_dir).map_err(|e| {
        format!(
            "could not canonicalize recordings dir {}: {e}",
            output_dir.display()
        )
    })?;
    let canon_target = std::fs::canonicalize(&session_dir).map_err(|e| {
        format!(
            "could not canonicalize session dir {}: {e}",
            session_dir.display()
        )
    })?;
    if !canon_target.starts_with(&canon_root) {
        return Err(format!(
            "refused to delete {}: not under recordings folder {}",
            canon_target.display(),
            canon_root.display(),
        ));
    }
    if canon_target == canon_root {
        return Err("refused to delete the recordings folder itself".into());
    }
    std::fs::remove_dir_all(&canon_target).map_err(|e| {
        format!("could not delete {}: {e}", canon_target.display())
    })?;
    info!(path = %canon_target.display(), "recording deleted");
    Ok(())
}

#[tauri::command]
pub fn reveal_in_finder(path: PathBuf) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        warn!("reveal_in_finder not implemented on this platform");
        Ok(())
    }
}
