//! Persisted chat threads + Recents (GET-167). Thin Tauri wrappers over
//! `attune_core::storage::chats`.

use attune_core::storage::chats::{self, ChatThread};
use tauri::State;
use tracing::info;

use crate::app::AppState;

/// List persisted chat threads, newest first. Optional `scope`
/// ("library" / "note") and `session_dir` filters narrow to one surface.
#[tauri::command]
pub async fn list_chat_threads(
    state: State<'_, AppState>,
    scope: Option<String>,
    session_dir: Option<String>,
) -> Result<Vec<ChatThread>, String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        chats::list_threads(&output_dir, scope.as_deref(), session_dir.as_deref())
    })
    .await
    .map_err(|e| format!("list_chat_threads task panicked: {e}"))
}

/// Save (upsert) a chat thread.
#[tauri::command]
pub async fn save_chat_thread(
    state: State<'_, AppState>,
    thread: ChatThread,
) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    info!(id = %thread.id, scope = %thread.scope, "save_chat_thread");
    tauri::async_runtime::spawn_blocking(move || chats::save_thread(&output_dir, &thread))
        .await
        .map_err(|e| format!("save_chat_thread task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Delete a chat thread by id.
#[tauri::command]
pub async fn delete_chat_thread(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let output_dir = state.settings.lock().output_dir.clone();
    tauri::async_runtime::spawn_blocking(move || chats::delete_thread(&output_dir, &id))
        .await
        .map_err(|e| format!("delete_chat_thread task panicked: {e}"))?
        .map_err(|e| e.to_string())
}
