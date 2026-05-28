//! `/api/settings/*` — settings snapshot sync with LWW conflict
//! resolution by `updated_at`.
//!
//! The server keeps one snapshot document per user. `pull` returns
//! the latest (or `{settings: null}` for never-pushed accounts);
//! `push` writes a new snapshot unless the server's `updated_at` is
//! strictly newer, in which case the server returns its copy with
//! `message: "conflict"` so the client can merge or overwrite.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::backend::client::{BackendClient, BackendError};

#[derive(Debug, Clone, Deserialize)]
pub struct SettingsSnapshot {
    /// The full Settings struct as JSON. `None` means the user has
    /// never pushed (fresh account).
    pub settings: Option<Value>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsPush<'a> {
    pub settings: &'a Value,
    pub updated_at: DateTime<Utc>,
}

pub async fn pull(client: &BackendClient) -> Result<SettingsSnapshot, BackendError> {
    client.get("/settings").await
}

/// Push a new snapshot. The server may reply with the existing newer
/// snapshot (LWW conflict) — the caller compares `updated_at` against
/// what it sent to decide whether the push won.
pub async fn push(
    client: &BackendClient,
    settings: &Value,
    updated_at: DateTime<Utc>,
) -> Result<SettingsSnapshot, BackendError> {
    let body = SettingsPush {
        settings,
        updated_at,
    };
    // The server returns 200 with `message: "conflict"` when our
    // updated_at lost the LWW comparison — the envelope's `success`
    // is still true so the unwrapper returns the snapshot and the
    // caller resolves.
    client
        .request(reqwest::Method::PUT, "/settings", Some(&body))
        .await
}
