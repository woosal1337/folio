//! Memory CRUD commands backing the /memory page + the `remember`
//! and `search_memory` agent tools.
//!
//! Disk + embedding work runs inside `spawn_blocking` so the IPC
//! runtime stays free. Embedding calls go to OpenAI; if the API
//! errors (no key, transient), the memory is still persisted to
//! files + FTS5 — only the vec index slot is empty, which the
//! hybrid retrieval path falls back from gracefully.
//!
//! v2 roadmap finding R14: all commands share a single
//! [`attune_core::memory::MemoryStore`] cached in [`AppState`] (one
//! SQLite connection per process), not one open per IPC.

use std::sync::Arc;

use attune_core::llm::{KeyStore, ProviderId};
use attune_core::memory::{
    EmbeddingClient, Memory, MemoryKind, MemoryQuery, MemoryStore, MemoryUpdate, NewMemory,
};
use tauri::State;
use tracing::{debug, info, warn};

use crate::app::AppState;

/// Read the OpenAI key from the keyring on a blocking task.
async fn openai_key_opt() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| KeyStore::get(ProviderId::OpenAi))
        .await
        .map_err(|e| format!("keystore task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

/// Embed `text` if an OpenAI key is configured; otherwise return
/// None so the upsert proceeds without a vector. Embedding failures
/// are logged but not surfaced as errors — the memory still lands.
async fn embed_if_possible(text: &str) -> Option<Vec<f32>> {
    let key = match openai_key_opt().await {
        Ok(Some(k)) => k,
        Ok(None) => return None,
        Err(e) => {
            warn!(error = %e, "could not read openai key for memory embedding");
            return None;
        }
    };
    let client = EmbeddingClient::new(key);
    match client.embed(text).await {
        Ok(v) => Some(v),
        Err(e) => {
            warn!(error = %e, "memory embedding failed; storing without vector");
            None
        }
    }
}

/// Resolve the shared memory store handle. Lazy (opens on first
/// IPC, reused thereafter); reopens if `memory_dir` was edited.
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

#[tauri::command]
pub async fn get_memory(state: State<'_, AppState>, id: String) -> Result<Option<Memory>, String> {
    let store = shared_store(&state)?;
    tauri::async_runtime::spawn_blocking(move || store.get(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| format!("get_memory panicked: {e}"))?
}

#[tauri::command]
pub async fn create_memory(
    state: State<'_, AppState>,
    memory: NewMemory,
) -> Result<Memory, String> {
    info!(kind = %memory.kind.as_str(), "create_memory");
    let store = shared_store(&state)?;
    let content_for_embed = memory.content.clone();

    // Write via the shared store; conflict resolution may mutate
    // previously-current rows (supersede). All on a blocking thread.
    let written = {
        let store = store.clone();
        tauri::async_runtime::spawn_blocking(move || {
            store
                .create(memory)
                .map(|o| o.into_memory())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("create_memory panicked: {e}"))??
    };

    // Best-effort embedding upsert so the vec index can serve future
    // searches. Failure is logged but not fatal.
    if let Some(embedding) = embed_if_possible(&content_for_embed).await {
        let store = store.clone();
        let m = written.clone();
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            store
                .upsert_with_embedding(&m, &embedding)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("embedding upsert panicked: {e}"))??;
    }
    Ok(written)
}

#[tauri::command]
pub async fn update_memory(
    state: State<'_, AppState>,
    id: String,
    patch: MemoryUpdate,
) -> Result<Memory, String> {
    info!(id = %id, "update_memory");
    let store = shared_store(&state)?;
    let content_changed = patch.content.is_some();
    let updated = {
        let store = store.clone();
        let id = id.clone();
        tauri::async_runtime::spawn_blocking(move || {
            store.update(&id, patch).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("update_memory panicked: {e}"))??
    };
    if content_changed {
        if let Some(embedding) = embed_if_possible(&updated.content).await {
            let store = store.clone();
            let m = updated.clone();
            tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
                store
                    .upsert_with_embedding(&m, &embedding)
                    .map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| format!("update embedding panicked: {e}"))??;
        }
    }
    Ok(updated)
}

#[tauri::command]
pub async fn delete_memory(state: State<'_, AppState>, id: String) -> Result<Memory, String> {
    info!(id = %id, "delete_memory");
    let store = shared_store(&state)?;
    tauri::async_runtime::spawn_blocking(move || store.soft_delete(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| format!("delete_memory panicked: {e}"))?
}

#[tauri::command]
pub async fn purge_memory(state: State<'_, AppState>, id: String) -> Result<(), String> {
    info!(id = %id, "purge_memory");
    let store = shared_store(&state)?;
    tauri::async_runtime::spawn_blocking(move || store.purge(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| format!("purge_memory panicked: {e}"))?
}

#[tauri::command]
pub async fn pin_memory(
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<Memory, String> {
    info!(id = %id, pinned, "pin_memory");
    let store = shared_store(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        store
            .update(
                &id,
                MemoryUpdate {
                    pinned: Some(pinned),
                    ..MemoryUpdate::default()
                },
            )
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("pin_memory panicked: {e}"))?
}

#[tauri::command]
pub async fn search_memories(
    state: State<'_, AppState>,
    query: String,
    kinds: Vec<MemoryKind>,
    limit: Option<usize>,
) -> Result<Vec<Memory>, String> {
    let store = shared_store(&state)?;
    let embedding = embed_if_possible(&query).await;
    let limit = limit.unwrap_or(10);
    tauri::async_runtime::spawn_blocking(move || {
        store
            .search(&query, embedding.as_deref(), &kinds, limit)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("search_memories panicked: {e}"))?
}

/// Resolve the absolute path on disk for a memory's markdown page.
/// Returns `None` if the memory exists in the index but the file is
/// missing (which would be a drift the user should Reindex from).
/// Used by the frontend to build `obsidian://` deep-links + Copy-path
/// affordances (v2 roadmap finding 069).
#[tauri::command]
pub async fn memory_file_path(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<String>, String> {
    let store = shared_store(&state)?;
    tauri::async_runtime::spawn_blocking(move || -> Result<Option<String>, String> {
        let Some(memory) = store.get(&id).map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let path = attune_core::memory::path_for(store.dir(), &memory);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| format!("memory_file_path panicked: {e}"))?
}

#[tauri::command]
pub async fn rebuild_memory_index(state: State<'_, AppState>) -> Result<usize, String> {
    info!("rebuild_memory_index");
    let store = shared_store(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        // Embeddings are not refetched during rebuild — they survive
        // only via the .md files (currently we do not write
        // embeddings to disk). After a rebuild, vec rows are empty
        // until each memory is touched again.
        store.rebuild_index(|_| None).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("rebuild_memory_index panicked: {e}"))?
}
