use std::path::{Path, PathBuf};

use folio_core::storage::session::TRANSCRIPT_FILENAME;
use folio_core::transcription::{
    ChannelTranscript, LocalWhisperTranscriber, OpenAiTranscriber, SessionTranscript, Transcriber,
    Transcript, TranscriptionResult, WhisperModel, WhisperModelStatus, WhisperModelStore,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracing::{debug, info};

use crate::app::AppState;

const DOWNLOAD_PROGRESS_EVENT: &str = "whisper:model-download-progress";

#[tauri::command]
pub async fn transcribe_recording(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<TranscriptionResult, String> {
    let (transcriber_kind, settings_language, local_model, output_dir) = {
        let settings = state.settings.lock();
        (
            settings.transcriber.clone(),
            settings.transcription_language.clone(),
            settings.local_whisper_model.clone(),
            settings.output_dir.clone(),
        )
    };

    let session_dir = folio_core::paths::canonicalize_under(&output_dir, &session_dir)
        .map_err(|e| format!("invalid session directory: {e}"))?;

    let api_key = folio_core::llm::KeyStore::get(folio_core::llm::ProviderId::OpenAi)
        .map_err(|e| format!("could not read OpenAI key from Keychain: {e}"))?
        .unwrap_or_default();

    let language = read_session_language_override(&session_dir).unwrap_or(settings_language);

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
        let sidecar_path = source.vad_sidecar.clone();

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
            Ok(mut transcript) => {
                if let Some(side) = sidecar_path.as_ref() {
                    match std::fs::read(side)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| {
                            serde_json::from_slice::<folio_core::audio::vad_filter::VadSidecar>(
                                &bytes,
                            )
                            .map_err(|e| e.to_string())
                        }) {
                        Ok(sidecar) => {
                            for seg in &mut transcript.segments {
                                seg.start_seconds =
                                    folio_core::audio::vad_filter::remap_cut_seconds_to_original(
                                        &sidecar,
                                        seg.start_seconds,
                                    );
                                seg.end_seconds =
                                    folio_core::audio::vad_filter::remap_cut_seconds_to_original(
                                        &sidecar,
                                        seg.end_seconds,
                                    );
                            }
                        }
                        Err(e) => tracing::warn!(
                            channel = %label,
                            error = %e,
                            "vad sidecar unreadable; timestamps left in cut-audio timeline"
                        ),
                    }
                }
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

    vad_sidecar: Option<PathBuf>,
}

fn collect_audio_sources(session_dir: &Path) -> Vec<AudioSource> {
    let mut out = Vec::new();
    for channel in &["mic", "system"] {
        let raw = session_dir.join(format!("{channel}.wav"));
        if !raw.exists() {
            continue;
        }
        let speech = session_dir.join(format!("{channel}.speech.wav"));
        let sidecar = session_dir.join(format!("{channel}.vad.json"));
        if speech.exists() && sidecar.exists() {
            out.push(AudioSource {
                channel: (*channel).to_string(),
                path: speech,
                vad_sidecar: Some(sidecar),
            });
        } else {
            out.push(AudioSource {
                channel: (*channel).to_string(),
                path: raw,
                vad_sidecar: None,
            });
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgressPayload {
    model_id: String,
    downloaded: u64,
    total: Option<u64>,
}

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
        folio_core::storage::atomic_write::atomic_write(&path, json.as_bytes())
            .map_err(|e| format!("could not write transcript file {}: {e}", path.display()))?;
        info!(path = %path.display(), "transcript saved (edited)");
        Ok(path)
    })
    .await
    .map_err(|e| format!("save_transcript task panicked: {e}"))?
}

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

#[tauri::command]
pub async fn locate_note_evidence(
    state: State<'_, AppState>,
    session_dir: PathBuf,
    line: String,
) -> Result<Option<folio_core::transcription::locate::TranscriptHit>, String> {
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
        Ok(folio_core::transcription::locate::locate_fuzzy(
            &transcript,
            &line,
        ))
    })
    .await
    .map_err(|e| format!("locate_note_evidence task panicked: {e}"))?
}

const LANGUAGE_OVERRIDE_FILE: &str = "language.txt";

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

#[tauri::command]
pub async fn diarize_session(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<bool, String> {
    let (diarization_enabled, output_dir) = {
        let s = state.settings.lock();
        (s.diarization_enabled, s.output_dir.clone())
    };
    if !diarization_enabled {
        return Ok(false);
    }
    let session_dir = {
        let target = session_dir;
        tauri::async_runtime::spawn_blocking(move || {
            folio_core::paths::canonicalize_under(&output_dir, &target).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("canonicalize panicked: {e}"))??
    };

    let did_label = tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        use folio_core::diarization::{
            anchor_self_from_session, identify_session_speakers, DiarizationError,
            DiarizationOptions,
        };
        use folio_core::speaker_memory::{self, SpeakerRegistry};
        use folio_core::storage::session::TRANSCRIPT_FILENAME;

        let transcript_path = session_dir.join(TRANSCRIPT_FILENAME);
        let mut session_transcript = SessionTranscript::read_json(&transcript_path)
            .map_err(|e| format!("read transcript: {e}"))?;

        let opts = DiarizationOptions::default();
        let mut registry = speaker_memory::load_default().unwrap_or_else(|e| {
            tracing::warn!(error = %e, "speaker registry load failed; using empty");
            SpeakerRegistry::new()
        });

        let ident = match identify_session_speakers(
            &session_dir,
            &mut session_transcript,
            &opts,
            &registry,
        ) {
            Ok(i) => i,
            Err(DiarizationError::ModelsNotDownloaded) => {
                info!("diarization skipped: models not downloaded");
                return Ok(false);
            }
            Err(e) => {
                tracing::warn!(error = %e, "diarization failed; leaving channel labels");
                return Ok(false);
            }
        };
        info!(
            speakers = ident.outcome.num_speakers,
            labeled = ident.outcome.num_labeled,
            segments = ident.outcome.num_segments,
            "diarization complete",
        );

        if let Err(e) = ident.speakers.write(&session_dir) {
            tracing::warn!(error = %e, "could not write speakers sidecar");
        }

        session_transcript
            .write_json(&transcript_path)
            .map_err(|e| format!("write transcript: {e}"))?;

        match anchor_self_from_session(&mut registry, &session_dir, &opts) {
            Ok(true) => {
                if let Err(e) = speaker_memory::save_default(&registry) {
                    tracing::warn!(error = %e, "saving speaker registry failed");
                }
            }
            Ok(false) => {}
            Err(e) => tracing::warn!(error = %e, "anchoring self voice failed"),
        }

        Ok(true)
    })
    .await
    .map_err(|e| format!("diarize_session task panicked: {e}"))??;

    Ok(did_label)
}
