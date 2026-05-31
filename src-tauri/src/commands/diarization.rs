//! Diarization model management.
//!
//! Reports the on-disk status of the two ONNX models the speaker-
//! diarization pipeline needs (pyannote segmentation + WeSpeaker
//! embedding) and downloads whichever are missing, streaming progress to
//! the Settings UI. Mirrors the whisper model commands in
//! `transcription.rs` so the Settings download affordance is identical.

use attune_core::diarization::{DiarizationModel, DiarizationModelStatus, DiarizationModelStore};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Tauri event channel for live diarization-model download progress.
const DOWNLOAD_PROGRESS_EVENT: &str = "diarization:model-download-progress";

#[derive(Debug, Clone, Serialize)]
struct DownloadProgressPayload {
    model_id: String,
    downloaded: u64,
    total: Option<u64>,
}

/// On-disk status of every diarization model. Diarization is ready when
/// each entry's `present` is true.
#[tauri::command]
pub async fn diarization_model_status() -> Result<Vec<DiarizationModelStatus>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let store = DiarizationModelStore::default_location();
        store.status_all()
    })
    .await
    .map_err(|e| format!("diarization_model_status task panicked: {e}"))
}

/// Download whichever diarization models are missing, in sequence,
/// emitting `diarization:model-download-progress` as bytes arrive so the
/// Settings UI can show a live progress bar. Already-present models are
/// skipped. Each download is sha256-verified against the pinned hash; a
/// mismatch aborts with an error and leaves the bad file unwritten (a
/// malformed ONNX would otherwise crash the sherpa runtime). Returns the
/// final status of all models.
#[tauri::command]
pub async fn ensure_diarization_models(
    app: AppHandle,
) -> Result<Vec<DiarizationModelStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<DiarizationModelStatus>, String> {
        let store = DiarizationModelStore::default_location();
        store.clean_partials();

        for model in DiarizationModel::ALL.iter().copied() {
            // Fast path: skip models already on disk.
            if store.status(model).present {
                continue;
            }
            let model_id = model.id().to_string();
            store
                .download(model, |progress| {
                    let _ = app.emit(
                        DOWNLOAD_PROGRESS_EVENT,
                        DownloadProgressPayload {
                            model_id: model_id.clone(),
                            downloaded: progress.downloaded,
                            total: progress.total,
                        },
                    );
                })
                .map_err(|e| e.to_string())?;
        }

        Ok(store.status_all())
    })
    .await
    .map_err(|e| format!("ensure_diarization_models task panicked: {e}"))?
}
