//! MCP client detection + config generator (GET-208).
//!
//! Detects installed MCP clients (Claude Desktop, Cursor, Claude Code
//! CLI) by checking well-known config file locations on macOS, then
//! produces the exact JSON snippet (or CLI command) the user can paste /
//! auto-write. Zero egress — all paths are resolved locally.

use std::path::PathBuf;

use serde::Serialize;
use tauri::AppHandle;
use tracing::debug;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Status of a detected MCP client.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientStatus {
    /// Config file exists (client is likely installed).
    Detected,
    /// Could not find the client's config directory.
    NotFound,
}

/// One detected (or absent) MCP client.
#[derive(Debug, Clone, Serialize)]
pub struct McpClient {
    pub id: String,
    pub name: String,
    pub status: ClientStatus,
    /// Absolute path to the client's MCP config file (when detected).
    pub config_path: Option<String>,
    /// The ready-to-paste JSON block for this client's config file.
    pub json_snippet: String,
    /// For CLI clients: the exact command to run.
    pub cli_command: Option<String>,
}

/// Return from [`generate_mcp_config`].
#[derive(Debug, Clone, Serialize)]
pub struct McpConnectInfo {
    pub clients: Vec<McpClient>,
    /// Best-effort path to the `attune-mcp` binary. If not yet available
    /// (binary not shipped in this build), this is `None` and the snippet
    /// shows a placeholder the user can fill in.
    pub binary_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Binary discovery
// ---------------------------------------------------------------------------

/// Try to locate the `attune-mcp` server binary. Checks:
/// 1. Next to the main app binary (dev + prod).
/// 2. `PATH` shim (when installed via Homebrew or similar).
fn find_binary(app: &AppHandle) -> Option<PathBuf> {
    // Prefer a binary sitting next to the main executable.
    if let Ok(exe) = std::env::current_exe() {
        let candidate = exe.parent().unwrap_or(exe.as_path()).join("attune-mcp");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    // Fall back to PATH lookup.
    if let Ok(output) = std::process::Command::new("which")
        .arg("attune-mcp")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    let _ = app; // Reserved for future resource-dir lookup.
    None
}

// ---------------------------------------------------------------------------
// Config builders
// ---------------------------------------------------------------------------

fn attune_json_block(binary: &str) -> String {
    format!(
        r#"{{
  "attune": {{
    "command": "{binary}",
    "args": [],
    "env": {{}}
  }}
}}"#
    )
}

fn attune_json_block_in_servers(binary: &str) -> String {
    format!(
        r#"{{
  "mcpServers": {{
    "attune": {{
      "command": "{binary}",
      "args": [],
      "env": {{}}
    }}
  }}
}}"#
    )
}

// ---------------------------------------------------------------------------
// Client detection
// ---------------------------------------------------------------------------

fn home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn detect_claude_desktop(binary: &str) -> McpClient {
    let config_path =
        home().map(|h| h.join("Library/Application Support/Claude/claude_desktop_config.json"));
    let status = config_path
        .as_ref()
        .filter(|p| p.parent().map(|d| d.exists()).unwrap_or(false))
        .map(|_| ClientStatus::Detected)
        .unwrap_or(ClientStatus::NotFound);

    // Claude Desktop merges into the existing JSON; show only the
    // `mcpServers` block the user needs to add / merge.
    let json_snippet = format!(
        "// Add this to your claude_desktop_config.json → \"mcpServers\" object:\n{}",
        attune_json_block(binary)
    );

    McpClient {
        id: "claude-desktop".into(),
        name: "Claude Desktop".into(),
        status,
        config_path: config_path.map(|p| p.to_string_lossy().into_owned()),
        json_snippet,
        cli_command: None,
    }
}

fn detect_cursor(binary: &str) -> McpClient {
    let config_path = home().map(|h| h.join(".cursor/mcp.json"));
    let status = config_path
        .as_ref()
        .filter(|p| p.parent().map(|d| d.exists()).unwrap_or(false))
        .map(|_| ClientStatus::Detected)
        .unwrap_or(ClientStatus::NotFound);

    let json_snippet = format!(
        "// Paste into ~/.cursor/mcp.json (create if missing):\n{}",
        attune_json_block_in_servers(binary)
    );

    McpClient {
        id: "cursor".into(),
        name: "Cursor".into(),
        status,
        config_path: config_path.map(|p| p.to_string_lossy().into_owned()),
        json_snippet,
        cli_command: None,
    }
}

fn detect_claude_code(binary: &str) -> McpClient {
    // Claude Code CLI: `claude mcp add <name> <command> [args...]`
    // Detect by checking if `claude` is on PATH.
    let has_claude = std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some();

    let status = if has_claude {
        ClientStatus::Detected
    } else {
        ClientStatus::NotFound
    };

    let cli_command = format!("claude mcp add attune --transport stdio {binary}");
    let json_snippet = format!("// Run this in your terminal:\n{cli_command}");

    McpClient {
        id: "claude-code".into(),
        name: "Claude Code".into(),
        status,
        config_path: None,
        json_snippet,
        cli_command: Some(cli_command),
    }
}

fn detect_windsurf(binary: &str) -> McpClient {
    // Windsurf (Codeium): config at ~/Library/Application Support/Windsurf/mcp_config.json
    let config_path =
        home().map(|h| h.join("Library/Application Support/Windsurf/mcp_config.json"));
    let status = config_path
        .as_ref()
        .filter(|p| p.parent().map(|d| d.exists()).unwrap_or(false))
        .map(|_| ClientStatus::Detected)
        .unwrap_or(ClientStatus::NotFound);

    let json_snippet = format!(
        "// Paste into the Windsurf MCP config:\n{}",
        attune_json_block_in_servers(binary)
    );

    McpClient {
        id: "windsurf".into(),
        name: "Windsurf".into(),
        status,
        config_path: config_path.map(|p| p.to_string_lossy().into_owned()),
        json_snippet,
        cli_command: None,
    }
}

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

/// Detect installed MCP clients and generate connection configs.
#[tauri::command]
pub fn generate_mcp_config(app: AppHandle) -> McpConnectInfo {
    debug!("generate_mcp_config");
    let binary = find_binary(&app);
    let binary_str = binary
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/path/to/attune-mcp".to_string());

    let clients = vec![
        detect_claude_desktop(&binary_str),
        detect_cursor(&binary_str),
        detect_claude_code(&binary_str),
        detect_windsurf(&binary_str),
    ];

    McpConnectInfo {
        clients,
        binary_path: binary.map(|p| p.to_string_lossy().into_owned()),
    }
}

/// Write the Attune MCP block into a client's config file.
///
/// For Claude Desktop: merges `attune` into the existing
/// `mcpServers` object (or creates the key if absent).
/// For Cursor / Windsurf: writes / merges `mcpServers.attune`.
///
/// Returns the updated file content as a string so the caller
/// can show it in a diff view before committing.
#[tauri::command]
pub fn write_mcp_config(
    config_path: String,
    binary_path: String,
    client_id: String,
) -> Result<String, String> {
    let path = std::path::PathBuf::from(&config_path);

    // Read existing content or start fresh.
    let existing: serde_json::Value = if path.exists() {
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let mut cfg = existing;

    // Normalize: Claude Desktop uses top-level `mcpServers`.
    // Cursor / Windsurf use the same structure.
    let _ = client_id; // reserved for client-specific transformations
    let mcp_servers = cfg
        .as_object_mut()
        .ok_or("config root is not a JSON object")?
        .entry("mcpServers")
        .or_insert(serde_json::json!({}));

    mcp_servers
        .as_object_mut()
        .ok_or("mcpServers is not a JSON object")?
        .insert(
            "attune".to_string(),
            serde_json::json!({
                "command": binary_path,
                "args": [],
                "env": {}
            }),
        );

    let updated = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;

    // Create parent dirs if needed.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Atomic write via temp + rename.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &updated).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;

    tracing::info!(path = %path.display(), "wrote attune MCP config");
    Ok(updated)
}
