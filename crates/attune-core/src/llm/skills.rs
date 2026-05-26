//! Skillification: corrections become reusable skills.
//! v2 finding 034 / GET-59.
//!
//! When the user edits the same agent output the same way twice in a
//! row (e.g. corrects 'Acme corp' to 'ACME Corporation' on two
//! separate meetings), Attune offers to turn the correction into a
//! reusable skill: a tiny TOML file under `<vault>/.attune/skills/`
//! that ships as a few-shot example with future agent prompts.
//!
//! On-disk format:
//!
//! ```toml
//! slug = "acme-canonicalisation"
//! agent_id = "extract-tasks"
//! description = "Always render Acme as 'ACME Corporation' in tasks."
//! version = 1
//! created_at = "2026-05-26T10:00:00Z"
//!
//! [[examples]]
//! before = "send the deck to Acme corp"
//! after  = "send the deck to ACME Corporation"
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

const SKILLS_DIR: &str = ".attune/skills";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillExample {
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Skill {
    pub slug: String,
    pub agent_id: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub examples: Vec<SkillExample>,
}

fn default_version() -> u32 {
    1
}

/// Track the user's recent corrections in-memory. When the same
/// (agent_id, before, after) triple is seen for the second time, we
/// suggest a skill. Pure data — the React side keeps the instance.
#[derive(Debug, Default, Clone)]
pub struct CorrectionTracker {
    seen: HashMap<(String, String, String), u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSuggestion {
    pub agent_id: String,
    pub before: String,
    pub after: String,
    pub seen_count: u32,
}

impl CorrectionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a correction. Returns Some(SkillSuggestion) the second
    /// (and subsequent) times the same triple is seen — the caller
    /// renders the 'turn into a skill?' prompt on Some.
    pub fn record(&mut self, agent_id: &str, before: &str, after: &str) -> Option<SkillSuggestion> {
        let key = (agent_id.to_string(), before.to_string(), after.to_string());
        let entry = self.seen.entry(key).or_insert(0);
        *entry += 1;
        if *entry >= 2 {
            Some(SkillSuggestion {
                agent_id: agent_id.to_string(),
                before: before.to_string(),
                after: after.to_string(),
                seen_count: *entry,
            })
        } else {
            None
        }
    }
}

pub fn skills_dir(vault_root: &Path) -> PathBuf {
    vault_root.join(SKILLS_DIR)
}

pub fn ensure_dir(vault_root: &Path) -> Result<PathBuf> {
    let dir = skills_dir(vault_root);
    fs::create_dir_all(&dir).map_err(|e| {
        AttuneError::Storage(format!("could not create skills dir {}: {e}", dir.display()))
    })?;
    Ok(dir)
}

pub fn parse(input: &str) -> Result<Skill> {
    toml::from_str::<Skill>(input)
        .map_err(|e| AttuneError::Storage(format!("invalid skill TOML: {e}")))
}

pub fn render(skill: &Skill) -> Result<String> {
    toml::to_string_pretty(skill)
        .map_err(|e| AttuneError::Storage(format!("could not serialise skill: {e}")))
}

pub fn save(vault_root: &Path, skill: &Skill) -> Result<PathBuf> {
    let dir = ensure_dir(vault_root)?;
    let final_path = dir.join(format!("{}.toml", skill.slug));
    let tmp_path = dir.join(format!("{}.toml.tmp", skill.slug));
    let body = render(skill)?;
    fs::write(&tmp_path, body).map_err(|e| {
        AttuneError::Storage(format!("could not write {}: {e}", tmp_path.display()))
    })?;
    fs::rename(&tmp_path, &final_path).map_err(|e| {
        AttuneError::Storage(format!("could not rename {}: {e}", final_path.display()))
    })?;
    Ok(final_path)
}

/// Enumerate every skill TOML under the vault. Returns an empty list
/// when the directory does not exist.
pub fn list_all(vault_root: &Path) -> Result<Vec<Skill>> {
    let dir = skills_dir(vault_root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| {
        AttuneError::Storage(format!("could not read {}: {e}", dir.display()))
    })? {
        let entry = entry.map_err(|e| AttuneError::Storage(format!("read_dir: {e}")))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|e| {
            AttuneError::Storage(format!("could not read {}: {e}", path.display()))
        })?;
        out.push(parse(&raw)?);
    }
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(out)
}

/// Filter the catalogue to the skills relevant for `agent_id`. The
/// agent runner injects these as few-shot examples in the system
/// prompt.
pub fn for_agent(skills: &[Skill], agent_id: &str) -> Vec<Skill> {
    skills
        .iter()
        .filter(|s| s.agent_id == agent_id)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(slug: &str, agent: &str) -> Skill {
        Skill {
            slug: slug.into(),
            agent_id: agent.into(),
            description: "Demo skill".into(),
            version: 1,
            created_at: Utc::now(),
            examples: vec![SkillExample {
                before: "acme corp".into(),
                after: "ACME Corporation".into(),
            }],
        }
    }

    #[test]
    fn tracker_returns_none_on_first_sighting() {
        let mut tracker = CorrectionTracker::new();
        assert!(tracker.record("extract-tasks", "acme corp", "ACME Corporation").is_none());
    }

    #[test]
    fn tracker_returns_suggestion_on_second_sighting() {
        let mut tracker = CorrectionTracker::new();
        tracker.record("extract-tasks", "acme corp", "ACME Corporation");
        let suggestion = tracker
            .record("extract-tasks", "acme corp", "ACME Corporation")
            .unwrap();
        assert_eq!(suggestion.agent_id, "extract-tasks");
        assert_eq!(suggestion.seen_count, 2);
    }

    #[test]
    fn tracker_distinguishes_distinct_corrections() {
        let mut tracker = CorrectionTracker::new();
        tracker.record("extract-tasks", "a", "A");
        let suggestion = tracker.record("extract-tasks", "b", "B");
        assert!(suggestion.is_none());
    }

    #[test]
    fn parse_round_trips_through_render() {
        let original = sample("acme", "extract-tasks");
        let s = render(&original).unwrap();
        let parsed = parse(&s).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn save_then_load_returns_same_skill() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &sample("acme", "extract-tasks")).unwrap();
        let listed = list_all(dir.path()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].slug, "acme");
    }

    #[test]
    fn for_agent_filters_by_agent_id() {
        let skills = vec![sample("a", "extract-tasks"), sample("b", "summarize")];
        let hits = for_agent(&skills, "extract-tasks");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].slug, "a");
    }

    #[test]
    fn list_all_returns_empty_when_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_all(dir.path()).unwrap().is_empty());
    }
}
