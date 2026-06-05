use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

const AGENT_DIR: &str = ".attune/agents";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentDefinition {
    pub slug: String,
    pub name: String,
    pub system_prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

pub fn agents_dir(vault_root: &Path) -> PathBuf {
    vault_root.join(AGENT_DIR)
}

pub fn ensure_dir(vault_root: &Path) -> Result<PathBuf> {
    let dir = agents_dir(vault_root);
    fs::create_dir_all(&dir).map_err(|e| {
        AttuneError::Storage(format!(
            "could not create agents dir {}: {e}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

pub fn parse(input: &str) -> Result<AgentDefinition> {
    toml::from_str::<AgentDefinition>(input)
        .map_err(|e| AttuneError::Storage(format!("invalid agent TOML: {e}")))
}

pub fn render(agent: &AgentDefinition) -> Result<String> {
    toml::to_string_pretty(agent)
        .map_err(|e| AttuneError::Storage(format!("could not serialise agent: {e}")))
}

pub fn load(vault_root: &Path, slug: &str) -> Result<Option<AgentDefinition>> {
    let path = agents_dir(vault_root).join(format!("{slug}.toml"));
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| AttuneError::Storage(format!("could not read {}: {e}", path.display())))?;
    parse(&raw).map(Some)
}

pub fn save(vault_root: &Path, agent: &AgentDefinition) -> Result<PathBuf> {
    let dir = ensure_dir(vault_root)?;
    let final_path = dir.join(format!("{}.toml", agent.slug));
    let tmp_path = dir.join(format!("{}.toml.tmp", agent.slug));
    let rendered = render(agent)?;
    fs::write(&tmp_path, rendered).map_err(|e| {
        AttuneError::Storage(format!("could not write {}: {e}", tmp_path.display()))
    })?;
    fs::rename(&tmp_path, &final_path).map_err(|e| {
        AttuneError::Storage(format!("could not rename {}: {e}", final_path.display()))
    })?;
    Ok(final_path)
}

pub fn list_all(vault_root: &Path) -> Result<Vec<AgentDefinition>> {
    let dir = agents_dir(vault_root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|e| AttuneError::Storage(format!("could not read {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| AttuneError::Storage(format!("read_dir: {e}")))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .map_err(|e| AttuneError::Storage(format!("could not read {}: {e}", path.display())))?;
        let agent = parse(&raw)?;
        out.push(agent);
    }
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AgentDefinition {
        AgentDefinition {
            slug: "extract-tasks".into(),
            name: "Extract Tasks".into(),
            system_prompt: "You are a task-extraction agent.".into(),
            description: Some("Pulls action items from the transcript.".into()),
            model: Some("gpt-4o-mini".into()),
            tools: vec!["create_task".into()],
            trigger: Some("post-transcribe".into()),
            output_format: Some("tool-calls".into()),
        }
    }

    #[test]
    fn parse_round_trips_through_render() {
        let original = sample();
        let s = render(&original).unwrap();
        let parsed = parse(&s).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn save_then_load_returns_same_agent() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &sample()).unwrap();
        let loaded = load(dir.path(), "extract-tasks").unwrap().unwrap();
        assert_eq!(loaded, sample());
    }

    #[test]
    fn load_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path(), "nope").unwrap().is_none());
    }

    #[test]
    fn parse_tolerates_minimum_fields() {
        let minimal = r#"
            slug = "tiny"
            name = "Tiny"
            system_prompt = "You are tiny."
        "#;
        let parsed = parse(minimal).unwrap();
        assert_eq!(parsed.slug, "tiny");
        assert!(parsed.description.is_none());
        assert!(parsed.tools.is_empty());
    }

    #[test]
    fn list_all_returns_sorted_agents() {
        let dir = tempfile::tempdir().unwrap();
        let mut alpha = sample();
        alpha.slug = "alpha".into();
        alpha.name = "Alpha".into();
        let mut beta = sample();
        beta.slug = "beta".into();
        beta.name = "Beta".into();
        save(dir.path(), &beta).unwrap();
        save(dir.path(), &alpha).unwrap();
        let listed = list_all(dir.path()).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].slug, "alpha");
        assert_eq!(listed[1].slug, "beta");
    }

    #[test]
    fn list_all_ignores_non_toml_files() {
        let dir = tempfile::tempdir().unwrap();
        let agents = ensure_dir(dir.path()).unwrap();
        fs::write(agents.join("notes.txt"), "hello").unwrap();
        save(dir.path(), &sample()).unwrap();
        let listed = list_all(dir.path()).unwrap();
        assert_eq!(listed.len(), 1);
    }
}
