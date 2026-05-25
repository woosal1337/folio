//! Task CRUD commands backing the kanban + the `create_task` agent tool.
//!
//! The [`TaskStore`] is path-stateless, so each command grabs the
//! current `tasks_path` from settings, runs the disk operation inside
//! a blocking task (so the Tauri runtime stays free for other IPC
//! calls), and returns the result. There is no in-memory cache —
//! the frontend keeps its own copy and re-fetches after mutations.

use attune_core::storage::{NewTask, Task, TaskStatus, TaskStore, TaskUpdate};
use std::path::PathBuf;
use tauri::State;
use tracing::{debug, info};

use crate::app::AppState;

/// Snapshot the current `tasks_path` from settings. Held briefly under
/// the settings lock and copied so the disk work below doesn't keep
/// the mutex.
fn current_tasks_path(state: &AppState) -> PathBuf {
    state.settings.lock().tasks_path.clone()
}

/// List every persisted task in insertion order. Missing/empty/malformed
/// files yield `[]` so a corrupted tasks.json never blocks the UI.
#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let path = current_tasks_path(&state);
    debug!(path = %path.display(), "list_tasks");
    tauri::async_runtime::spawn_blocking(move || TaskStore::new(path).list())
        .await
        .map_err(|e| format!("list_tasks task panicked: {e}"))
}

/// Create a new task. The store generates id + timestamps; the caller
/// supplies the title and any optional metadata (owner, due, notes,
/// source recording back-link, `agent_origin` flag).
#[tauri::command]
pub async fn create_task(state: State<'_, AppState>, task: NewTask) -> Result<Task, String> {
    let path = current_tasks_path(&state);
    info!(title = %task.title, "create_task");
    tauri::async_runtime::spawn_blocking(move || TaskStore::new(path).create(task))
        .await
        .map_err(|e| format!("create_task task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Patch an existing task. Fields left `None` are unchanged; an empty
/// string clears a nullable field. Returns the updated task.
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

/// Delete a task. Idempotent: deleting an unknown id is a no-op + Ok.
#[tauri::command]
pub async fn delete_task(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let path = current_tasks_path(&state);
    info!(id = %id, "delete_task");
    tauri::async_runtime::spawn_blocking(move || TaskStore::new(path).delete(&id))
        .await
        .map_err(|e| format!("delete_task task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Convenience for the kanban's drag-and-drop. Equivalent to
/// `update_task` with only the status set, but keeps the IPC surface
/// expressive for the frontend's optimistic-update layer.
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
