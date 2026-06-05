use folio_core::diarization::{DiarizationModel, DiarizationModelStatus, DiarizationModelStore};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

const DOWNLOAD_PROGRESS_EVENT: &str = "diarization:model-download-progress";

#[derive(Debug, Clone, Serialize)]
struct DownloadProgressPayload {
    model_id: String,
    downloaded: u64,
    total: Option<u64>,
}

#[tauri::command]
pub async fn diarization_model_status() -> Result<Vec<DiarizationModelStatus>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let store = DiarizationModelStore::default_location();
        store.status_all()
    })
    .await
    .map_err(|e| format!("diarization_model_status task panicked: {e}"))
}

#[tauri::command]
pub async fn ensure_diarization_models(
    app: AppHandle,
) -> Result<Vec<DiarizationModelStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<DiarizationModelStatus>, String> {
        let store = DiarizationModelStore::default_location();
        store.clean_partials();

        for model in DiarizationModel::ALL.iter().copied() {
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
