use std::path::Path;

use serde::{Deserialize, Serialize};

const RECIPES_DIR: &str = ".folio/recipes";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecipe {
    pub label: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

pub fn load(vault_root: &Path) -> Vec<UserRecipe> {
    let dir = vault_root.join(RECIPES_DIR);
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "user recipe: read failed");
                continue;
            }
        };
        match toml::from_str::<UserRecipe>(&raw) {
            Ok(r) => out.push(r),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "user recipe: parse failed — skipped");
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_returns_empty_when_dir_missing() {
        let dir = std::path::PathBuf::from("/nonexistent-vault-root");
        let recipes = load(&dir);
        assert!(recipes.is_empty());
    }

    #[test]
    fn load_parses_minimal_recipe() {
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join(".folio/recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();
        std::fs::write(
            recipes_dir.join("standup.toml"),
            "label = \"Standup prep\"\nprompt = \"Summarise my last 3 meetings.\"\n",
        )
        .unwrap();
        let recipes = load(dir.path());
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].label, "Standup prep");
        assert_eq!(recipes[0].prompt, "Summarise my last 3 meetings.");
        assert!(recipes[0].icon.is_none());
    }

    #[test]
    fn load_skips_non_toml_and_malformed_files() {
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join(".folio/recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();
        std::fs::write(
            recipes_dir.join("good.toml"),
            "label = \"Good\"\nprompt = \"p\"\n",
        )
        .unwrap();
        std::fs::write(recipes_dir.join("bad.toml"), "not toml{{{{").unwrap();
        std::fs::write(recipes_dir.join("ignored.md"), "# skip me").unwrap();
        let recipes = load(dir.path());
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].label, "Good");
    }

    #[test]
    fn load_parses_icon_field() {
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join(".folio/recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();
        std::fs::write(
            recipes_dir.join("r.toml"),
            "label = \"Recap\"\nprompt = \"p\"\nicon = \"calendar\"\n",
        )
        .unwrap();
        let recipes = load(dir.path());
        assert_eq!(recipes[0].icon.as_deref(), Some("calendar"));
    }
}
