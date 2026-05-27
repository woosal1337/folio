//! Template marketplace client. v2 finding 086 / GET-106.
//!
//! Reads template manifests from a GitHub repo (default
//! `attune-ai/attune-templates`) and installs picked templates into
//! the user's local `<vault>/.attune/templates/` directory. The
//! manifest format is intentionally tiny so PR contributions to the
//! upstream repo stay frictionless.
//!
//! On-disk format in the marketplace repo:
//!
//!   templates/
//!     <slug>/
//!       template.toml
//!       README.md
//!     index.toml
//!
//! `index.toml` lists every template with a slug + display name +
//! description; the manifest URL the client fetches is the raw
//! GitHub blob path. The Tauri command surface fetches the index,
//! then installs the picked entry by copying its `template.toml`
//! into the user's templates directory via the existing
//! `templates::save` helper from GET-36.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceIndex {
    pub version: u32,
    pub entries: Vec<MarketplaceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceEntry {
    pub slug: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub author: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

pub const DEFAULT_REPO: &str = "attune-ai/attune-templates";

/// Build the raw GitHub URL for an asset inside the marketplace repo
/// on the given branch (default `main`).
pub fn raw_url(repo: &str, branch: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{repo}/{branch}/{path}")
}

/// Parse the index.toml fetched from the marketplace repo.
pub fn parse_index(input: &str) -> Result<MarketplaceIndex, String> {
    toml::from_str::<MarketplaceIndex>(input)
        .map_err(|e| format!("invalid marketplace index TOML: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_url_builds_canonical_github_path() {
        let url = raw_url(
            "attune-ai/attune-templates",
            "main",
            "templates/standup/template.toml",
        );
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/attune-ai/attune-templates/main/templates/standup/template.toml"
        );
    }

    #[test]
    fn parse_index_handles_minimum_fields() {
        let input = r#"
            version = 1

            [[entries]]
            slug = "standup"
            name = "Daily standup"
            author = "attune"
        "#;
        let parsed = parse_index(input).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.entries.len(), 1);
        let entry = &parsed.entries[0];
        assert_eq!(entry.slug, "standup");
        assert!(entry.description.is_none());
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn parse_index_returns_string_error_on_invalid_toml() {
        let result = parse_index("not = valid = toml");
        assert!(result.is_err());
    }

    #[test]
    fn default_repo_is_attune_templates() {
        assert_eq!(DEFAULT_REPO, "attune-ai/attune-templates");
    }
}
