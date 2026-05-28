//! Recording lifecycle: start, stop, pause, resume, and poll the
//! current capture session.

use std::time::Instant;

use attune_core::audio::{
    concat_wavs, CaptureArtifacts, CaptureConfig, CaptureSession, RecordingResult, RecordingStatus,
};
use tauri::State;
use tracing::{debug, info};

use crate::app::state::PausedNote;
use crate::app::AppState;

/// Snapshot of the current recording session for the UI. Pure
/// in-memory read so this stays sync.
#[tauri::command]
pub fn recording_status(state: State<'_, AppState>) -> RecordingStatus {
    debug!("recording_status");
    state.recording_status()
}

/// Build the capture config from the current settings.
fn capture_config(state: &AppState) -> CaptureConfig {
    let settings = state.settings.lock().clone();
    CaptureConfig {
        mic_enabled: true,
        system_enabled: settings.system_audio_enabled,
        mic_device_name: settings.mic_device.clone(),
        target_sample_rate: None,
        output_dir: settings.output_dir.clone(),
        voice_processing_enabled: settings.voice_processing_enabled,
    }
}

/// Start a new capture session. Building the cpal stream and the
/// ScreenCaptureKit pipeline takes real OS calls; we run that work on
/// a blocking task so the Tauri command runtime is free to dispatch
/// other commands in the meantime.
#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> Result<RecordingStatus, String> {
    if state.session.lock().is_some() {
        return Err("already recording".into());
    }
    // A fresh recording abandons any paused note from a prior session.
    *state.active_note.lock() = None;
    let config = capture_config(&state);

    info!(
        device = ?config.mic_device_name,
        system = config.system_enabled,
        voice_processing = config.voice_processing_enabled,
        output = %config.output_dir.display(),
        "starting capture"
    );

    let session = tauri::async_runtime::spawn_blocking(move || CaptureSession::start(config))
        .await
        .map_err(|e| format!("start_recording task panicked: {e}"))?
        .map_err(|e| e.to_string())?;

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

/// Pause the in-progress recording (GET-149). Finalizes the current
/// segment's WAVs and keeps the note open so a Resume continues into the
/// same note. The first pause promotes the recording into a multi-part
/// note rooted at its session dir.
#[tauri::command]
pub async fn pause_recording(state: State<'_, AppState>) -> Result<RecordingStatus, String> {
    let session = state
        .session
        .lock()
        .take()
        .ok_or_else(|| "not recording".to_string())?;
    let segment_secs = state
        .recording_started
        .lock()
        .take()
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);

    let artifacts = tauri::async_runtime::spawn_blocking(move || session.stop())
        .await
        .map_err(|e| format!("pause_recording task panicked: {e}"))?
        .map_err(|e| e.to_string())?;

    let mut note = state.active_note.lock();
    match note.as_mut() {
        Some(n) => {
            if let Some(m) = artifacts.mic_path {
                n.mic_parts.push(m);
            }
            if let Some(s) = artifacts.system_path {
                n.system_parts.push(s);
            }
            n.base_offset_secs += segment_secs;
            n.next_part += 1;
        }
        None => {
            *note = Some(PausedNote {
                dir: artifacts.session_dir.clone(),
                mic_parts: artifacts.mic_path.into_iter().collect(),
                system_parts: artifacts.system_path.into_iter().collect(),
                base_offset_secs: segment_secs,
                next_part: 1,
                started_at: artifacts.started_at,
            });
        }
    }
    drop(note);
    info!("recording paused");
    Ok(state.recording_status())
}

/// Resume a paused note (GET-149). Starts a new capture segment that
/// records into `dir/parts/NNN/`; the final stop merges every segment
/// into one continuous file.
#[tauri::command]
pub async fn resume_recording(state: State<'_, AppState>) -> Result<RecordingStatus, String> {
    if state.session.lock().is_some() {
        return Err("already recording".into());
    }
    let (part_dir,) = {
        let note = state.active_note.lock();
        let n = note
            .as_ref()
            .ok_or_else(|| "no paused recording to resume".to_string())?;
        (n.dir.join("parts").join(format!("{:03}", n.next_part)),)
    };
    let config = capture_config(&state);

    let session =
        tauri::async_runtime::spawn_blocking(move || CaptureSession::start_in(config, part_dir))
            .await
            .map_err(|e| format!("resume_recording task panicked: {e}"))?
            .map_err(|e| e.to_string())?;

    let channels = session.channels_active();
    if channels.is_empty() {
        return Err(
            "No capture channels available. Check microphone permission in System Settings → Privacy.".into(),
        );
    }

    *state.session.lock() = Some(session);
    *state.recording_started.lock() = Some(Instant::now());
    info!("recording resumed");
    Ok(state.recording_status())
}

/// Stop the current capture session, finalize the WAVs, and return
/// the artifacts. For a multi-part note (the user paused at least once),
/// the segments are merged into one continuous `mic.wav` / `system.wav`
/// before returning.
#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<RecordingResult, String> {
    let session = state
        .session
        .lock()
        .take()
        .ok_or_else(|| "not recording".to_string())?;
    *state.recording_started.lock() = None;

    let artifacts = tauri::async_runtime::spawn_blocking(move || session.stop())
        .await
        .map_err(|e| format!("stop_recording task panicked: {e}"))?
        .map_err(|e| e.to_string())?;

    // Multi-part note: merge every segment into the note dir. Single-shot
    // recordings (note is None) return the artifacts untouched.
    let note = state.active_note.lock().take();
    let artifacts = if let Some(note) = note {
        merge_note_segments(note, artifacts).await?
    } else {
        artifacts
    };

    let label = artifacts
        .session_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "session".into());

    info!(dir = %artifacts.session_dir.display(), "capture stopped");

    Ok(RecordingResult { artifacts, label })
}

/// Append the final segment to the note's parts and concatenate them
/// into one continuous `mic.wav` / `system.wav` in the note dir.
async fn merge_note_segments(
    note: PausedNote,
    final_segment: CaptureArtifacts,
) -> Result<CaptureArtifacts, String> {
    let dir = note.dir.clone();
    let started_at = note.started_at;
    let stopped_at = final_segment.stopped_at;

    let mut mic_parts = note.mic_parts;
    if let Some(m) = final_segment.mic_path {
        mic_parts.push(m);
    }
    let mut system_parts = note.system_parts;
    if let Some(s) = final_segment.system_path {
        system_parts.push(s);
    }

    let mic_out = dir.join("mic.wav");
    let system_out = dir.join("system.wav");
    let has_mic = !mic_parts.is_empty();
    let has_system = !system_parts.is_empty();

    let mic_out_task = mic_out.clone();
    let system_out_task = system_out.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        if has_mic {
            concat_wavs(&mic_parts, &mic_out_task).map_err(|e| e.to_string())?;
        }
        if has_system {
            concat_wavs(&system_parts, &system_out_task).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("merge task panicked: {e}"))??;

    info!(dir = %dir.display(), "merged paused note segments");
    Ok(CaptureArtifacts {
        session_dir: dir,
        mic_path: has_mic.then_some(mic_out),
        system_path: has_system.then_some(system_out),
        started_at,
        stopped_at,
    })
}
