use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

const TEMPLATE_DIR: &str = ".attune/templates";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingTemplate {
    pub slug: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub match_keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub summary_sections: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_bias: Option<String>,
}

pub fn templates_dir(vault_root: &Path) -> PathBuf {
    vault_root.join(TEMPLATE_DIR)
}

pub fn ensure_dir(vault_root: &Path) -> Result<PathBuf> {
    let dir = templates_dir(vault_root);
    fs::create_dir_all(&dir).map_err(|e| {
        AttuneError::Storage(format!(
            "could not create templates dir {}: {e}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

pub fn parse(input: &str) -> Result<MeetingTemplate> {
    toml::from_str::<MeetingTemplate>(input)
        .map_err(|e| AttuneError::Storage(format!("invalid template TOML: {e}")))
}

pub fn render(template: &MeetingTemplate) -> Result<String> {
    toml::to_string_pretty(template)
        .map_err(|e| AttuneError::Storage(format!("could not serialise template: {e}")))
}

pub fn load(vault_root: &Path, slug: &str) -> Result<Option<MeetingTemplate>> {
    let path = templates_dir(vault_root).join(format!("{slug}.toml"));
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| AttuneError::Storage(format!("could not read {}: {e}", path.display())))?;
    parse(&raw).map(Some)
}

pub fn save(vault_root: &Path, template: &MeetingTemplate) -> Result<PathBuf> {
    let dir = ensure_dir(vault_root)?;
    let final_path = dir.join(format!("{}.toml", template.slug));
    let tmp_path = dir.join(format!("{}.toml.tmp", template.slug));
    let rendered = render(template)?;
    fs::write(&tmp_path, rendered).map_err(|e| {
        AttuneError::Storage(format!("could not write {}: {e}", tmp_path.display()))
    })?;
    fs::rename(&tmp_path, &final_path).map_err(|e| {
        AttuneError::Storage(format!("could not rename {}: {e}", final_path.display()))
    })?;
    Ok(final_path)
}

pub fn list_all(vault_root: &Path) -> Result<Vec<MeetingTemplate>> {
    let dir = templates_dir(vault_root);
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
        out.push(parse(&raw)?);
    }
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(out)
}

pub fn pick_for_label(templates: &[MeetingTemplate], label: &str) -> Option<MeetingTemplate> {
    let label_lc = label.to_lowercase();
    if let Some(hit) = templates
        .iter()
        .find(|t| t.slug.eq_ignore_ascii_case(&label_lc))
    {
        return Some(hit.clone());
    }
    templates
        .iter()
        .find(|t| {
            t.match_keywords
                .iter()
                .any(|kw| label_lc.contains(&kw.to_lowercase()))
        })
        .cloned()
}

pub fn baked_in_defaults() -> Vec<MeetingTemplate> {
    vec![
        MeetingTemplate {
            slug: "standup".into(),
            name: "Daily standup".into(),
            description: Some("Yesterday / today / blockers per attendee.".into()),
            match_keywords: vec!["standup".into(), "daily".into(), "scrum".into()],
            agents: vec!["summarize".into(), "extract-tasks".into()],
            summary_sections: vec!["yesterday".into(), "today".into(), "blockers".into()],
            prompt_bias: Some(
                "This is a daily standup. Focus on commitments and blockers, not narration.".into(),
            ),
        },
        MeetingTemplate {
            slug: "1on1".into(),
            name: "1-on-1".into(),
            description: Some("Career growth, blockers, feedback in both directions.".into()),
            match_keywords: vec!["1:1".into(), "1-on-1".into(), "one-on-one".into()],
            agents: vec!["summarize".into(), "extract-memories".into()],
            summary_sections: vec!["growth".into(), "blockers".into(), "feedback".into()],
            prompt_bias: Some(
                "This is a 1-on-1. Treat personal context as memories worth keeping.".into(),
            ),
        },
        MeetingTemplate {
            slug: "interview".into(),
            name: "Interview".into(),
            description: Some("Candidate signal, red flags, follow-up questions.".into()),
            match_keywords: vec!["interview".into(), "candidate".into()],
            agents: vec!["summarize".into(), "find-decisions".into()],
            summary_sections: vec!["signal".into(), "red-flags".into(), "follow-up".into()],
            prompt_bias: Some(
                "This is an interview. Focus on candidate-evaluative claims with evidence.".into(),
            ),
        },
        MeetingTemplate {
            slug: "design-review".into(),
            name: "Design review".into(),
            description: Some(
                "Decisions made, open questions, design constraints surfaced.".into(),
            ),
            match_keywords: vec!["design".into(), "review".into()],
            agents: vec![
                "summarize".into(),
                "extract-tasks".into(),
                "find-decisions".into(),
            ],
            summary_sections: vec![
                "decisions".into(),
                "open-questions".into(),
                "constraints".into(),
            ],
            prompt_bias: Some(
                "This is a design review. Prioritise explicit decisions and open questions.".into(),
            ),
        },
        MeetingTemplate {
            slug: "customer-call".into(),
            name: "Customer call".into(),
            description: Some("Pain points, asks, commitments to the customer.".into()),
            match_keywords: vec!["customer".into(), "client".into(), "sales".into()],
            agents: vec![
                "summarize".into(),
                "extract-tasks".into(),
                "extract-memories".into(),
            ],
            summary_sections: vec!["pain-points".into(), "asks".into(), "commitments".into()],
            prompt_bias: Some(
                "This is a customer call. Treat every commitment as a task and every \
                 pain point as a memory."
                    .into(),
            ),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MeetingTemplate {
        MeetingTemplate {
            slug: "demo".into(),
            name: "Demo".into(),
            description: Some("A demo template.".into()),
            match_keywords: vec!["demo".into()],
            agents: vec!["summarize".into()],
            summary_sections: vec!["intro".into(), "outro".into()],
            prompt_bias: Some("Bias the model toward demo framing.".into()),
        }
    }

    #[test]
    fn round_trip_through_render_then_parse() {
        let original = sample();
        let s = render(&original).unwrap();
        let parsed = parse(&s).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn save_then_load_returns_same_template() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), &sample()).unwrap();
        let loaded = load(dir.path(), "demo").unwrap().unwrap();
        assert_eq!(loaded, sample());
    }

    #[test]
    fn pick_for_label_prefers_exact_slug_match() {
        let templates = baked_in_defaults();
        let hit = pick_for_label(&templates, "standup").unwrap();
        assert_eq!(hit.slug, "standup");
    }

    #[test]
    fn pick_for_label_falls_back_to_keyword_match() {
        let templates = baked_in_defaults();
        let hit = pick_for_label(&templates, "Mon morning standup with Alice").unwrap();
        assert_eq!(hit.slug, "standup");
    }

    #[test]
    fn pick_for_label_returns_none_when_nothing_matches() {
        let templates = baked_in_defaults();
        assert!(pick_for_label(&templates, "random thoughts").is_none());
    }

    #[test]
    fn baked_in_defaults_have_unique_slugs() {
        let defaults = baked_in_defaults();
        let mut slugs: Vec<&str> = defaults.iter().map(|t| t.slug.as_str()).collect();
        slugs.sort();
        let before = slugs.len();
        slugs.dedup();
        assert_eq!(slugs.len(), before, "duplicate slug in baked_in_defaults");
    }

    #[test]
    fn list_all_returns_empty_when_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        let listed = list_all(dir.path()).unwrap();
        assert!(listed.is_empty());
    }
}
