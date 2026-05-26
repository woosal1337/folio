//! Transcription: kick off an OpenAI Whisper run against a recorded
//! session, persist the result inside the session directory, and hand
//! the parsed transcript back to the UI.

use std::path::{Path, PathBuf};

use attune_core::storage::session::TRANSCRIPT_FILENAME;
use attune_core::transcription::{
    ChannelTranscript, LocalWhisperTranscriber, OpenAiTranscriber, SessionTranscript, Transcriber,
    Transcript, TranscriptionResult, WhisperModel, WhisperModelStatus, WhisperModelStore,
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
/// Transcribes both `mic.wav` and `system.wav` (whichever are present)
/// independently. The mic track captures the user ("You") and the system
/// track captures the rest of the meeting ("Others"). Each channel
/// produces its own `ChannelTranscript`, both are packed into a single
/// `SessionTranscript` and persisted to `<session_dir>/transcript.json`.
/// A channel that fails to transcribe is logged and skipped; the
/// command succeeds as long as at least one channel produced output.
///
/// Declared `async` and each channel runs on `spawn_blocking` so the
/// blocking inference / upload does not park a Tauri command worker.
#[tauri::command]
pub async fn transcribe_recording(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<TranscriptionResult, String> {
    let (transcriber_kind, settings_api_key, settings_language, local_model) = {
        let settings = state.settings.lock();
        (
            settings.transcriber.clone(),
            settings.openai_api_key.clone(),
            settings.transcription_language.clone(),
            settings.local_whisper_model.clone(),
        )
    };

    let api_key = match attune_core::llm::KeyStore::get(attune_core::llm::ProviderId::OpenAi) {
        Ok(Some(stored)) if !stored.trim().is_empty() => stored,
        _ => settings_api_key,
    };

    // Per-recording language override (v2 finding 046 / GET-89). When
    // `<session_dir>/language.txt` exists and is non-empty, its first
    // line wins over the global setting. The file is written by the
    // `set_recording_language` command below from the Library UI's
    // language picker; an empty file means "fall back to global".
    let language = read_session_language_override(&session_dir).unwrap_or(settings_language);

    // Resolve the local model up front so both channels see the same
    // resolved path — keeps error messages consistent across channels.
    let local_model_path = if transcriber_kind == "local_whisper" {
        let model = WhisperModel::from_id(&local_model).ok_or_else(|| {
            format!(
                "unknown local Whisper model {local_model:?} — pick one in Settings → Transcription"
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
        Some(status.path)
    } else {
        if transcriber_kind == "openai" && api_key.is_empty() {
            return Err("OpenAI API key is empty — add it in Settings → Transcription".into());
        }
        None
    };

    let sources = collect_audio_sources(&session_dir);
    if sources.is_empty() {
        return Err(format!(
            "no mic.wav or system.wav under {}",
            session_dir.display()
        ));
    }

    debug!(
        session = %session_dir.display(),
        channels = sources.len(),
        language = %language,
        transcriber = %transcriber_kind,
        "starting transcription (multi-channel)",
    );

    let mut channels: Vec<ChannelTranscript> = Vec::new();
    let mut channel_errors: Vec<String> = Vec::new();

    for source in sources {
        let kind = transcriber_kind.clone();
        let key = api_key.clone();
        let model_path = local_model_path.clone();
        let language_for_task = language.clone();
        let label = source.channel.clone();
        let path = source.path.clone();

        let result: Result<Transcript, String> = tauri::async_runtime::spawn_blocking(move || {
            let hint = (!language_for_task.is_empty() && language_for_task != "auto")
                .then_some(language_for_task.clone());
            match kind.as_str() {
                "openai" => {
                    let t = OpenAiTranscriber::new(key);
                    t.transcribe(&path, hint.as_deref())
                        .map_err(|e| e.to_string())
                }
                "local_whisper" => {
                    let p = model_path.expect("local model path resolved above");
                    let t = LocalWhisperTranscriber::new(p);
                    t.transcribe(&path, hint.as_deref())
                        .map_err(|e| e.to_string())
                }
                other => Err(format!(
                    "unknown transcriber kind {other:?} — supported: \"openai\", \"local_whisper\""
                )),
            }
        })
        .await
        .map_err(|e| format!("transcription task panicked on channel {label}: {e}"))?;

        match result {
            Ok(transcript) => {
                info!(
                    channel = %label,
                    segments = transcript.segments.len(),
                    "channel transcribed",
                );
                channels.push(ChannelTranscript {
                    channel: label,
                    language: transcript.language,
                    segments: transcript.segments,
                });
            }
            Err(e) => {
                tracing::warn!(channel = %label, error = %e, "channel transcription failed");
                channel_errors.push(format!("{label}: {e}"));
            }
        }
    }

    if channels.is_empty() {
        return Err(format!(
            "all channels failed to transcribe: {}",
            channel_errors.join("; ")
        ));
    }

    let session_transcript = SessionTranscript { channels };
    let transcript_path = session_dir.join(TRANSCRIPT_FILENAME);
    session_transcript
        .write_json(&transcript_path)
        .map_err(|e| e.to_string())?;

    let total_segments: usize = session_transcript
        .channels
        .iter()
        .map(|c| c.segments.len())
        .sum();
    info!(
        path = %transcript_path.display(),
        channels = session_transcript.channels.len(),
        total_segments,
        "transcript saved (multi-channel)",
    );

    Ok(TranscriptionResult {
        session_dir,
        transcript_path,
        session_transcript,
    })
}

struct AudioSource {
    channel: String,
    path: PathBuf,
}

/// Discover the audio channels present in `session_dir`. We always try
/// both mic and system; whichever WAVs exist on disk are returned in a
/// stable order (mic first, then system) so the resulting transcript
/// reads top-to-bottom as "You", then "Others" by convention.
fn collect_audio_sources(session_dir: &Path) -> Vec<AudioSource> {
    let mut out = Vec::new();
    let mic = session_dir.join("mic.wav");
    if mic.exists() {
        out.push(AudioSource {
            channel: "mic".to_string(),
            path: mic,
        });
    }
    let system = session_dir.join("system.wav");
    if system.exists() {
        out.push(AudioSource {
            channel: "system".to_string(),
            path: system,
        });
    }
    out
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

/// Persist an edited transcript bundle back to disk.
///
/// Same defence-in-depth as the other path-taking commands: the target
/// must canonicalize to a path under the user's recordings folder.
/// Writes via an atomic temp-file-rename so a crash mid-write cannot
/// corrupt the on-disk JSON.
#[tauri::command]
pub async fn save_transcript(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    transcript: SessionTranscript,
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
/// JSON parse run on a blocking task. Old single-channel transcripts
/// are silently upgraded to the new multi-channel shape by
/// [`SessionTranscript::read_json`].
#[tauri::command]
pub async fn read_transcript(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<SessionTranscript, String> {
    let output_dir = state.settings.lock().output_dir.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<SessionTranscript, String> {
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
        SessionTranscript::read_json(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("read_transcript task panicked: {e}"))?
}

/// Locate an evidence span inside a session's transcript and return
/// the channel / segment / timestamp it lives in. v2 finding 038 /
/// GET-41. Used by the inbox + memory + task UIs to backlink a claim
/// to the exact second of audio it came from.
#[tauri::command]
pub async fn locate_transcript_span(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    span: String,
) -> Result<Option<attune_core::transcription::locate::TranscriptHit>, String> {
    let output_dir = state.settings.lock().output_dir.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        let canon_root = std::fs::canonicalize(&output_dir).map_err(|e| e.to_string())?;
        let canon_target = std::fs::canonicalize(&session_dir).map_err(|e| e.to_string())?;
        if !canon_target.starts_with(&canon_root) {
            return Err(format!(
                "refused: {} not under recordings folder",
                canon_target.display()
            ));
        }
        let path = canon_target.join(TRANSCRIPT_FILENAME);
        let transcript = SessionTranscript::read_json(&path).map_err(|e| e.to_string())?;
        Ok(attune_core::transcription::locate::locate_span(&transcript, &span))
    })
    .await
    .map_err(|e| format!("locate_transcript_span task panicked: {e}"))?
}

const LANGUAGE_OVERRIDE_FILE: &str = "language.txt";

/// Read `<session_dir>/language.txt` if present and return a trimmed,
/// non-empty language code. Treats missing / empty files as 'no
/// override' so the caller falls through to the global setting.
/// v2 finding 046 / GET-89.
fn read_session_language_override(session_dir: &Path) -> Option<String> {
    let path = session_dir.join(LANGUAGE_OVERRIDE_FILE);
    let raw = std::fs::read_to_string(&path).ok()?;
    let trimmed = raw.lines().next()?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Read the per-recording language override the UI displays under the
/// Library row's language chip. Returns `None` when no override file
/// exists.
#[tauri::command]
pub async fn get_recording_language(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<Option<String>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Option<String>, String> {
        let dir = attune_core::paths::canonicalize_under(&output_dir, &session_dir)
            .map_err(|e| e.to_string())?;
        Ok(read_session_language_override(&dir))
    })
    .await
    .map_err(|e| format!("get_recording_language task panicked: {e}"))?
}

/// Set or clear the per-recording language override. Empty / null
/// `language` deletes the override file so the global setting wins
/// again. v2 finding 046 / GET-89.
#[tauri::command]
pub async fn set_recording_language(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    language: Option<String>,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let session_dir = attune_core::paths::canonicalize_under(&output_dir, &session_dir)
            .map_err(|e| e.to_string())?;
        let path = session_dir.join(LANGUAGE_OVERRIDE_FILE);
        match language.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(code) => {
                std::fs::write(&path, format!("{code}\n"))
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                info!(path = %path.display(), language = %code, "session language override saved");
            }
            None => match std::fs::remove_file(&path) {
                Ok(()) => info!(path = %path.display(), "session language override cleared"),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(format!(
                        "could not clear override at {}: {e}",
                        path.display()
                    ))
                }
            },
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("set_recording_language task panicked: {e}"))?
}
