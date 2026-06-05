use std::sync::Arc;

use folio_core::llm::{KeyStore, ProviderId};
use folio_core::memory::{
    EmbeddingClient, Memory, MemoryKind, MemoryQuery, MemoryStore, MemoryUpdate, NewMemory,
};
use tauri::State;
use tracing::{debug, info, warn};

use crate::app::AppState;

async fn openai_key_opt() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| KeyStore::get(ProviderId::OpenAi))
        .await
        .map_err(|e| format!("keystore task panicked: {e}"))?
        .map_err(|e| e.to_string())
}

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
        let path = folio_core::memory::path_for(store.dir(), &memory);
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
        store.rebuild_index(|_| None).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("rebuild_memory_index panicked: {e}"))?
}
