use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use folio_core::server::{sync_session, RemoteClient, ServerTokens, SyncOutcome, SyncState};
use serde::Serialize;
use tauri::State;
use tracing::info;

use crate::app::AppState;

static SYNCING: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);

pub struct SyncGuard(PathBuf);

impl SyncGuard {
    fn claim(session_dir: &Path) -> Option<Self> {
        let mut slot = SYNCING.lock().ok()?;
        let set = slot.get_or_insert_with(HashSet::new);
        if !set.insert(session_dir.to_path_buf()) {
            return None;
        }
        Some(SyncGuard(session_dir.to_path_buf()))
    }
}

impl Drop for SyncGuard {
    fn drop(&mut self) {
        if let Ok(mut slot) = SYNCING.lock() {
            if let Some(set) = slot.as_mut() {
                set.remove(&self.0);
            }
        }
    }
}

pub const SYNC_ALREADY_RUNNING: &str = "a sync is already running for this recording";

fn build_client(endpoint: &str, with_token: bool) -> Result<RemoteClient, String> {
    let mut client = RemoteClient::new(endpoint).map_err(|e| e.to_string())?;
    if with_token {
        if let Some(token) = ServerTokens::access().map_err(|e| e.to_string())? {
            client = client.with_token(token);
        }
    }
    Ok(client)
}

fn require_endpoint(state: &State<'_, AppState>) -> Result<String, String> {
    let endpoint = state.settings.lock().remote_endpoint.clone();
    if endpoint.trim().is_empty() {
        return Err("remote endpoint is not set — configure it in Settings → Transcription".into());
    }
    Ok(endpoint)
}

#[tauri::command]
pub async fn remote_register(
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> Result<(), String> {
    let endpoint = require_endpoint(&state)?;
    let client = build_client(&endpoint, false)?;
    let tokens = client
        .register(&email, &password)
        .await
        .map_err(|e| e.to_string())?;
    ServerTokens::set(&tokens.access_token, &tokens.refresh_token).map_err(|e| e.to_string())?;
    info!("registered with remote server");
    Ok(())
}

#[tauri::command]
pub async fn remote_login(
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> Result<(), String> {
    let endpoint = require_endpoint(&state)?;
    let client = build_client(&endpoint, false)?;
    let tokens = client
        .login(&email, &password)
        .await
        .map_err(|e| e.to_string())?;
    ServerTokens::set(&tokens.access_token, &tokens.refresh_token).map_err(|e| e.to_string())?;
    info!("logged in to remote server");
    Ok(())
}

#[tauri::command]
pub async fn remote_logout() -> Result<(), String> {
    ServerTokens::clear().map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoteAccount {
    pub signed_in: bool,
    pub email: Option<String>,
}

#[tauri::command]
pub async fn remote_me(state: State<'_, AppState>) -> Result<RemoteAccount, String> {
    if !ServerTokens::has() {
        return Ok(RemoteAccount {
            signed_in: false,
            email: None,
        });
    }
    let endpoint = state.settings.lock().remote_endpoint.clone();
    if endpoint.trim().is_empty() {
        return Ok(RemoteAccount {
            signed_in: true,
            email: None,
        });
    }
    let client = build_client(&endpoint, true)?;
    match client.me().await {
        Ok(user) => Ok(RemoteAccount {
            signed_in: true,
            email: Some(user.email),
        }),
        Err(_) => Ok(RemoteAccount {
            signed_in: true,
            email: None,
        }),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointTest {
    pub ok: bool,
    pub engine: Option<String>,
    pub model: Option<String>,
    pub gpu: Option<bool>,
    pub message: String,
}

#[tauri::command]
pub async fn test_remote_endpoint(endpoint: String) -> Result<EndpointTest, String> {
    let client = RemoteClient::new(&endpoint).map_err(|e| e.to_string())?;
    match client.capabilities().await {
        Ok(caps) => Ok(EndpointTest {
            ok: true,
            engine: Some(caps.engine),
            model: Some(caps.model),
            gpu: Some(caps.gpu),
            message: format!("Connected to {} v{}", caps.name, caps.version),
        }),
        Err(e) => Ok(EndpointTest {
            ok: false,
            engine: None,
            model: None,
            gpu: None,
            message: e.to_string(),
        }),
    }
}

#[tauri::command]
pub async fn sync_recording(
    state: State<'_, AppState>,
    session_dir: PathBuf,
) -> Result<SyncState, String> {
    if !ServerTokens::has() {
        return Err(
            "Not signed in to your Folio server — open Settings → Transcription → Remote server and create an account or sign in.".into(),
        );
    }
    let (endpoint, output_dir, language) = {
        let settings = state.settings.lock();
        (
            settings.remote_endpoint.clone(),
            settings.output_dir.clone(),
            settings.transcription_language.clone(),
        )
    };
    if endpoint.trim().is_empty() {
        return Err("remote endpoint is not set".into());
    }
    let session_dir = folio_core::paths::canonicalize_under(&output_dir, &session_dir)
        .map_err(|e| format!("invalid session directory: {e}"))?;
    let language = (!language.is_empty() && language != "auto").then_some(language);
    let outcome = run_sync(&endpoint, &session_dir, language.as_deref()).await?;
    Ok(outcome.state)
}

fn is_auth_error(e: &folio_core::FolioError) -> bool {
    let s = e.to_string().to_lowercase();
    s.contains("401")
        || s.contains("invalid token")
        || s.contains("unauthorized")
        || s.contains("missing bearer")
}

async fn refresh_tokens(endpoint: &str) -> Result<(), String> {
    let refresh = ServerTokens::refresh()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "session expired — sign in to your server again".to_string())?;
    let client = RemoteClient::new(endpoint).map_err(|e| e.to_string())?;
    match client.refresh(&refresh).await {
        Ok(tokens) => {
            ServerTokens::set(&tokens.access_token, &tokens.refresh_token)
                .map_err(|e| e.to_string())?;
            info!("refreshed remote server access token");
            Ok(())
        }
        Err(e) => {
            let _ = ServerTokens::clear();
            Err(format!(
                "session expired — sign in to your server again ({e})"
            ))
        }
    }
}

pub async fn run_sync(
    endpoint: &str,
    session_dir: &std::path::Path,
    language: Option<&str>,
) -> Result<SyncOutcome, String> {
    let _guard = SyncGuard::claim(session_dir).ok_or(SYNC_ALREADY_RUNNING)?;
    let client = build_client(endpoint, true)?;
    match sync_session(&client, session_dir, language).await {
        Ok(outcome) => Ok(outcome),
        Err(e) if is_auth_error(&e) => {
            refresh_tokens(endpoint).await?;
            let client = build_client(endpoint, true)?;
            sync_session(&client, session_dir, language)
                .await
                .map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}
