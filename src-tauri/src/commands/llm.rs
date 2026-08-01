use folio_core::llm::provider::LlmProvider;
use folio_core::llm::types::{ModelInfo, ProviderStatus};
use folio_core::llm::{KeyStore, OpenAiProvider, ProviderId};
use tracing::{debug, info};

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

#[tauri::command]
pub async fn set_provider_key(provider: ProviderId, api_key: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || KeyStore::set(provider, &api_key))
        .await
        .map_err(|e| format!("set_provider_key task panicked: {e}"))?
        .map_err(|e| e.to_string())?;
    info!(provider = provider.as_str(), "stored provider api key");
    Ok(())
}

#[tauri::command]
pub async fn delete_provider_key(provider: ProviderId) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || KeyStore::delete(provider))
        .await
        .map_err(|e| format!("delete_provider_key task panicked: {e}"))?
        .map_err(|e| e.to_string())?;
    info!(provider = provider.as_str(), "deleted provider api key");
    Ok(())
}

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
        _ => Err(format!("{} is not yet supported", provider.display_name())),
    }
}

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
        _ => Err(format!("{} is not yet supported", provider.display_name())),
    }
}
