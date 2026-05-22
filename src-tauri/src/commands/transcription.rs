//! Transcription: kick off an OpenAI Whisper run against a recorded
//! session, persist the result inside the session directory, and hand
//! the parsed transcript back to the UI.

use std::path::{Path, PathBuf};

use attune_core::storage::session::TRANSCRIPT_FILENAME;
use attune_core::transcription::{
    LocalWhisperTranscriber, OpenAiTranscriber, Transcriber, Transcript, TranscriptionResult,
    WhisperModel, WhisperModelStatus, WhisperModelStore,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracing::{debug, info};

use crate::app::AppState;

/// Tauri event channel name for live download progress.
const DOWNLOAD_PROGRESS_EVENT: &str = "whisper:model-download-progress";

/// Transcribe a previously recorded session.
///
/// Dispatches on `settings.transcriber`:
///   - "openai"         → OpenAI Whisper API (uploads the WAV).
///   - "local_whisper"  → On-device whisper.cpp inference. Requires the
///                        model file referenced by
///                        `settings.local_whisper_model` to be present
///                        on disk; if not, the user is prompted to
///                        download it from Settings.
///
/// Picks `mic.wav` if present (the canonical "me" channel), falls back
/// to `system.wav` otherwise. Writes the resulting transcript JSON to
/// `<session_dir>/transcript.json`.
///
/// Declared `async` and offloaded to `spawn_blocking` so the blocking
/// inference or upload does not park a Tauri command worker thread.
#[tauri::command]
pub async fn transcribe_recording(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<TranscriptionResult, String> {
    let (transcriber_kind, api_key, language, local_model) = {
        let settings = state.settings.lock();
        (
            settings.transcriber.clone(),
            settings.openai_api_key.clone(),
            settings.transcription_language.clone(),
            settings.local_whisper_model.clone(),
        )
    };

    let audio_path = pick_audio_source(&session_dir)
        .ok_or_else(|| format!("no mic.wav or system.wav under {}", session_dir.display()))?;

    debug!(
        session = %session_dir.display(),
        audio = %audio_path.display(),
        language = %language,
        transcriber = %transcriber_kind,
        "starting transcription",
    );

    let language_for_task = language.clone();

    let transcript = match transcriber_kind.as_str() {
        "openai" => {
            if api_key.is_empty() {
                return Err("OpenAI API key is empty — add it in Settings → Transcription".into());
            }
            let audio_for_task = audio_path.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let transcriber = OpenAiTranscriber::new(api_key);
                let hint = (!language_for_task.is_empty() && language_for_task != "auto")
                    .then_some(language_for_task.as_str());
                transcriber.transcribe(&audio_for_task, hint)
            })
            .await
            .map_err(|e| format!("transcription task panicked: {e}"))?
            .map_err(|e| e.to_string())?
        }
        "local_whisper" => {
            let model = WhisperModel::from_id(&local_model).ok_or_else(|| {
                format!(
                    "unknown local Whisper model {:?} — pick one in Settings → Transcription",
                    local_model
                )
            })?;
            let store = WhisperModelStore::default_location();
            let status = store.status(model);
            if !status.present {
                return Err(format!(
                    "local Whisper model {:?} is not downloaded yet — open Settings → Transcription and download it first",
                    model.id()
                ));
            }
            let model_path = status.path;
            let audio_for_task = audio_path.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let transcriber = LocalWhisperTranscriber::new(model_path);
                let hint = (!language_for_task.is_empty() && language_for_task != "auto")
                    .then_some(language_for_task.as_str());
                transcriber.transcribe(&audio_for_task, hint)
            })
            .await
            .map_err(|e| format!("transcription task panicked: {e}"))?
            .map_err(|e| e.to_string())?
        }
        other => {
            return Err(format!(
                "unknown transcriber kind {other:?} — supported: \"openai\", \"local_whisper\""
            ));
        }
    };

    let transcript_path = session_dir.join(TRANSCRIPT_FILENAME);
    transcript
        .write_json(&transcript_path)
        .map_err(|e| e.to_string())?;

    info!(
        path = %transcript_path.display(),
        segments = transcript.segments.len(),
        "transcript saved",
    );

    Ok(TranscriptionResult {
        session_dir,
        transcript_path,
        transcript,
    })
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgressPayload {
    model_id: String,
    downloaded: u64,
    total: Option<u64>,
}

/// Report which whisper model is currently selected in settings and
/// whether it is present on disk.
#[tauri::command]
pub async fn whisper_model_status(
    state: State<'_, AppState>,
) -> Result<WhisperModelStatus, String> {
    let model_id = state.settings.lock().local_whisper_model.clone();
    let model = WhisperModel::from_id(&model_id)
        .ok_or_else(|| format!("unknown whisper model: {model_id}"))?;

    tauri::async_runtime::spawn_blocking(move || {
        let store = WhisperModelStore::default_location();
        store.status(model)
    })
    .await
    .map_err(|e| format!("whisper_model_status task panicked: {e}"))
}

/// Download a whisper model. Emits `whisper:model-download-progress`
/// events as bytes arrive so the Settings UI can show a live
/// progress bar.
#[tauri::command]
pub async fn ensure_whisper_model(
    app: AppHandle,
    model_id: String,
) -> Result<WhisperModelStatus, String> {
    let model = WhisperModel::from_id(&model_id)
        .ok_or_else(|| format!("unknown whisper model: {model_id}"))?;

    let app_for_task = app.clone();
    let model_id_for_event = model_id.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<WhisperModelStatus, String> {
        let store = WhisperModelStore::default_location();
        store.clean_partials();

        // Fast path: model already present.
        let status = store.status(model);
        if status.present {
            return Ok(status);
        }

        store
            .download(model, |progress| {
                let _ = app_for_task.emit(
                    DOWNLOAD_PROGRESS_EVENT,
                    DownloadProgressPayload {
                        model_id: model_id_for_event.clone(),
                        downloaded: progress.downloaded,
                        total: progress.total,
                    },
                );
            })
            .map_err(|e| e.to_string())?;

        Ok(store.status(model))
    })
    .await
    .map_err(|e| format!("ensure_whisper_model task panicked: {e}"))?
}

fn pick_audio_source(session_dir: &Path) -> Option<PathBuf> {
    let mic = session_dir.join("mic.wav");
    if mic.exists() {
        return Some(mic);
    }
    let system = session_dir.join("system.wav");
    if system.exists() {
        return Some(system);
    }
    None
}

/// Persist an edited transcript back to disk.
///
/// Same defence-in-depth as the other path-taking commands: the target
/// must canonicalize to a path under the user's recordings folder.
/// Writes via an atomic temp-file-rename so a crash mid-write cannot
/// corrupt the on-disk JSON.
#[tauri::command]
pub async fn save_transcript(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    transcript: Transcript,
) -> Result<PathBuf, String> {
    let output_dir = state.settings.lock().output_dir.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<PathBuf, String> {
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
                "refused to write {}: not under recordings folder {}",
                canon_target.display(),
                canon_root.display(),
            ));
        }

        let path = canon_target.join(TRANSCRIPT_FILENAME);
        let json = serde_json::to_string_pretty(&transcript)
            .map_err(|e| format!("could not serialize transcript: {e}"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| {
            format!(
                "could not write transcript temp file {}: {e}",
                tmp.display()
            )
        })?;
        std::fs::rename(&tmp, &path)
            .map_err(|e| format!("could not finalize transcript file {}: {e}", path.display()))?;
        info!(path = %path.display(), "transcript saved (edited)");
        Ok(path)
    })
    .await
    .map_err(|e| format!("save_transcript task panicked: {e}"))?
}

/// Read a previously persisted transcript for `session_dir`.
///
/// Validates that the target is under the user's configured recordings
/// folder — same defence-in-depth as `delete_recording`. Disk read +
/// JSON parse run on a blocking task.
#[tauri::command]
pub async fn read_transcript(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<Transcript, String> {
    let output_dir = state.settings.lock().output_dir.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<Transcript, String> {
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
                "refused to read {}: not under recordings folder {}",
                canon_target.display(),
                canon_root.display(),
            ));
        }

        let path = canon_target.join(TRANSCRIPT_FILENAME);
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("could not read transcript {}: {e}", path.display()))?;
        serde_json::from_str(&raw)
            .map_err(|e| format!("could not parse transcript {}: {e}", path.display()))
    })
    .await
    .map_err(|e| format!("read_transcript task panicked: {e}"))?
}
