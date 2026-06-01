//! Local user profile that conditions all agent output (GET-206).
//!
//! Stored as `.attune/profile.toml` in the vault root. The file is
//! optional — if absent every field defaults to `None` and no context
//! is injected. When present, the profile is prepended to every agent's
//! system prompt so the model knows who is taking notes and what they
//! care about.
//!
//! ## Example `.attune/profile.toml`
//!
//! ```toml
//! role = "Product Manager"
//! team = "Growth"
//! focus_areas = ["user retention", "OKR tracking", "cross-functional alignment"]
//! ```
//!
//! All fields are optional. The user edits this file directly in any
//! text editor; Attune reloads it on each agent run. No backend, no
//! sync — the file is owned by the user and can be committed to a team
//! repo for shared context.

use std::path::Path;

use serde::{Deserialize, Serialize};

const PROFILE_PATH: &str = ".attune/profile.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfile {
    /// The user's role (e.g. "Product Manager", "Engineer", "CEO").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The user's team or department.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    /// Recurring focus areas, OKRs, or topics the user cares about.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_areas: Vec<String>,
    /// Optional standing objective for all meetings (a sentence the
    /// agents read as "what the user is trying to achieve").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
}

impl UserProfile {
    /// True when all fields are empty — no context to inject.
    pub fn is_empty(&self) -> bool {
        self.role.is_none()
            && self.team.is_none()
            && self.focus_areas.is_empty()
            && self.objective.is_none()
    }

    /// Render the profile as a system-prompt context block. Returns
    /// `None` when the profile is empty.
    pub fn as_prompt_context(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let mut lines = vec!["## About the note-taker".to_string()];
        if let Some(role) = &self.role {
            lines.push(format!("Role: {role}"));
        }
        if let Some(team) = &self.team {
            lines.push(format!("Team: {team}"));
        }
        if !self.focus_areas.is_empty() {
            lines.push(format!("Focus areas: {}", self.focus_areas.join(", ")));
        }
        if let Some(obj) = &self.objective {
            lines.push(format!("Objective: {obj}"));
        }
        lines.push(
            "Use this context to make your output relevant to the note-taker's role and goals. \
             Prioritise action items, decisions, and insights that bear on their focus areas."
                .to_string(),
        );
        Some(lines.join("\n"))
    }
}

/// Load the user profile from `<vault_root>/.attune/profile.toml`.
/// Returns `None` when the file does not exist (normal — no profile).
/// Logs and returns `None` on parse failure so a typo never breaks agents.
pub fn load(vault_root: &Path) -> Option<UserProfile> {
    let path = vault_root.join(PROFILE_PATH);
    if !path.exists() {
        return None;
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "user profile: read failed");
            return None;
        }
    };
    match toml::from_str::<UserProfile>(&raw) {
        Ok(p) if !p.is_empty() => Some(p),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "user profile: parse failed — ignored");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_is_empty() {
        assert!(UserProfile::default().is_empty());
        assert!(UserProfile::default().as_prompt_context().is_none());
    }

    #[test]
    fn profile_with_role_is_not_empty() {
        let p = UserProfile {
            role: Some("PM".into()),
            ..Default::default()
        };
        assert!(!p.is_empty());
        let ctx = p.as_prompt_context().unwrap();
        assert!(ctx.contains("PM"));
        assert!(ctx.contains("About the note-taker"));
    }

    #[test]
    fn full_profile_renders_all_fields() {
        let p = UserProfile {
            role: Some("Engineer".into()),
            team: Some("Platform".into()),
            focus_areas: vec!["reliability".into(), "latency".into()],
            objective: Some("Ship the cache layer by Q3.".into()),
        };
        let ctx = p.as_prompt_context().unwrap();
        assert!(ctx.contains("Engineer"));
        assert!(ctx.contains("Platform"));
        assert!(ctx.contains("reliability"));
        assert!(ctx.contains("latency"));
        assert!(ctx.contains("Ship the cache layer"));
    }

    #[test]
    fn load_returns_none_for_missing_dir() {
        assert!(load(std::path::Path::new("/nonexistent")).is_none());
    }

    #[test]
    fn load_parses_role_and_focus_areas() {
        let dir = tempfile::tempdir().unwrap();
        let attune_dir = dir.path().join(".attune");
        std::fs::create_dir_all(&attune_dir).unwrap();
        std::fs::write(
            attune_dir.join("profile.toml"),
            "role = \"PM\"\nfocus_areas = [\"retention\", \"growth\"]\n",
        )
        .unwrap();
        let profile = load(dir.path()).unwrap();
        assert_eq!(profile.role.as_deref(), Some("PM"));
        assert_eq!(profile.focus_areas, vec!["retention", "growth"]);
    }
}
