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
#[tauri::command]
pub fn transcribe_recording(
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

    let transcriber = OpenAiTranscriber::new(api_key);
    let language_hint = (!language.is_empty() && language != "auto").then_some(language.as_str());

    let transcript = transcriber
        .transcribe(&audio_path, language_hint)
        .map_err(|e| e.to_string())?;

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

/// Read a previously persisted transcript for `session_dir`.
///
/// Validates that the target is under the user's configured recordings
/// folder — same defence-in-depth as `delete_recording`.
#[tauri::command]
pub fn read_transcript(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<Transcript, String> {
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
            "refused to read {}: not under recordings folder {}",
            canon_target.display(),
            canon_root.display(),
        ));
    }

    let path = canon_target.join(TRANSCRIPT_FILENAME);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read transcript {}: {e}", path.display()))?;
    let transcript: Transcript = serde_json::from_str(&raw)
        .map_err(|e| format!("could not parse transcript {}: {e}", path.display()))?;
    Ok(transcript)
}
