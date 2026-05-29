//! Note folders ("Spaces"), GET-162. Thin Tauri wrappers over the
//! `attune_core::storage::folders` registry + per-note assignment.

use std::path::PathBuf;

use attune_core::storage::folders;
use tauri::State;
use tracing::info;

use crate::app::AppState;

/// List every folder (registry order, then in-use orphans).
#[tauri::command]
pub async fn list_folders(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || folders::list_folders(&output_dir))
        .await
        .map_err(|e| format!("list_folders task panicked: {e}"))
}

/// Create a folder. Idempotent on a case-insensitive name match.
#[tauri::command]
pub async fn create_folder(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<String>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    info!(name = %name, "create_folder");
    tauri::async_runtime::spawn_blocking(move || folders::create_folder(&output_dir, &name))
        .await
        .map_err(|e| format!("create_folder task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Rename a folder, rewriting every member note's assignment.
#[tauri::command]
pub async fn rename_folder(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<Vec<String>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    info!(from = %from, to = %to, "rename_folder");
    tauri::async_runtime::spawn_blocking(move || folders::rename_folder(&output_dir, &from, &to))
        .await
        .map_err(|e| format!("rename_folder task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Delete a folder, clearing the assignment on every member note.
#[tauri::command]
pub async fn delete_folder(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<String>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    info!(name = %name, "delete_folder");
    tauri::async_runtime::spawn_blocking(move || folders::delete_folder(&output_dir, &name))
        .await
        .map_err(|e| format!("delete_folder task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Assign (or clear, with `folder = None`) the folder a note belongs to.
/// The `session_dir` must lie under the recordings folder.
#[tauri::command]
pub async fn set_note_folder(
    state: State<'_, AppState>,
    session_dir: String,
    folder: Option<String>,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    let session = PathBuf::from(&session_dir);
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let canon = attune_core::paths::canonicalize_under(&output_dir, &session)
            .map_err(|e| e.to_string())?;
        folders::set_note_folder(&output_dir, &canon, folder.as_deref()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("set_note_folder task panicked: {e}"))?
}
