use folio_core::storage::{Task, TaskStore};
use std::path::PathBuf;
use tauri::State;
use tracing::debug;

use crate::app::AppState;

fn current_tasks_path(state: &AppState) -> PathBuf {
    state.settings.lock().tasks_path.clone()
}

#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let path = current_tasks_path(&state);
    debug!(path = %path.display(), "list_tasks");
    tauri::async_runtime::spawn_blocking(move || TaskStore::new(path).list())
        .await
        .map_err(|e| format!("list_tasks task panicked: {e}"))
}
