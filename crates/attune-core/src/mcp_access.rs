//! MCP per-client consent layer + append-only access ledger (GET-210).
//!
//! Every MCP client that connects to Attune must hold a named grant
//! before it can read notes. Grants are stored in
//! `<vault>/.attune/mcp-grants.toml` — a file the user owns and edits.
//! The access ledger (`<vault>/.attune/mcp-access.log`) records every
//! read as a one-line JSON entry so the user can audit what each tool
//! called and which notes it touched.
//!
//! ## Grant file format (mcp-grants.toml)
//!
//! ```toml
//! [[clients]]
//! client_id   = "claude-code"
//! client_name = "Claude Code CLI"
//! allow_reads = true
//! granted_at  = "2026-06-01T12:00:00Z"
//!
//! [[clients]]
//! client_id   = "cursor"
//! allow_reads = false   # revoked
//! ```
//!
//! ## Access log format (mcp-access.log)
//!
//! One JSON object per line (JSON-L):
//! ```json
//! {"ts":"2026-06-01T12:05:00Z","client":"claude-code","tool":"recent_meetings","notes":["2026-06-01-10-00-00"]}
//! ```

use std::fs;
use std::io::Write as IoWrite;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const GRANTS_PATH: &str = ".attune/mcp-grants.toml";
const ACCESS_LOG_PATH: &str = ".attune/mcp-access.log";

// ---------------------------------------------------------------------------
// Grant types
// ---------------------------------------------------------------------------

/// One named MCP client and its read permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientGrant {
    /// Stable identifier for the client (e.g. "claude-code", "cursor").
    pub client_id: String,
    /// Human-readable name shown in Settings → Connectors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    /// When false the client's read requests are rejected.
    #[serde(default)]
    pub allow_reads: bool,
    /// ISO-8601 timestamp when the grant was created or last changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_at: Option<DateTime<Utc>>,
}

/// Root of `mcp-grants.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpGrants {
    #[serde(default)]
    pub clients: Vec<McpClientGrant>,
}

impl McpGrants {
    /// True when `client_id` has an explicit `allow_reads = true` grant.
    /// Defaults to **denied** if no grant exists (opt-in, not opt-out).
    pub fn is_allowed(&self, client_id: &str) -> bool {
        self.clients
            .iter()
            .find(|g| g.client_id == client_id)
            .map(|g| g.allow_reads)
            .unwrap_or(false)
    }

    /// Grant read access for a client. Creates the entry if absent.
    pub fn grant(&mut self, client_id: &str, client_name: Option<&str>) {
        if let Some(entry) = self.clients.iter_mut().find(|g| g.client_id == client_id) {
            entry.allow_reads = true;
            entry.granted_at = Some(Utc::now());
        } else {
            self.clients.push(McpClientGrant {
                client_id: client_id.to_string(),
                client_name: client_name.map(str::to_string),
                allow_reads: true,
                granted_at: Some(Utc::now()),
            });
        }
    }

    /// Revoke read access for a client. Does not remove the entry so the
    /// revocation is visible in the file.
    pub fn revoke(&mut self, client_id: &str) {
        if let Some(entry) = self.clients.iter_mut().find(|g| g.client_id == client_id) {
            entry.allow_reads = false;
            entry.granted_at = Some(Utc::now());
        }
    }
}

/// Load grants from `<vault_root>/.attune/mcp-grants.toml`.
/// Returns an empty (deny-all) `McpGrants` when the file is absent.
///
/// # Errors
///
/// Returns `Err(AttuneError::Storage(...))` when the file exists but
/// cannot be read or parsed.
pub fn load_grants(vault_root: &Path) -> crate::error::Result<McpGrants> {
    let path = vault_root.join(GRANTS_PATH);
    if !path.exists() {
        return Ok(McpGrants::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| crate::error::AttuneError::Storage(format!("mcp-grants read: {e}")))?;
    toml::from_str(&raw)
        .map_err(|e| crate::error::AttuneError::Storage(format!("mcp-grants parse: {e}")))
}

/// Persist grants to `<vault_root>/.attune/mcp-grants.toml`.
///
/// # Errors
///
/// Returns `Err` if the directory cannot be created or the file cannot
/// be written.
pub fn save_grants(vault_root: &Path, grants: &McpGrants) -> crate::error::Result<()> {
    let path = vault_root.join(GRANTS_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| crate::error::AttuneError::Storage(format!("mcp-grants mkdir: {e}")))?;
    }
    let toml = toml::to_string_pretty(grants)
        .map_err(|e| crate::error::AttuneError::Storage(format!("mcp-grants serialize: {e}")))?;
    crate::storage::atomic_write::atomic_write(&path, toml.as_bytes())
        .map_err(|e| crate::error::AttuneError::Storage(format!("mcp-grants write: {e}")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Access ledger
// ---------------------------------------------------------------------------

/// One entry in the MCP access log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAccessEntry {
    /// ISO-8601 timestamp.
    pub ts: DateTime<Utc>,
    /// Client identifier.
    pub client: String,
    /// Tool that was called.
    pub tool: String,
    /// Session labels (or other identifiers) of notes returned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// Optional free-form query or filter string for auditability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

/// Append one access entry to the ledger as a JSON-L line.
/// The file is created on first write; subsequent calls append.
/// Non-fatal on failure — a broken log must never block the actual
/// MCP tool response.
pub fn append_access_entry(vault_root: &Path, entry: &McpAccessEntry) {
    let path = vault_root.join(ACCESS_LOG_PATH);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            if let Ok(line) = serde_json::to_string(entry) {
                let _ = writeln!(f, "{line}");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "mcp-access.log write failed (non-fatal)");
        }
    }
}

/// Read all access log entries. Returns an empty vec when the file
/// doesn't exist. Malformed lines are skipped with a warning.
pub fn read_access_log(vault_root: &Path) -> Vec<McpAccessEntry> {
    let path = vault_root.join(ACCESS_LOG_PATH);
    let Ok(raw) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            match serde_json::from_str::<McpAccessEntry>(line) {
                Ok(e) => Some(e),
                Err(e) => {
                    tracing::warn!(error = %e, "mcp-access.log: skipping malformed line");
                    None
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_grants_deny_all() {
        let g = McpGrants::default();
        assert!(!g.is_allowed("claude-code"));
        assert!(!g.is_allowed("cursor"));
    }

    #[test]
    fn grant_allows_revoke_denies() {
        let mut g = McpGrants::default();
        g.grant("claude-code", Some("Claude Code CLI"));
        assert!(g.is_allowed("claude-code"));
        g.revoke("claude-code");
        assert!(!g.is_allowed("claude-code"));
        // Entry still present for audit trail.
        assert_eq!(g.clients.len(), 1);
    }

    #[test]
    fn load_returns_empty_when_file_missing() {
        let dir = std::path::PathBuf::from("/nonexistent-vault");
        let grants = load_grants(&dir).unwrap();
        assert!(grants.clients.is_empty());
    }

    #[test]
    fn access_entry_round_trips_json() {
        let e = McpAccessEntry {
            ts: chrono::Utc::now(),
            client: "cursor".into(),
            tool: "recent_meetings".into(),
            notes: vec!["2026-06-01-10-00-00".into()],
            query: None,
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: McpAccessEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.client, "cursor");
        assert_eq!(back.notes.len(), 1);
    }

    #[test]
    fn append_and_read_log() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();
        let e = McpAccessEntry {
            ts: chrono::Utc::now(),
            client: "test-client".into(),
            tool: "recent_meetings".into(),
            notes: vec!["note-a".into(), "note-b".into()],
            query: Some("Q3".into()),
        };
        append_access_entry(vault, &e);
        append_access_entry(vault, &e); // two entries
        let log = read_access_log(vault);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].client, "test-client");
        assert_eq!(log[0].notes.len(), 2);
        assert_eq!(log[0].query.as_deref(), Some("Q3"));
    }
}
