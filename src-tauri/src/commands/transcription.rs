//! Transcription: kick off an OpenAI Whisper run against a recorded
//! session, persist the result inside the session directory, and hand
//! the parsed transcript back to the UI.

use std::path::{Path, PathBuf};

use attune_core::storage::session::TRANSCRIPT_FILENAME;
use attune_core::transcription::{OpenAiTranscriber, Transcriber, Transcript, TranscriptionResult};
use tauri::State;
use tracing::{debug, info};

use crate::app::AppState;

/// Transcribe a previously recorded session.
///
/// Picks `mic.wav` if present (the canonical "me" channel), falls back
/// to `system.wav` otherwise. Writes the resulting transcript JSON to
/// `<session_dir>/transcript.json`.
///
/// Declared `async` and offloaded to `spawn_blocking` so the blocking
/// reqwest upload does not park a Tauri command worker thread. The
/// command can run for the full ~10 minute Whisper timeout without
/// affecting the responsiveness of other IPC calls.
#[tauri::command]
pub async fn transcribe_recording(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<TranscriptionResult, String> {
    let (api_key, language) = {
        let settings = state.settings.lock();
        if settings.transcriber != "openai" {
            return Err(format!(
                "transcriber is set to {:?}, only \"openai\" is supported in this build",
                settings.transcriber
            ));
        }
        (
            settings.openai_api_key.clone(),
            settings.transcription_language.clone(),
        )
    };

    if api_key.is_empty() {
        return Err("OpenAI API key is empty — add it in Settings → Transcription".into());
    }

    let audio_path = pick_audio_source(&session_dir)
        .ok_or_else(|| format!("no mic.wav or system.wav under {}", session_dir.display()))?;

    debug!(
        session = %session_dir.display(),
        audio = %audio_path.display(),
        language = %language,
        "starting OpenAI transcription",
    );

    let session_dir_for_task = session_dir.clone();
    let language_for_task = language.clone();

    // Move the blocking HTTP call onto a dedicated blocking thread so
    // the Tauri runtime stays free to dispatch other commands.
    let transcript = tauri::async_runtime::spawn_blocking(move || {
        let transcriber = OpenAiTranscriber::new(api_key);
        let hint = (!language_for_task.is_empty() && language_for_task != "auto")
            .then_some(language_for_task.as_str());
        transcriber.transcribe(&audio_path, hint)
    })
    .await
    .map_err(|e| format!("transcription task panicked: {e}"))?
    .map_err(|e| e.to_string())?;

    let transcript_path = session_dir_for_task.join(TRANSCRIPT_FILENAME);
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
