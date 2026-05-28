//! Tauri IPC for `attune-core::backend::auth` + identity surface.
//!
//! Wire shape:
//!   * `auth_request_signin_code(email)` → ()
//!   * `auth_verify_signin_code(email, code, device_id, device_name)` → AuthIdentityWire
//!   * `auth_status()` → AuthStatus (cached identity for app boot)
//!   * `auth_logout()` → ()
//!
//! Token persistence lives in the keychain via
//! `attune_core::backend::tokens`. The frontend never sees the access
//! or refresh tokens — it only sees the cached identity.

use attune_core::backend::auth as backend_auth;
use attune_core::backend::tokens::TokenStore;
use attune_core::backend::{BackendClient, UserIdentity};
use serde::Serialize;

/// Boot-time auth probe result. Mirrors the hand-written TS shape in
/// `src/shared/types/AuthStatus.ts` — too small to wire through ts-rs.
#[derive(Debug, Clone, Serialize)]
pub struct AuthStatus {
    pub signed_in: bool,
    pub identity: Option<UserIdentity>,
}

#[tauri::command]
pub async fn auth_request_signin_code(email: String) -> Result<(), String> {
    let client = BackendClient::new();
    backend_auth::request_signin_code(&client, &email)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn auth_verify_signin_code(
    email: String,
    code: String,
    device_id: String,
    device_name: String,
) -> Result<UserIdentity, String> {
    let client = BackendClient::new();
    let session =
        backend_auth::verify_signin_code(&client, &email, &code, &device_id, &device_name)
            .await
            .map_err(|e| e.to_string())?;
    Ok(session.identity)
}

#[tauri::command]
pub fn auth_status() -> AuthStatus {
    let signed_in = TokenStore::is_signed_in();
    let identity = if signed_in {
        TokenStore::identity().ok().flatten()
    } else {
        None
    };
    AuthStatus {
        signed_in,
        identity,
    }
}

#[tauri::command]
pub async fn auth_logout() -> Result<(), String> {
    let client = BackendClient::new();
    backend_auth::logout(&client)
        .await
        .map_err(|e| e.to_string())
}
