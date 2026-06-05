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

pub fn raw_url(repo: &str, branch: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{repo}/{branch}/{path}")
}

pub fn parse_index(input: &str) -> crate::error::Result<MarketplaceIndex> {
    toml::from_str::<MarketplaceIndex>(input).map_err(|e| {
        crate::error::AttuneError::Storage(format!("invalid marketplace index TOML: {e}"))
    })
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
