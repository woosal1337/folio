use std::path::{Path, PathBuf};
use std::time::Duration;

use folio_core::server::{RemoteStatus, ServerTokens};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{debug, warn};

use crate::app::AppState;

const INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Serialize)]
struct SyncProgress {
    session_dir: String,
    remote_status: String,
    transcript_written: bool,
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(INTERVAL).await;
            if let Err(e) = tick(&app).await {
                debug!(error = %e, "remote sync scheduler tick failed");
            }
        }
    });
}

async fn tick(app: &AppHandle) -> Result<(), String> {
    let (endpoint, output_dir, language, enabled) = {
        let state = app.state::<AppState>();
        let s = state.settings.lock();
        let enabled = s.transcriber == "remote_server"
            && s.remote_auto_upload
            && !s.privacy_mode
            && !s.remote_endpoint.trim().is_empty();
        (
            s.remote_endpoint.clone(),
            s.output_dir.clone(),
            s.transcription_language.clone(),
            enabled,
        )
    };
    if !enabled || !ServerTokens::has() {
        return Ok(());
    }

    let scan_dir = output_dir.clone();
    let pending = tauri::async_runtime::spawn_blocking(move || pending_sessions(&scan_dir))
        .await
        .map_err(|e| e.to_string())?;
    if pending.is_empty() {
        return Ok(());
    }

    let language = (!language.is_empty() && language != "auto").then_some(language);

    for session in pending {
        match crate::commands::server::run_sync(&endpoint, &session, language.as_deref()).await {
            Ok(outcome) => {
                let _ = app.emit(
                    "remote-sync-progress",
                    SyncProgress {
                        session_dir: session.to_string_lossy().to_string(),
                        remote_status: status_label(outcome.state.remote_status),
                        transcript_written: outcome.transcript_written,
                    },
                );
            }
            Err(e) => warn!(session = %session.display(), error = %e, "background sync failed"),
        }
    }
    Ok(())
}

fn status_label(status: RemoteStatus) -> String {
    match status {
        RemoteStatus::None => "none",
        RemoteStatus::Queued => "queued",
        RemoteStatus::Running => "running",
        RemoteStatus::Succeeded => "succeeded",
        RemoteStatus::Failed => "failed",
    }
    .to_string()
}

fn pending_sessions(output_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(output_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(Some(state)) = folio_core::server::sync_state::load(&path) {
            let terminal = matches!(
                state.remote_status,
                RemoteStatus::Succeeded | RemoteStatus::Failed
            );
            if !terminal {
                out.push(path);
            }
        }
    }
    out
}
