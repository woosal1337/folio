use tauri::State;
use tracing::{debug, info};

use attune_core::mcp_access::{
    append_access_entry, load_grants, read_access_log, save_grants, McpAccessEntry, McpClientGrant,
};

use crate::app::AppState;

fn vault_root(state: &AppState) -> std::path::PathBuf {
    let output_dir = state.settings.lock().output_dir.clone();
    output_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(output_dir)
}

#[tauri::command]
pub fn list_mcp_grants(state: State<'_, AppState>) -> Result<Vec<McpClientGrant>, String> {
    debug!("list_mcp_grants");
    let vault = vault_root(&state);
    load_grants(&vault)
        .map(|g| g.clients)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn grant_mcp_client(
    state: State<'_, AppState>,
    client_id: String,
    client_name: Option<String>,
) -> Result<(), String> {
    info!(client_id = %client_id, "grant_mcp_client");
    let vault = vault_root(&state);
    let mut grants = load_grants(&vault).map_err(|e| e.to_string())?;
    grants.grant(&client_id, client_name.as_deref());
    save_grants(&vault, &grants).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn revoke_mcp_client(state: State<'_, AppState>, client_id: String) -> Result<(), String> {
    info!(client_id = %client_id, "revoke_mcp_client");
    let vault = vault_root(&state);
    let mut grants = load_grants(&vault).map_err(|e| e.to_string())?;
    grants.revoke(&client_id);
    save_grants(&vault, &grants).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_mcp_access_log(state: State<'_, AppState>) -> Vec<McpAccessEntry> {
    debug!("list_mcp_access_log");
    let vault = vault_root(&state);
    let mut entries = read_access_log(&vault);
    entries.reverse();
    entries.truncate(200);
    entries
}

#[tauri::command]
pub fn check_mcp_grant(state: State<'_, AppState>, client_id: String) -> bool {
    let vault = vault_root(&state);
    load_grants(&vault)
        .map(|g| g.is_allowed(&client_id))
        .unwrap_or(false)
}

#[tauri::command]
pub fn record_mcp_access(
    state: State<'_, AppState>,
    client_id: String,
    tool: String,
    notes: Vec<String>,
    query: Option<String>,
) -> Result<(), String> {
    let vault = vault_root(&state);
    let entry = McpAccessEntry {
        ts: chrono::Utc::now(),
        client: client_id,
        tool,
        notes,
        query,
    };
    append_access_entry(&vault, &entry);
    Ok(())
}
