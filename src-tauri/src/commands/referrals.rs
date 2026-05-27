//! IPC for `attune-core::backend::referrals`.

use attune_core::backend::referrals as backend_referrals;
use attune_core::backend::types::{ReferralStats, ReferralTokenResponse};
use attune_core::backend::BackendClient;

#[tauri::command]
pub async fn referrals_generate() -> Result<ReferralTokenResponse, String> {
    let client = BackendClient::new();
    backend_referrals::generate_token(&client)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn referrals_me() -> Result<ReferralStats, String> {
    let client = BackendClient::new();
    backend_referrals::stats(&client)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn referrals_redeem(
    token: String,
    new_user_id: String,
    new_user_email: String,
) -> Result<(), String> {
    let client = BackendClient::new();
    backend_referrals::redeem(&client, &token, &new_user_id, &new_user_email)
        .await
        .map_err(|e| e.to_string())
}
