use folio_core::storage::{NewTask, Task, TaskStatus, TaskStore, TaskUpdate};
use std::path::PathBuf;
use tauri::State;
use tracing::{debug, info};

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

#[tauri::command]
pub async fn create_task(state: State<'_, AppState>, task: NewTask) -> Result<Task, String> {
    let path = current_tasks_path(&state);
    info!(title = %task.title, "create_task");
    tauri::async_runtime::spawn_blocking(move || TaskStore::new(path).create(task))
        .await
        .map_err(|e| format!("create_task task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_task(
    state: State<'_, AppState>,
    id: String,
    patch: TaskUpdate,
) -> Result<Task, String> {
    let path = current_tasks_path(&state);
    info!(id = %id, "update_task");
    tauri::async_runtime::spawn_blocking(move || TaskStore::new(path).update(&id, patch))
        .await
        .map_err(|e| format!("update_task task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_task(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let path = current_tasks_path(&state);
    info!(id = %id, "delete_task");
    tauri::async_runtime::spawn_blocking(move || TaskStore::new(path).delete(&id))
        .await
        .map_err(|e| format!("delete_task task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_task_status(
    state: State<'_, AppState>,
    id: String,
    status: TaskStatus,
) -> Result<Task, String> {
    let path = current_tasks_path(&state);
    info!(id = %id, ?status, "set_task_status");
    tauri::async_runtime::spawn_blocking(move || TaskStore::new(path).set_status(&id, status))
        .await
        .map_err(|e| format!("set_task_status task panicked: {e}"))?
        .map_err(|e| e.to_string())
}
