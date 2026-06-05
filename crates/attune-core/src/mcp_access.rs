use std::fs;
use std::io::Write as IoWrite;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const GRANTS_PATH: &str = ".attune/mcp-grants.toml";
const ACCESS_LOG_PATH: &str = ".attune/mcp-access.log";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientGrant {
    pub client_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,

    #[serde(default)]
    pub allow_reads: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpGrants {
    #[serde(default)]
    pub clients: Vec<McpClientGrant>,
}

impl McpGrants {
    pub fn is_allowed(&self, client_id: &str) -> bool {
        self.clients
            .iter()
            .find(|g| g.client_id == client_id)
            .map(|g| g.allow_reads)
            .unwrap_or(false)
    }

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

    pub fn revoke(&mut self, client_id: &str) {
        if let Some(entry) = self.clients.iter_mut().find(|g| g.client_id == client_id) {
            entry.allow_reads = false;
            entry.granted_at = Some(Utc::now());
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAccessEntry {
    pub ts: DateTime<Utc>,

    pub client: String,

    pub tool: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

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
        append_access_entry(vault, &e);
        let log = read_access_log(vault);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].client, "test-client");
        assert_eq!(log[0].notes.len(), 2);
        assert_eq!(log[0].query.as_deref(), Some("Q3"));
    }
}
