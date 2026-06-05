use std::path::PathBuf;

use serde::Serialize;
use tauri::AppHandle;
use tracing::debug;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientStatus {
    Detected,

    NotFound,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpClient {
    pub id: String,
    pub name: String,
    pub status: ClientStatus,

    pub config_path: Option<String>,

    pub json_snippet: String,

    pub cli_command: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpConnectInfo {
    pub clients: Vec<McpClient>,

    pub binary_path: Option<String>,
}

fn find_binary(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let candidate = exe.parent().unwrap_or(exe.as_path()).join("folio-mcp");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Ok(output) = std::process::Command::new("which")
        .arg("folio-mcp")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    let _ = app;
    None
}

fn folio_json_block(binary: &str) -> String {
    format!(
        r#"{{
  "folio": {{
    "command": "{binary}",
    "args": [],
    "env": {{}}
  }}
}}"#
    )
}

fn folio_json_block_in_servers(binary: &str) -> String {
    format!(
        r#"{{
  "mcpServers": {{
    "folio": {{
      "command": "{binary}",
      "args": [],
      "env": {{}}
    }}
  }}
}}"#
    )
}

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

    let json_snippet = format!(
        "// Add this to your claude_desktop_config.json → \"mcpServers\" object:\n{}",
        folio_json_block(binary)
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
        folio_json_block_in_servers(binary)
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

    let cli_command = format!("claude mcp add folio --transport stdio {binary}");
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
    let config_path =
        home().map(|h| h.join("Library/Application Support/Windsurf/mcp_config.json"));
    let status = config_path
        .as_ref()
        .filter(|p| p.parent().map(|d| d.exists()).unwrap_or(false))
        .map(|_| ClientStatus::Detected)
        .unwrap_or(ClientStatus::NotFound);

    let json_snippet = format!(
        "// Paste into the Windsurf MCP config:\n{}",
        folio_json_block_in_servers(binary)
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

#[tauri::command]
pub fn generate_mcp_config(app: AppHandle) -> McpConnectInfo {
    debug!("generate_mcp_config");
    let binary = find_binary(&app);
    let binary_str = binary
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/path/to/folio-mcp".to_string());

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

const ALLOWED_CONFIG_DIRS: &[&str] = &[
    "Library/Application Support/Claude",
    ".cursor",
    "Library/Application Support/Windsurf",
];

fn is_allowed_config_path(path: &std::path::Path) -> bool {
    if path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return false;
    }
    let Ok(home_str) = std::env::var("HOME") else {
        return false;
    };
    let home = std::path::PathBuf::from(home_str);

    let parent = match path.parent() {
        Some(p) => p,
        None => return false,
    };
    let leaf = match path.file_name() {
        Some(n) => n,
        None => return false,
    };

    if std::path::Path::new(leaf).components().count() != 1 {
        return false;
    }
    let canon_parent = match std::fs::canonicalize(parent) {
        Ok(p) => p,

        Err(_) => {
            let mut ancestor = parent;
            loop {
                if let Some(p) = ancestor.parent() {
                    if p.exists() {
                        if let Ok(canon) = std::fs::canonicalize(p) {
                            return ALLOWED_CONFIG_DIRS.iter().filter(|d| !d.is_empty()).any(
                                |dir| {
                                    std::fs::canonicalize(home.join(dir))
                                        .map(|allowed| canon.starts_with(&allowed))
                                        .unwrap_or(false)
                                },
                            );
                        }
                        return false;
                    }
                    ancestor = p;
                } else {
                    return false;
                }
            }
        }
    };
    ALLOWED_CONFIG_DIRS
        .iter()
        .filter(|d| !d.is_empty())
        .any(|dir| {
            std::fs::canonicalize(home.join(dir))
                .map(|canon_allowed| canon_parent.starts_with(&canon_allowed))
                .unwrap_or(false)
        })
}

#[tauri::command]
pub fn write_mcp_config(
    config_path: String,
    binary_path: String,
    client_id: String,
) -> Result<String, String> {
    let path = std::path::PathBuf::from(&config_path);
    if !is_allowed_config_path(&path) {
        return Err(format!(
            "write_mcp_config: config_path '{}' is outside the allowed MCP config directories",
            path.display()
        ));
    }

    if binary_path.is_empty()
        || binary_path.contains('\n')
        || binary_path.contains('\r')
        || binary_path.contains('"')
    {
        return Err("write_mcp_config: binary_path contains invalid characters".to_string());
    }

    let existing: serde_json::Value = if path.exists() {
        let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let mut cfg = existing;

    let _ = client_id;
    let mcp_servers = cfg
        .as_object_mut()
        .ok_or("config root is not a JSON object")?
        .entry("mcpServers")
        .or_insert(serde_json::json!({}));

    mcp_servers
        .as_object_mut()
        .ok_or("mcpServers is not a JSON object")?
        .insert(
            "folio".to_string(),
            serde_json::json!({
                "command": binary_path,
                "args": [],
                "env": {}
            }),
        );

    let updated = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &updated).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;

    tracing::info!(path = %path.display(), "wrote folio MCP config");
    Ok(updated)
}
