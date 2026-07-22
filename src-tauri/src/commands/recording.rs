use std::path::PathBuf;
use std::time::Instant;

use folio_core::audio::{
    concat_wavs, CaptureArtifacts, CaptureConfig, CaptureSession, RecordingResult, RecordingStatus,
};
use folio_core::storage::RecordingSummary;
use tauri::{Emitter, State};
use tracing::{debug, info};

use crate::app::state::PausedNote;
use crate::app::AppState;

fn maybe_start_live_transcript(app: &tauri::AppHandle, state: &AppState, session_dir: PathBuf) {
    use folio_core::transcription::{WhisperModel, WhisperModelStore};

    let (enabled, kind, model_id, language) = {
        let s = state.settings.lock();
        (
            s.live_transcript_enabled,
            s.transcriber.clone(),
            s.local_whisper_model.clone(),
            s.transcription_language.clone(),
        )
    };

    if !enabled {
        return;
    }
    if kind != "local_whisper" {
        return;
    }
    let Some(model) = WhisperModel::from_id(&model_id) else {
        return;
    };
    let status = WhisperModelStore::default_location().status(model);
    if !status.present {
        return;
    }
    let hint = (!language.is_empty() && language != "auto").then_some(language);

    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state.live_transcript_stop.lock() = Some(stop.clone());

    let handle =
        crate::app::live_transcript::spawn(app.clone(), session_dir, status.path, hint, stop);
    *state.live_transcript_thread.lock() = Some(handle);
}

#[tauri::command]
pub fn recording_status(state: State<'_, AppState>) -> RecordingStatus {
    debug!("recording_status");
    state.recording_status()
}

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

#[tauri::command]
pub async fn create_note(state: State<'_, AppState>) -> Result<RecordingSummary, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<RecordingSummary, String> {
        let label = chrono::Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();
        let dir = output_dir.join(&label);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        folio_core::storage::atomic_write::atomic_write(&dir.join("live_notes.json"), b"[]")
            .map_err(|e| e.to_string())?;

        let draft_name = folio_core::storage::session::allocate_draft_name(&output_dir);
        folio_core::storage::atomic_write::atomic_write(
            &dir.join("draft.txt"),
            draft_name.as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        info!(dir = %dir.display(), draft = %draft_name, "created empty note");
        Ok(RecordingSummary {
            session_dir: dir,
            label,
            duration_seconds: 0,
            mic_bytes: None,
            system_bytes: None,
            mic_sample_rate: None,
            system_sample_rate: None,
            created_at: Some(chrono::Utc::now()),
            has_transcript: false,
            title: None,
            folder: None,
            draft_name: Some(draft_name),
            suggested_title: None,
            suggested_tags: Vec::new(),
            suggested_subtitle: None,
            language_override: None,
            sync: None,
        })
    })
    .await
    .map_err(|e| format!("create_note task panicked: {e}"))?
}

#[tauri::command]
pub async fn rename_note(
    state: State<'_, AppState>,
    session_dir: String,
    title: String,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();

    let dir =
        folio_core::paths::canonicalize_under(&output_dir, std::path::Path::new(&session_dir))
            .map_err(|e| format!("invalid session directory: {e}"))?;
    let trimmed = title.trim().to_string();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let path = dir.join("title.txt");
        if trimmed.is_empty() {
            match std::fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        } else {
            folio_core::storage::atomic_write::atomic_write(&path, trimmed.as_bytes())
                .map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("rename_note task panicked: {e}"))?
}

#[tauri::command]
pub async fn get_enhanced_notes_accepted(session_dir: String) -> Result<Option<String>, String> {
    let dir = PathBuf::from(&session_dir);
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::read_to_string(dir.join("enhanced-notes-accepted.txt"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
    .await
    .map_err(|e| format!("get_enhanced_notes_accepted task panicked: {e}"))
}

#[tauri::command]
pub async fn set_enhanced_notes_accepted(
    session_dir: String,
    marker: String,
) -> Result<(), String> {
    let dir = PathBuf::from(&session_dir);
    if !dir.is_dir() {
        return Err(format!("session directory does not exist: {session_dir}"));
    }
    let trimmed = marker.trim().to_string();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let path = dir.join("enhanced-notes-accepted.txt");
        if trimmed.is_empty() {
            match std::fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        } else {
            folio_core::storage::atomic_write::atomic_write(&path, trimmed.as_bytes())
                .map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("set_enhanced_notes_accepted task panicked: {e}"))?
}

#[tauri::command]
pub async fn start_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_dir: Option<String>,
) -> Result<RecordingStatus, String> {
    if state.session.lock().is_some() {
        return Err("already recording".into());
    }

    *state.active_note.lock() = None;
    let config = capture_config(&state);

    let session_dir = match session_dir {
        Some(dir) => Some(
            folio_core::paths::canonicalize_under(&config.output_dir, std::path::Path::new(&dir))
                .map_err(|e| format!("invalid session directory: {e}"))?,
        ),
        None => None,
    };

    info!(
        device = ?config.mic_device_name,
        system = config.system_enabled,
        voice_processing = config.voice_processing_enabled,
        output = %config.output_dir.display(),
        into = ?session_dir,
        "starting capture"
    );

    let session = tauri::async_runtime::spawn_blocking(move || match session_dir {
        Some(dir) => CaptureSession::start_in(config, dir),
        None => CaptureSession::start(config),
    })
    .await
    .map_err(|e| format!("start_recording task panicked: {e}"))?
    .map_err(|e| e.to_string())?;

    let channels = session.channels_active();
    if channels.is_empty() {
        return Err(
            "No capture channels available. Check microphone permission in System Settings → Privacy.".into(),
        );
    }

    let live_dir = session.session_dir().clone();
    *state.session.lock() = Some(session);
    *state.recording_started.lock() = Some(Instant::now());

    maybe_start_live_transcript(&app, &state, live_dir);

    Ok(state.recording_status())
}

#[tauri::command]
pub async fn pause_recording(state: State<'_, AppState>) -> Result<RecordingStatus, String> {
    state.stop_live_transcript();
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

#[tauri::command]
pub async fn resume_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<RecordingStatus, String> {
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

    let live_dir = session.session_dir().clone();
    *state.session.lock() = Some(session);
    *state.recording_started.lock() = Some(Instant::now());
    maybe_start_live_transcript(&app, &state, live_dir);
    info!("recording resumed");
    Ok(state.recording_status())
}

pub const STITCHING_STARTED_EVENT: &str = "recording:stitching-started";

pub const STITCHING_DONE_EVENT: &str = "recording:stitching-done";

#[tauri::command]
pub async fn stop_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<RecordingResult, String> {
    state.stop_live_transcript();
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

    let note = state.active_note.lock().take();
    let artifacts = if let Some(note) = note {
        let _ = app.emit(STITCHING_STARTED_EVENT, ());
        let result = merge_note_segments(note, artifacts).await;
        let _ = app.emit(STITCHING_DONE_EVENT, ());
        result?
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
