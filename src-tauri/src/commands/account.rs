//! IPC for `attune-core::backend::account`.

use attune_core::backend::account as backend_account;
use attune_core::backend::types::{DeviceDoc, UserDoc};
use attune_core::backend::BackendClient;

#[tauri::command]
pub async fn account_get() -> Result<UserDoc, String> {
    let client = BackendClient::new();
    backend_account::get_account(&client)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn account_update(display_name: Option<String>) -> Result<UserDoc, String> {
    let client = BackendClient::new();
    backend_account::update_account(&client, display_name.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn account_devices() -> Result<Vec<DeviceDoc>, String> {
    let client = BackendClient::new();
    backend_account::list_devices(&client)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn account_revoke_device(device_id: String) -> Result<(), String> {
    let client = BackendClient::new();
    backend_account::revoke_device(&client, &device_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn account_soft_delete() -> Result<(), String> {
    let client = BackendClient::new();
    backend_account::soft_delete_account(&client)
        .await
        .map_err(|e| e.to_string())
}
