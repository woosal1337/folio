use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct VaultLayout {
    pub root: PathBuf,
    pub recordings: PathBuf,
    pub memory: PathBuf,
    pub tasks: PathBuf,
    pub digests: PathBuf,
    pub inbox: PathBuf,
    pub outbox: PathBuf,
    pub index: PathBuf,
}

impl VaultLayout {
    pub fn from_vault_root(vault_root: &Path) -> Self {
        let root = vault_root.join(".attune");
        VaultLayout {
            recordings: root.join("recordings"),
            memory: root.join("memory"),
            tasks: root.join("tasks").join("tasks.json"),
            digests: root.join("digests"),
            inbox: root.join("inbox"),
            outbox: root.join("outbox"),
            index: root.join("_index"),
            root,
        }
    }

    pub fn suggested_gitignore() -> &'static str {
        "# Attune vault gitignore\n_index/\n"
    }

    pub fn looks_initialised(&self) -> bool {
        self.recordings.is_dir()
            && self.memory.is_dir()
            && self.tasks.parent().map(|p| p.is_dir()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_vault_root_places_everything_under_dot_attune() {
        let layout = VaultLayout::from_vault_root(Path::new("/home/me/vault"));
        assert_eq!(layout.root, Path::new("/home/me/vault/.attune"));
        assert_eq!(
            layout.recordings,
            Path::new("/home/me/vault/.attune/recordings")
        );
        assert_eq!(
            layout.tasks,
            Path::new("/home/me/vault/.attune/tasks/tasks.json")
        );
        assert_eq!(layout.index, Path::new("/home/me/vault/.attune/_index"));
    }

    #[test]
    fn suggested_gitignore_only_excludes_index() {
        let g = VaultLayout::suggested_gitignore();
        assert!(g.contains("_index/"));
        assert!(!g.contains("memory"));
        assert!(!g.contains("recordings"));
    }

    #[test]
    fn looks_initialised_returns_false_for_empty_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let layout = VaultLayout::from_vault_root(dir.path());
        assert!(!layout.looks_initialised());
    }
}
