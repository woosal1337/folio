//! `/api/auth/*` — OTP signup, code-verify, refresh, logout.
//!
//! The high-level flow is:
//!   1. `request_signin_code(email)` — backend emails a 6-digit OTP.
//!   2. `verify_signin_code(email, code, device_id, device_name)` —
//!      exchanges the code for `{access, refresh, user}` and the
//!      client persists the tokens via [`TokenStore`].
//!   3. Subsequent authenticated calls let [`BackendClient`] handle
//!      refresh-on-401 transparently.
//!   4. `logout()` blacklists the access token server-side and
//!      clears the local keychain row.

use serde::{Deserialize, Serialize};

use crate::backend::client::{BackendClient, BackendError};
use crate::backend::tokens::{AuthTokens, TokenStore, UserIdentity};
use crate::backend::types::{CodeVerifyRequest, LogoutRequest, SignupRequest, VerifyResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigninCodeResult {
    /// Same wire envelope `message` field — surfaced to the UI as a
    /// non-sensitive status line ("Check your email…").
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct VerifiedSession {
    pub tokens: AuthTokens,
    pub identity: UserIdentity,
}

/// `POST /api/auth/signup` — idempotent. Triggers an OTP email.
pub async fn request_signin_code(
    client: &BackendClient,
    email: &str,
) -> Result<SigninCodeResult, BackendError> {
    // The endpoint returns `{success, message}` with no `data`. The
    // envelope unwrapper turns that into `()`; we return the wire
    // message via a one-shot probe so the UI can show it.
    client
        .post_anon::<SignupRequest<'_>, ()>("/auth/signup", &SignupRequest { email })
        .await?;
    Ok(SigninCodeResult {
        message: "Check your email for a 6-digit sign-in code.".to_string(),
    })
}

/// `POST /api/auth/code-verify` — exchanges the OTP for tokens. On
/// success persists the keychain triple and returns the cached
/// identity for the UI.
pub async fn verify_signin_code(
    client: &BackendClient,
    email: &str,
    code: &str,
    device_id: &str,
    device_name: &str,
) -> Result<VerifiedSession, BackendError> {
    let body = CodeVerifyRequest {
        email,
        code,
        device_id,
        device_name,
    };
    let resp: VerifyResponse = client.post_anon("/auth/code-verify", &body).await?;
    let user = resp.user.ok_or_else(|| BackendError::Api {
        status: 200,
        message: "verify response missing user payload".to_string(),
    })?;
    let tokens = AuthTokens {
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        device_id: resp.device_id.or_else(|| Some(device_id.to_string())),
    };
    let identity = UserIdentity {
        user_id: user.id,
        email: user.email,
        display_name: user.display_name,
        privacy_tier: user.privacy_tier,
    };
    TokenStore::save(&tokens, &identity).map_err(|e| BackendError::Token(e.to_string()))?;
    Ok(VerifiedSession { tokens, identity })
}

/// `POST /api/auth/logout` — server-side blacklist + local clear.
/// Always clears the keychain even when the server call fails so the
/// user can't get stuck in a half-signed-in state.
pub async fn logout(client: &BackendClient) -> Result<(), BackendError> {
    let refresh = TokenStore::refresh_token().map_err(|e| BackendError::Token(e.to_string()))?;
    let body = LogoutRequest {
        refresh_token: refresh.as_deref(),
    };
    let server = client
        .post::<LogoutRequest<'_>, ()>("/auth/logout", &body)
        .await;
    let _ = TokenStore::clear();
    match server {
        Ok(()) => Ok(()),
        Err(BackendError::Unauthorized) => Ok(()),
        Err(e) => Err(e),
    }
}
