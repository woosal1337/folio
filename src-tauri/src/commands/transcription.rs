//! Transcription: kick off an OpenAI Whisper run against a recorded
//! session, persist the result inside the session directory, and hand
//! the parsed transcript back to the UI.

use std::path::{Path, PathBuf};

use attune_core::transcription::{OpenAiTranscriber, Transcriber, TranscriptionResult};
use tauri::State;
use tracing::{debug, info};

use crate::app::AppState;

/// Where the transcript lives on disk, relative to the session dir.
const TRANSCRIPT_FILENAME: &str = "transcript.json";

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
