//! Vault-native .attune/ layout. v2 roadmap finding 056 / GET-91.
//!
//! When the user has a vault (an Obsidian / git directory they
//! already manage), Attune can stay entirely within `<vault>/.attune/`
//! so a single `git push` snapshots the whole brain.
//!
//! ```text
//! <vault>/
//!   .attune/
//!     recordings/     # session_dirs (mic.wav, system.wav, transcript.json.zst, …)
//!     memory/         # canonical memory markdown files
//!     tasks/tasks.json
//!     digests/        # weekly markdown digests
//!     inbox/, outbox/ # filesystem IPC (#073 / GET-75)
//!     showcase.md     # Linktree-style portfolio (#087 / GET-107)
//!     webhooks.json   # subscriptions (#079 / GET-101)
//!     _index/         # derived SQLite + FTS5 + vec indexes (gitignored)
//! ```
//!
//! This module is the single resolver: given a vault root, returns
//! every path the app reads/writes. Callers that don't have a
//! vault keep using the legacy split (output_dir, memory_dir,
//! tasks_path, …) per Settings; the layout migration is opt-in.

use std::path::{Path, PathBuf};

/// Computed paths beneath `<vault>/.attune/`.
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

    /// Suggested `.gitignore` contents for the layout. Only the
    /// `_index/` directory is gitignored — everything else is the
    /// user's source of truth they want versioned.
    pub fn suggested_gitignore() -> &'static str {
        "# Attune vault gitignore (v2 #056 / GET-91)\n_index/\n"
    }

    /// `true` iff every Attune-managed path inside the vault root
    /// already exists. The Settings UI uses this to decide whether
    /// to surface a 'Run migration' prompt.
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
