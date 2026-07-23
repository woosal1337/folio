use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{FolioError, Result};

const DECISIONS_DIR: &str = ".folio/decisions";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Decision {
    pub id: String,
    pub statement: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    pub source_session_dir: String,
    pub source_session_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_span: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversed_by_id: Option<String>,
    pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDecision {
    pub statement: String,
    pub rationale: Option<String>,
    pub source_session_dir: String,
    pub source_session_label: String,
    pub evidence_span: Option<String>,
}

pub fn decisions_dir(vault_root: &Path) -> PathBuf {
    vault_root.join(DECISIONS_DIR)
}

pub fn ensure_dir(vault_root: &Path) -> Result<PathBuf> {
    let dir = decisions_dir(vault_root);
    fs::create_dir_all(&dir).map_err(|e| {
        FolioError::Storage(format!(
            "could not create decisions dir {}: {e}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

pub fn create(vault_root: &Path, new: NewDecision) -> Result<Decision> {
    let dir = ensure_dir(vault_root)?;
    let decision = Decision {
        id: Uuid::new_v4().to_string(),
        statement: new.statement,
        rationale: new.rationale,
        source_session_dir: new.source_session_dir,
        source_session_label: new.source_session_label,
        evidence_span: new.evidence_span,
        reversed_by_id: None,
        decided_at: Utc::now(),
    };
    write_atomic(&dir, &decision)?;
    Ok(decision)
}

pub fn get(vault_root: &Path, id: &str) -> Result<Option<Decision>> {
    let path = decisions_dir(vault_root).join(format!("{id}.json"));
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read(&path)
        .map_err(|e| FolioError::Storage(format!("could not read {}: {e}", path.display())))?;
    let parsed = serde_json::from_slice::<Decision>(&raw).map_err(|e| {
        FolioError::Storage(format!("invalid decision JSON {}: {e}", path.display()))
    })?;
    Ok(Some(parsed))
}

pub fn list_all(vault_root: &Path) -> Result<Vec<Decision>> {
    let dir = decisions_dir(vault_root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|e| FolioError::Storage(format!("could not read {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| FolioError::Storage(format!("read_dir: {e}")))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read(&path)
            .map_err(|e| FolioError::Storage(format!("could not read {}: {e}", path.display())))?;
        let parsed = serde_json::from_slice::<Decision>(&raw).map_err(|e| {
            FolioError::Storage(format!("invalid decision JSON {}: {e}", path.display()))
        })?;
        out.push(parsed);
    }
    out.sort_by_key(|d| std::cmp::Reverse(d.decided_at));
    Ok(out)
}

pub fn mark_reversed_by(vault_root: &Path, prior_id: &str, new_id: &str) -> Result<Decision> {
    let dir = decisions_dir(vault_root);
    let mut prior = get(vault_root, prior_id)?
        .ok_or_else(|| FolioError::Storage(format!("decision {prior_id} not found")))?;
    prior.reversed_by_id = Some(new_id.to_string());
    write_atomic(&dir, &prior)?;
    Ok(prior)
}

fn write_atomic(dir: &Path, decision: &Decision) -> Result<()> {
    let final_path = dir.join(format!("{}.json", decision.id));
    let tmp_path = dir.join(format!("{}.json.tmp", decision.id));
    let json = serde_json::to_string_pretty(decision)
        .map_err(|e| FolioError::Storage(format!("could not serialise decision: {e}")))?;
    fs::write(&tmp_path, json)
        .map_err(|e| FolioError::Storage(format!("could not write {}: {e}", tmp_path.display())))?;
    fs::rename(&tmp_path, &final_path).map_err(|e| {
        FolioError::Storage(format!("could not rename {}: {e}", final_path.display()))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_decision(label: &str) -> NewDecision {
        NewDecision {
            statement: format!("ship the {label} by Friday"),
            rationale: Some(format!("agreed in the {label} review")),
            source_session_dir: format!("/r/{label}"),
            source_session_label: label.into(),
            evidence_span: Some(format!("let's ship the {label}")),
        }
    }

    #[test]
    fn create_then_get_returns_same_decision() {
        let dir = tempfile::tempdir().unwrap();
        let created = create(dir.path(), new_decision("redesign")).unwrap();
        let fetched = get(dir.path(), &created.id).unwrap().unwrap();
        assert_eq!(fetched, created);
    }

    #[test]
    fn list_all_returns_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let first = create(dir.path(), new_decision("alpha")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let second = create(dir.path(), new_decision("beta")).unwrap();
        let listed = list_all(dir.path()).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, second.id);
        assert_eq!(listed[1].id, first.id);
    }

    #[test]
    fn list_all_empty_when_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_all(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn mark_reversed_by_persists_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let prior = create(dir.path(), new_decision("old")).unwrap();
        let new = create(dir.path(), new_decision("new")).unwrap();
        mark_reversed_by(dir.path(), &prior.id, &new.id).unwrap();
        let refreshed = get(dir.path(), &prior.id).unwrap().unwrap();
        assert_eq!(refreshed.reversed_by_id.as_deref(), Some(new.id.as_str()));
    }

    #[test]
    fn get_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(get(dir.path(), "does-not-exist").unwrap().is_none());
    }

    #[test]
    fn list_all_ignores_non_json_files() {
        let dir = tempfile::tempdir().unwrap();
        let written = ensure_dir(dir.path()).unwrap();
        fs::write(written.join("README"), "ignored").unwrap();
        create(dir.path(), new_decision("only")).unwrap();
        assert_eq!(list_all(dir.path()).unwrap().len(), 1);
    }
}
