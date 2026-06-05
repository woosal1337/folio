use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

const MCP_PATH: &str = ".attune/mcp.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServer>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Transport {
    #[default]
    Stdio,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServer {
    pub name: String,
    #[serde(default)]
    pub transport: Transport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
}

impl McpServer {
    pub fn is_valid(&self) -> bool {
        match self.transport {
            Transport::Stdio => self.command.is_some(),
            Transport::Http => self.url.is_some(),
        }
    }
}

pub fn config_path(vault_root: &Path) -> std::path::PathBuf {
    vault_root.join(MCP_PATH)
}

pub fn parse(input: &str) -> Result<McpConfig> {
    toml::from_str::<McpConfig>(input)
        .map_err(|e| AttuneError::Storage(format!("invalid mcp.toml: {e}")))
}

pub fn load(vault_root: &Path) -> Result<McpConfig> {
    let path = config_path(vault_root);
    if !path.exists() {
        return Ok(McpConfig {
            servers: Vec::new(),
        });
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| AttuneError::Storage(format!("could not read {}: {e}", path.display())))?;
    parse(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_minimum_stdio_server() {
        let input = r#"
            [[servers]]
            name = "linear"
            command = "npx"
            args = ["-y", "linear-mcp-server"]
        "#;
        let config = parse(input).unwrap();
        assert_eq!(config.servers.len(), 1);
        let server = &config.servers[0];
        assert_eq!(server.name, "linear");
        assert_eq!(server.transport, Transport::Stdio);
        assert!(server.is_valid());
    }

    #[test]
    fn parses_an_http_server() {
        let input = r#"
            [[servers]]
            name = "github"
            transport = "http"
            url = "https://mcp.github.com/sse"
        "#;
        let config = parse(input).unwrap();
        let server = &config.servers[0];
        assert_eq!(server.transport, Transport::Http);
        assert!(server.is_valid());
    }

    #[test]
    fn is_valid_rejects_stdio_without_command() {
        let server = McpServer {
            name: "broken".into(),
            transport: Transport::Stdio,
            command: None,
            args: vec![],
            env: BTreeMap::new(),
            url: None,
            headers: BTreeMap::new(),
        };
        assert!(!server.is_valid());
    }

    #[test]
    fn is_valid_rejects_http_without_url() {
        let server = McpServer {
            name: "broken".into(),
            transport: Transport::Http,
            command: None,
            args: vec![],
            env: BTreeMap::new(),
            url: None,
            headers: BTreeMap::new(),
        };
        assert!(!server.is_valid());
    }

    #[test]
    fn load_returns_empty_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config = load(dir.path()).unwrap();
        assert!(config.servers.is_empty());
    }

    #[test]
    fn load_reads_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let attune = dir.path().join(".attune");
        fs::create_dir_all(&attune).unwrap();
        fs::write(
            attune.join("mcp.toml"),
            r#"
                [[servers]]
                name = "linear"
                command = "npx"
            "#,
        )
        .unwrap();
        let config = load(dir.path()).unwrap();
        assert_eq!(config.servers.len(), 1);
    }

    #[test]
    fn env_and_headers_serde_round_trip() {
        let mut env = BTreeMap::new();
        env.insert("LINEAR_API_KEY".into(), "lin_api_secret".into());
        let server = McpServer {
            name: "linear".into(),
            transport: Transport::Stdio,
            command: Some("npx".into()),
            args: vec!["-y".into(), "linear-mcp-server".into()],
            env,
            url: None,
            headers: BTreeMap::new(),
        };
        let s = toml::to_string_pretty(&McpConfig {
            servers: vec![server.clone()],
        })
        .unwrap();
        let parsed = parse(&s).unwrap();
        assert_eq!(parsed.servers[0], server);
    }
}
