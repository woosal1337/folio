//! `BackendClient` — async HTTP client for attune-api with automatic
//! bearer-token attach + refresh-on-401.
//!
//! Two operational modes:
//!   * **Anonymous** — no token; used for `/auth/signup`, `/auth/code-verify`,
//!     `/auth/login`, `/health`.
//!   * **Authenticated** — adds `Authorization: Bearer <access>` and, on
//!     a 401 response, exchanges the refresh token for a new access
//!     token and replays the original request once. If the refresh
//!     itself returns 401, the local session is cleared and the
//!     caller surfaces a `BackendError::Unauthorized` so the UI can
//!     route the user back to signup.
//!
//! The base URL defaults to `https://attune.chele.bi`. Override with
//! the `ATTUNE_API_BASE_URL` env var (used by dev / staging builds).

use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::backend::tokens::TokenStore;
use crate::backend::types::{ErrorBody, RefreshRequest, RefreshResponse};

/// Production base URL. Used when the binary is built in `--release`
/// mode and no `ATTUNE_API_BASE_URL` env var is set.
const PROD_BASE_URL: &str = "https://attune.chele.bi";

/// Dev base URL. Used automatically by `cargo build` / `bun tauri dev`
/// debug builds so the client talks to the local Docker stack out of
/// the box. Override via `ATTUNE_API_BASE_URL` if your local API
/// binds to a non-default port.
const DEV_BASE_URL: &str = "http://localhost:8000";

/// Total request timeout. Generous because the OTP-email path can
/// stall on slow SMTP providers.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Lock acquired around refresh-token exchange so two concurrent 401s
/// don't both kick off a refresh.
type RefreshLock = Arc<Mutex<()>>;

#[derive(Clone)]
pub struct BackendClient {
    http: Client,
    base_url: String,
    refresh_lock: RefreshLock,
}

impl BackendClient {
    pub fn new() -> Self {
        let base_url = std::env::var("ATTUNE_API_BASE_URL")
            .unwrap_or_else(|_| default_base_url().to_string())
            .trim_end_matches('/')
            .to_string();
        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(format!("attune-app/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client init");
        Self {
            http,
            base_url,
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Anonymous POST — no auth header attached.
    pub async fn post_anon<Req, Resp>(&self, path: &str, body: &Req) -> Result<Resp, BackendError>
    where
        Req: Serialize + ?Sized,
        Resp: DeserializeOwned,
    {
        let url = self.url(path);
        let res = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(BackendError::transport)?;
        unwrap_envelope(res).await
    }

    /// Anonymous GET — useful for `/health`.
    pub async fn get_anon<Resp>(&self, path: &str) -> Result<Resp, BackendError>
    where
        Resp: DeserializeOwned,
    {
        let url = self.url(path);
        let res = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(BackendError::transport)?;
        unwrap_envelope(res).await
    }

    /// Authenticated request with body. Refreshes on 401 then retries
    /// once. Subsequent 401 → `BackendError::Unauthorized`.
    pub async fn request<Req, Resp>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Req>,
    ) -> Result<Resp, BackendError>
    where
        Req: Serialize + ?Sized,
        Resp: DeserializeOwned,
    {
        let access = TokenStore::access_token()
            .map_err(|e| BackendError::Token(e.to_string()))?
            .ok_or(BackendError::Unauthorized)?;

        let res = self
            .send_with_token(&method, path, body, &access)
            .await?;

        if res.status() != StatusCode::UNAUTHORIZED {
            return unwrap_envelope(res).await;
        }

        debug!("backend 401 — attempting refresh");
        let new_access = self.refresh().await?;
        let res = self
            .send_with_token(&method, path, body, &new_access)
            .await?;
        unwrap_envelope(res).await
    }

    /// Authenticated GET. Convenience wrapper over [`Self::request`].
    pub async fn get<Resp>(&self, path: &str) -> Result<Resp, BackendError>
    where
        Resp: DeserializeOwned,
    {
        self.request::<(), Resp>(Method::GET, path, None).await
    }

    /// Authenticated POST.
    pub async fn post<Req, Resp>(&self, path: &str, body: &Req) -> Result<Resp, BackendError>
    where
        Req: Serialize + ?Sized,
        Resp: DeserializeOwned,
    {
        self.request(Method::POST, path, Some(body)).await
    }

    /// Authenticated PATCH.
    pub async fn patch<Req, Resp>(&self, path: &str, body: &Req) -> Result<Resp, BackendError>
    where
        Req: Serialize + ?Sized,
        Resp: DeserializeOwned,
    {
        self.request(Method::PATCH, path, Some(body)).await
    }

    /// Authenticated DELETE — body is rare here but kept for symmetry.
    pub async fn delete<Resp>(&self, path: &str) -> Result<Resp, BackendError>
    where
        Resp: DeserializeOwned,
    {
        self.request::<(), Resp>(Method::DELETE, path, None).await
    }

    fn url(&self, path: &str) -> String {
        let p = path.trim_start_matches('/');
        format!("{}/api/{}", self.base_url, p)
    }

    async fn send_with_token<Req: Serialize + ?Sized>(
        &self,
        method: &Method,
        path: &str,
        body: Option<&Req>,
        access_token: &str,
    ) -> Result<reqwest::Response, BackendError> {
        let url = self.url(path);
        let mut req = self
            .http
            .request(method.clone(), &url)
            .bearer_auth(access_token);
        if let Some(b) = body {
            req = req.json(b);
        }
        req.send().await.map_err(BackendError::transport)
    }

    /// Exchange the refresh token for a fresh access token. Updates
    /// the keychain in place; returns the new access token for the
    /// caller's retry.
    async fn refresh(&self) -> Result<String, BackendError> {
        let _guard = self.refresh_lock.lock().await;

        // NOTE: re-read the refresh token after acquiring the lock so
        // two racing 401s converge on whichever token the first
        // refresh already rotated in, instead of both replaying the
        // now-stale one.
        let refresh = TokenStore::refresh_token()
            .map_err(|e| BackendError::Token(e.to_string()))?
            .ok_or(BackendError::Unauthorized)?;

        let payload = RefreshRequest {
            refresh_token: &refresh,
            device_id: &device_id_or_empty(),
        };
        let url = self.url("/auth/refresh");
        let res = self
            .http
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(BackendError::transport)?;

        if res.status() == StatusCode::UNAUTHORIZED {
            warn!("refresh returned 401 — clearing local session");
            let _ = TokenStore::clear();
            return Err(BackendError::Unauthorized);
        }

        let new: RefreshResponse = unwrap_envelope(res).await?;
        TokenStore::update_access_token(&new.access_token)
            .map_err(|e| BackendError::Token(e.to_string()))?;
        TokenStore::update_refresh_token(&new.refresh_token)
            .map_err(|e| BackendError::Token(e.to_string()))?;
        Ok(new.access_token)
    }
}

impl Default for BackendClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Choose the default backend URL based on the build profile.
/// Debug builds (`bun tauri dev`, `cargo run`) point at the local
/// Docker stack; release builds point at the production cloud.
/// The `ATTUNE_API_BASE_URL` env var overrides either path.
fn default_base_url() -> &'static str {
    if cfg!(debug_assertions) {
        DEV_BASE_URL
    } else {
        PROD_BASE_URL
    }
}

/// Device id sent on the refresh request. The backend uses it for
/// session audit only; an empty string is tolerated for first-launch
/// corner cases, and the access token rotates regardless of which
/// device-id row the refresh matched.
///
// TODO(ege): persist a stable device_id in its own keychain slot and
// return it here so "manage your devices" can attribute refreshes.
fn device_id_or_empty() -> String {
    String::new()
}

async fn unwrap_envelope<Resp>(res: reqwest::Response) -> Result<Resp, BackendError>
where
    Resp: DeserializeOwned,
{
    let status = res.status();
    let text = res.text().await.map_err(BackendError::transport)?;

    if status.is_success() {
        // Two-step parse so endpoints that return `{success, message}`
        // (no `data`, e.g. logout) deserialize cleanly when the caller
        // asks for `()` — serde turns `Value::Null` into the unit type.
        let raw: serde_json::Value = if text.is_empty() {
            serde_json::json!({"success": true, "message": "", "data": null})
        } else {
            serde_json::from_str(&text).map_err(|e| BackendError::Decode(e.to_string()))?
        };
        if !raw.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Err(BackendError::Api {
                status: status.as_u16(),
                message: msg,
            });
        }
        let data = raw
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::from_value::<Resp>(data).map_err(|e| BackendError::Decode(e.to_string()))
    } else {
        let body: ErrorBody = serde_json::from_str(&text).unwrap_or(ErrorBody {
            success: false,
            message: text.clone(),
            error: None,
            data: None,
        });
        let message = if body.message.is_empty() {
            text
        } else {
            body.message
        };
        Err(match status {
            StatusCode::UNAUTHORIZED => BackendError::Unauthorized,
            StatusCode::TOO_MANY_REQUESTS => BackendError::RateLimited(message),
            _ => BackendError::Api {
                status: status.as_u16(),
                message,
            },
        })
    }
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("transport: {0}")]
    Transport(String),

    #[error("decode: {0}")]
    Decode(String),

    #[error("api {status}: {message}")]
    Api { status: u16, message: String },

    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("not signed in")]
    Unauthorized,

    #[error("token storage: {0}")]
    Token(String),
}

impl BackendError {
    fn transport(e: reqwest::Error) -> Self {
        BackendError::Transport(e.to_string())
    }
}
