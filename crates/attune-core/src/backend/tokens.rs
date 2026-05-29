//! Keychain-backed storage for the auth-session triple
//! (access, refresh, identity) — distinct from the LLM provider keys in
//! `llm::keystore` so a user can rotate LLM keys without touching their
//! account session.
//!
//! Service: `com.attune.app.auth-tokens`
//! Accounts:
//!   * `access_token`  — short-lived JWT (~15 min)
//!   * `refresh_token` — long-lived opaque blob (~30 days)
//!   * `identity`      — JSON `{ user_id, email, display_name? }` for offline UI
//!
//! All three are written atomically by [`TokenStore::save`] and cleared
//! atomically by [`TokenStore::clear`]. Reading is per-account so a
//! partial save (rare — only on keychain unlock prompts) never returns
//! a half-set identity.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AttuneError, Result};

const KEYCHAIN_SERVICE: &str = "com.attune.app.auth-tokens";
const ACCOUNT_ACCESS: &str = "access_token";
const ACCOUNT_REFRESH: &str = "refresh_token";
const ACCOUNT_IDENTITY: &str = "identity";

/// What the backend returns after a successful auth flow. Mirrors the
/// `data` envelope of `/api/auth/code-verify`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// The freshly-issued device id (server may rotate it).
    #[serde(default)]
    pub device_id: Option<String>,
}

/// Cached identity surfaced to the UI between launches so the user
/// doesn't see "Signed in as ?" while the access token is being
/// refreshed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct UserIdentity {
    pub user_id: String,
    pub email: String,
    #[serde(default)]
    pub display_name: Option<String>,
    /// Tier-1 or Tier-2 — read from the wire response so the UI can
    /// reflect the encryption mode in the privacy band.
    #[serde(default)]
    pub privacy_tier: Option<String>,
}

pub struct TokenStore;

impl TokenStore {
    pub fn save(tokens: &AuthTokens, identity: &UserIdentity) -> Result<()> {
        entry(ACCOUNT_ACCESS)?
            .set_password(&tokens.access_token)
            .map_err(map_keychain)?;
        entry(ACCOUNT_REFRESH)?
            .set_password(&tokens.refresh_token)
            .map_err(map_keychain)?;
        let identity_json =
            serde_json::to_string(identity).map_err(|e| AttuneError::Other(e.to_string()))?;
        entry(ACCOUNT_IDENTITY)?
            .set_password(&identity_json)
            .map_err(map_keychain)?;
        Ok(())
    }

    pub fn access_token() -> Result<Option<String>> {
        read(ACCOUNT_ACCESS)
    }

    pub fn refresh_token() -> Result<Option<String>> {
        read(ACCOUNT_REFRESH)
    }

    /// Overwrite just the cached identity blob, leaving the access /
    /// refresh tokens untouched. Used when the profile changes
    /// (e.g. display name) so `auth_status` — which reads identity from
    /// this cache, not the API — reflects the change across restarts.
    pub fn update_identity(identity: &UserIdentity) -> Result<()> {
        let identity_json =
            serde_json::to_string(identity).map_err(|e| AttuneError::Other(e.to_string()))?;
        entry(ACCOUNT_IDENTITY)?
            .set_password(&identity_json)
            .map_err(map_keychain)?;
        Ok(())
    }

    pub fn identity() -> Result<Option<UserIdentity>> {
        let Some(json) = read(ACCOUNT_IDENTITY)? else {
            return Ok(None);
        };
        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| AttuneError::Other(format!("identity parse: {e}")))
    }

    pub fn update_access_token(token: &str) -> Result<()> {
        entry(ACCOUNT_ACCESS)?
            .set_password(token)
            .map_err(map_keychain)
    }

    pub fn update_refresh_token(token: &str) -> Result<()> {
        entry(ACCOUNT_REFRESH)?
            .set_password(token)
            .map_err(map_keychain)
    }

    pub fn clear() -> Result<()> {
        for acc in [ACCOUNT_ACCESS, ACCOUNT_REFRESH, ACCOUNT_IDENTITY] {
            match entry(acc)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(e) => return Err(AttuneError::Keychain(e.to_string())),
            }
        }
        Ok(())
    }

    pub fn is_signed_in() -> bool {
        matches!(read(ACCOUNT_REFRESH), Ok(Some(_)))
    }
}

fn entry(account: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(KEYCHAIN_SERVICE, account).map_err(map_keychain)
}

fn read(account: &str) -> Result<Option<String>> {
    match entry(account)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AttuneError::Keychain(e.to_string())),
    }
}

fn map_keychain(e: keyring::Error) -> AttuneError {
    AttuneError::Keychain(e.to_string())
}
