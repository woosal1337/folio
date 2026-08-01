use std::sync::Arc;

use folio_core::memory::{Memory, MemoryQuery, MemoryStore};
use tauri::State;
use tracing::debug;

use crate::app::AppState;

fn shared_store(state: &AppState) -> Result<Arc<MemoryStore>, String> {
    state.memory_store()
}

#[tauri::command]
pub async fn list_memories(
    state: State<'_, AppState>,
    query: MemoryQuery,
) -> Result<Vec<Memory>, String> {
    debug!(?query, "list_memories");
    let store = shared_store(&state)?;
    tauri::async_runtime::spawn_blocking(move || store.list(&query).map_err(|e| e.to_string()))
        .await
        .map_err(|e| format!("list_memories panicked: {e}"))?
}
