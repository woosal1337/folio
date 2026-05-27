//! IPC for `attune-core::backend::settings_sync`.
//!
//! The pull endpoint returns the full snapshot (or null when never
//! pushed). Push performs a last-write-wins comparison server-side
//! and may reply with the existing newer snapshot — the frontend
//! resolves by reading `updated_at` against what it submitted.

use attune_core::backend::settings_sync as backend_sync;
use attune_core::backend::BackendClient;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsSyncSnapshot {
    pub settings: Option<Value>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<backend_sync::SettingsSnapshot> for SettingsSyncSnapshot {
    fn from(s: backend_sync::SettingsSnapshot) -> Self {
        Self {
            settings: s.settings,
            updated_at: s.updated_at,
        }
    }
}

#[tauri::command]
pub async fn settings_sync_pull() -> Result<SettingsSyncSnapshot, String> {
    let client = BackendClient::new();
    backend_sync::pull(&client)
        .await
        .map(Into::into)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_sync_push(
    settings: Value,
    updated_at: DateTime<Utc>,
) -> Result<SettingsSyncSnapshot, String> {
    let client = BackendClient::new();
    backend_sync::push(&client, &settings, updated_at)
        .await
        .map(Into::into)
        .map_err(|e| e.to_string())
}
