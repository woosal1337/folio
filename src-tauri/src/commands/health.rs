//! Health checks used during scaffolding to verify the IPC bridge.

use tracing::debug;

#[tauri::command]
pub fn ping(name: Option<String>) -> String {
    debug!(?name, "ping");
    match name {
        Some(n) => format!("pong, {n}"),
        None => "pong".into(),
    }
}
