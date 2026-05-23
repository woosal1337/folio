//! Tauri commands for LLM provider configuration.
//!
//! Phase 1 of the AI chat feature ships the bare minimum the Settings
//! UI needs: list providers, see their status, store/delete a key in
//! the keychain, and a "Test" button that pings the provider with the
//! stored key. Chat commands (send_message, etc.) land in phase 4.

use attune_core::llm::provider::LlmProvider;
use attune_core::llm::types::{ModelInfo, ProviderStatus};
use attune_core::llm::{KeyStore, OpenAiProvider, ProviderId};
use tracing::{debug, info};

/// List every provider Attune supports plus its current configured /
/// not-configured state.
#[tauri::command]
pub fn list_providers() -> Vec<ProviderStatus> {
    debug!("list_providers");
    ProviderId::all()
        .iter()
        .map(|id| ProviderStatus {
            id: *id,
            display_name: id.display_name().to_string(),
            configured: KeyStore::has(*id),
            redacted_suffix: KeyStore::redacted_suffix(*id),
            recommended: matches!(id, ProviderId::OpenAi),
        })
        .collect()
}

/// Store an API key for `provider`. Empty strings are rejected.
#[tauri::command]
pub async fn set_provider_key(provider: ProviderId, api_key: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || KeyStore::set(provider, &api_key))
        .await
        .map_err(|e| format!("set_provider_key task panicked: {e}"))?
        .map_err(|e| e.to_string())?;
    info!(provider = provider.as_str(), "stored provider api key");
    Ok(())
}

/// Remove the API key for `provider`. Idempotent.
#[tauri::command]
pub async fn delete_provider_key(provider: ProviderId) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || KeyStore::delete(provider))
        .await
        .map_err(|e| format!("delete_provider_key task panicked: {e}"))?
        .map_err(|e| e.to_string())?;
    info!(provider = provider.as_str(), "deleted provider api key");
    Ok(())
}

/// Hit the provider's auth endpoint to confirm the stored key works.
/// Phase 1 only ships OpenAI; the other providers return a "not yet
/// implemented" error so the UI can disable their Test buttons until
/// phase 2.
#[tauri::command]
pub async fn test_provider(provider: ProviderId) -> Result<(), String> {
    let key = tauri::async_runtime::spawn_blocking(move || KeyStore::get(provider))
        .await
        .map_err(|e| format!("keystore lookup panicked: {e}"))?
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "no api key configured for {} — add one in Settings",
                provider.display_name()
            )
        })?;

    match provider {
        ProviderId::OpenAi => {
            let p = OpenAiProvider::new(key);
            p.test().await.map_err(|e| e.to_string())
        }
        ProviderId::Anthropic | ProviderId::Deepseek => Err(format!(
            "{} support arrives in phase 2 of the AI chat rollout",
            provider.display_name()
        )),
    }
}

/// List chat models the provider exposes for the configured key.
#[tauri::command]
pub async fn list_provider_models(provider: ProviderId) -> Result<Vec<ModelInfo>, String> {
    let key = tauri::async_runtime::spawn_blocking(move || KeyStore::get(provider))
        .await
        .map_err(|e| format!("keystore lookup panicked: {e}"))?
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "no api key configured for {} — add one in Settings",
                provider.display_name()
            )
        })?;

    match provider {
        ProviderId::OpenAi => {
            let p = OpenAiProvider::new(key);
            p.list_models().await.map_err(|e| e.to_string())
        }
        ProviderId::Anthropic | ProviderId::Deepseek => Err(format!(
            "{} support arrives in phase 2 of the AI chat rollout",
            provider.display_name()
        )),
    }
}
