//! Memory CRUD commands backing the /memory page + the `remember`
//! and `search_memory` agent tools.
//!
//! Disk + embedding work runs inside `spawn_blocking` so the IPC
//! runtime stays free. Embedding calls go to OpenAI; if the API
//! errors (no key, transient), the memory is still persisted to
//! files + FTS5 — only the vec index slot is empty, which the
//! hybrid retrieval path falls back from gracefully.

use std::path::PathBuf;

use attune_core::llm::{KeyStore, ProviderId};
use attune_core::memory::{
    EmbeddingClient, Memory, MemoryKind, MemoryQuery, MemoryStore, MemoryUpdate, NewMemory,
};
use tauri::State;
use tracing::{debug, info, warn};

use crate::app::AppState;

/// Snapshot the memory dir from settings without holding the lock.
fn current_memory_dir(state: &AppState) -> PathBuf {
    state.settings.lock().memory_dir.clone()
}

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

#[tauri::command]
pub async fn list_memories(
    state: State<'_, AppState>,
    query: MemoryQuery,
) -> Result<Vec<Memory>, String> {
    let dir = current_memory_dir(&state);
    debug!(?query, "list_memories");
    tauri::async_runtime::spawn_blocking(move || {
        let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
        store.list(&query).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("list_memories panicked: {e}"))?
}

#[tauri::command]
pub async fn get_memory(state: State<'_, AppState>, id: String) -> Result<Option<Memory>, String> {
    let dir = current_memory_dir(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
        store.get(&id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("get_memory panicked: {e}"))?
}

#[tauri::command]
pub async fn create_memory(
    state: State<'_, AppState>,
    memory: NewMemory,
) -> Result<Memory, String> {
    let dir = current_memory_dir(&state);
    let content_for_embed = memory.content.clone();
    info!(kind = %memory.kind.as_str(), "create_memory");

    // First: write through the conflict-resolution path. This may
    // mutate previously-current rows (supersede).
    let written = {
        let dir = dir.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
            store
                .create(memory)
                .map(|o| o.into_memory())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("create_memory panicked: {e}"))??
    };

    // Second: best-effort embedding upsert so the vec index can serve
    // future searches. Failure is logged but not fatal.
    if let Some(embedding) = embed_if_possible(&content_for_embed).await {
        let dir = dir.clone();
        let m = written.clone();
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
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
    let dir = current_memory_dir(&state);
    info!(id = %id, "update_memory");
    let content_changed = patch.content.is_some();
    let updated = {
        let dir = dir.clone();
        let id = id.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
            store.update(&id, patch).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("update_memory panicked: {e}"))??
    };
    if content_changed {
        if let Some(embedding) = embed_if_possible(&updated.content).await {
            let dir = dir.clone();
            let m = updated.clone();
            tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
                let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
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
    let dir = current_memory_dir(&state);
    info!(id = %id, "delete_memory");
    tauri::async_runtime::spawn_blocking(move || {
        let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
        store.soft_delete(&id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("delete_memory panicked: {e}"))?
}

#[tauri::command]
pub async fn purge_memory(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let dir = current_memory_dir(&state);
    info!(id = %id, "purge_memory");
    tauri::async_runtime::spawn_blocking(move || {
        let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
        store.purge(&id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("purge_memory panicked: {e}"))?
}

#[tauri::command]
pub async fn pin_memory(
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<Memory, String> {
    let dir = current_memory_dir(&state);
    info!(id = %id, pinned, "pin_memory");
    tauri::async_runtime::spawn_blocking(move || {
        let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
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
    let dir = current_memory_dir(&state);
    let embedding = embed_if_possible(&query).await;
    let limit = limit.unwrap_or(10);
    tauri::async_runtime::spawn_blocking(move || {
        let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
        store
            .search(&query, embedding.as_deref(), &kinds, limit)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("search_memories panicked: {e}"))?
}

#[tauri::command]
pub async fn rebuild_memory_index(state: State<'_, AppState>) -> Result<usize, String> {
    let dir = current_memory_dir(&state);
    info!("rebuild_memory_index");
    tauri::async_runtime::spawn_blocking(move || {
        let store = MemoryStore::open(dir).map_err(|e| e.to_string())?;
        // Embeddings are not refetched during rebuild — they survive
        // only via the .md files (currently we do not write
        // embeddings to disk). After a rebuild, vec rows are empty
        // until each memory is touched again.
        store.rebuild_index(|_| None).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("rebuild_memory_index panicked: {e}"))?
}
