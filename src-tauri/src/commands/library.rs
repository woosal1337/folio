use std::path::PathBuf;

use folio_core::storage::search::{search_notes, NoteSearchHit};
use folio_core::storage::{scan_recordings, RecordingSummary};
use tauri::State;
#[cfg(not(target_os = "macos"))]
use tracing::warn;
use tracing::{debug, info};

use crate::app::AppState;

#[tauri::command]
pub async fn list_recordings(state: State<'_, AppState>) -> Result<Vec<RecordingSummary>, String> {
    debug!("list_recordings");
    let output_dir = state.settings.lock().output_dir.clone();
    let active_session_dir = state
        .session
        .lock()
        .as_ref()
        .map(|s| s.session_dir().clone());

    tauri::async_runtime::spawn_blocking(move || {
        let mut list = scan_recordings(&output_dir);
        if let Some(active) = active_session_dir {
            list.retain(|entry| entry.session_dir != active);
        }
        list
    })
    .await
    .map_err(|e| format!("list_recordings task panicked: {e}"))
}

#[tauri::command]
pub async fn search_note_content(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<NoteSearchHit>, String> {
    debug!(query = %query, "search_note_content");
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || search_notes(&output_dir, &query))
        .await
        .map_err(|e| format!("search_note_content task panicked: {e}"))
}

#[tauri::command]
pub async fn export_note_markdown(
    state: State<'_, AppState>,
    session_dir: String,
) -> Result<String, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let session = PathBuf::from(&session_dir);
    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let canon = folio_core::paths::canonicalize_under(&output_dir, &session)
            .map_err(|e| e.to_string())?;
        let path = folio_core::storage::note_export::write_markdown(&output_dir, &canon)
            .map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| format!("export_note_markdown task panicked: {e}"))?
    .inspect(|path| info!(path = %path, "note exported to markdown"))
}

#[tauri::command]
pub async fn delete_recording(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<(), String> {
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
                "refused to delete {}: not under recordings folder {}",
                canon_target.display(),
                canon_root.display(),
            ));
        }
        if canon_target == canon_root {
            return Err("refused to delete the recordings folder itself".into());
        }
        std::fs::remove_dir_all(&canon_target)
            .map_err(|e| format!("could not delete {}: {e}", canon_target.display()))?;
        Ok(canon_target)
    })
    .await
    .map_err(|e| format!("delete_recording task panicked: {e}"))?
    .map(|path| {
        info!(path = %path.display(), "recording deleted");
    })
}

#[tauri::command]
pub async fn get_recording(
    state: State<'_, AppState>,
    label: String,
) -> Result<Option<RecordingSummary>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let active_session_dir = state
        .session
        .lock()
        .as_ref()
        .map(|s| s.session_dir().clone());

    tauri::async_runtime::spawn_blocking(move || {
        let list = scan_recordings(&output_dir);
        list.into_iter()
            .find(|r| r.label == label && Some(&r.session_dir) != active_session_dir.as_ref())
    })
    .await
    .map_err(|e| format!("get_recording task panicked: {e}"))
}

#[tauri::command]
pub async fn reveal_in_finder(state: State<'_, AppState>, path: PathBuf) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let canon =
            folio_core::paths::canonicalize_under(&output_dir, &path).map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&canon)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = canon;
            warn!("reveal_in_finder not implemented on this platform");
            Ok(())
        }
    })
    .await
    .map_err(|e| format!("reveal_in_finder task panicked: {e}"))?
}

#[tauri::command]
pub async fn share_paths(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    paths: Vec<PathBuf>,
) -> Result<(), String> {
    info!("share_paths: {} item(s)", paths.len());
    let output_dir = state.settings.lock().output_dir.clone();
    let mut canon_paths: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for p in &paths {
        let canon =
            folio_core::paths::canonicalize_under(&output_dir, p).map_err(|e| e.to_string())?;
        canon_paths.push(canon);
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(crate::app::share_sheet::share_paths(&canon_paths));
    })
    .map_err(|e| e.to_string())?;
    rx.await
        .map_err(|_| "share_paths: main-thread task dropped".to_string())?
}
