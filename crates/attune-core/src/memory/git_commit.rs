//! Avid Brain pattern — every memory write is a tiny git commit
//! against the memory directory. v2 finding 039 / GET-60.
//!
//! Each `MemoryStore::create/update/delete` produces a markdown change
//! on disk; this module records the change with `git -C <dir> commit
//! -m "attune-memory: <verb> <slug>"`. Once that habit is in place:
//!
//!   * `git log` shows which meeting changed which belief and when.
//!   * Conflict resolution between local edits and a sync pull
//!     becomes plain-old `git revert <sha>`.
//!   * `git blame` on any memory file points at the meeting that
//!     introduced the claim.
//!
//! Commits are debounced 30s in the wire-up layer (the store calls
//! `enqueue(path, verb, slug)` and a background ticker flushes a
//! coalesced commit). The pure helpers here build the commit message,
//! decide whether the directory is a git repo, and shell out to git.
//! Everything is std::process::Command — no libgit2.

use std::path::Path;
use std::process::Command;

pub const COMMIT_DEBOUNCE_SECS: u64 = 30;
pub const COMMIT_PREFIX: &str = "attune-memory";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryVerb {
    Create,
    Update,
    Delete,
    Supersede,
}

impl MemoryVerb {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryVerb::Create => "CREATE",
            MemoryVerb::Update => "UPDATE",
            MemoryVerb::Delete => "DELETE",
            MemoryVerb::Supersede => "SUPERSEDE",
        }
    }
}

/// Build the conventional commit message used everywhere.
/// `attune-memory: UPDATE user.company` is what `git log` shows.
pub fn message(verb: MemoryVerb, slug: &str) -> String {
    format!("{COMMIT_PREFIX}: {} {}", verb.as_str(), slug)
}

/// `true` iff `dir` (or one of its parents) is the root of a git
/// repository. Shells out to `git rev-parse --is-inside-work-tree`
/// which is cheap and gives us the answer the user expects (a vault
/// inside a larger monorepo counts).
pub fn is_git_repo(dir: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Stage + commit the given path. Returns the commit SHA on success.
/// No-ops (nothing-to-commit) are NOT errors — they return `Ok("")`.
///
/// # Errors
///
/// Returns `Err` when any git subprocess fails to spawn or returns a
/// non-zero exit code.
pub fn commit_path(
    dir: &Path,
    path: &Path,
    verb: MemoryVerb,
    slug: &str,
) -> crate::error::Result<String> {
    if !is_git_repo(dir) {
        return Ok(String::new());
    }
    // git add <path>
    let add = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("add")
        .arg(path)
        .output()
        .map_err(|e| crate::error::AttuneError::Storage(format!("git add: {e}")))?;
    if !add.status.success() {
        return Err(crate::error::AttuneError::Storage(format!(
            "git add failed: {}",
            String::from_utf8_lossy(&add.stderr)
        )));
    }
    // git diff --cached --quiet: exit 0 = nothing staged. Skip in that case.
    let diff = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["diff", "--cached", "--quiet"])
        .output()
        .map_err(|e| crate::error::AttuneError::Storage(format!("git diff: {e}")))?;
    if diff.status.success() {
        return Ok(String::new());
    }
    // git commit -m "..."
    let msg = message(verb, slug);
    let commit = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["commit", "-m", &msg])
        .output()
        .map_err(|e| crate::error::AttuneError::Storage(format!("git commit: {e}")))?;
    if !commit.status.success() {
        return Err(crate::error::AttuneError::Storage(format!(
            "git commit failed: {}",
            String::from_utf8_lossy(&commit.stderr)
        )));
    }
    // git rev-parse HEAD — return the new SHA so callers can log /
    // surface it in the UI ("committed abc1234").
    let head = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| crate::error::AttuneError::Storage(format!("git rev-parse: {e}")))?;
    Ok(String::from_utf8_lossy(&head.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_uses_the_canonical_prefix() {
        assert_eq!(
            message(MemoryVerb::Update, "user.company"),
            "attune-memory: UPDATE user.company"
        );
        assert_eq!(
            message(MemoryVerb::Create, "person.alice"),
            "attune-memory: CREATE person.alice"
        );
        assert_eq!(
            message(MemoryVerb::Delete, "ui.theme"),
            "attune-memory: DELETE ui.theme"
        );
        assert_eq!(
            message(MemoryVerb::Supersede, "x"),
            "attune-memory: SUPERSEDE x"
        );
    }

    #[test]
    fn is_git_repo_false_for_plain_tmpdir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_git_repo(dir.path()));
    }

    #[test]
    fn commit_path_noops_when_dir_is_not_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("foo.md");
        std::fs::write(&file, "hello").unwrap();
        let sha = commit_path(dir.path(), &file, MemoryVerb::Create, "foo").unwrap();
        assert_eq!(sha, "", "no repo → no commit, no error");
    }

    #[test]
    fn commit_path_writes_real_commit_in_a_real_repo() {
        let dir = tempfile::tempdir().unwrap();
        // Init repo + minimal identity so commit doesn't refuse.
        for args in [
            &["init", "-q", "-b", "main"][..],
            &["config", "user.email", "test@attune.local"][..],
            &["config", "user.name", "Attune Test"][..],
            &["config", "commit.gpgsign", "false"][..],
        ] {
            let status = Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .status()
                .expect("git available");
            assert!(status.success(), "git {args:?} failed");
        }
        assert!(is_git_repo(dir.path()));
        let file = dir.path().join("user.company.md");
        std::fs::write(&file, "Attune\n").unwrap();
        let sha = commit_path(dir.path(), &file, MemoryVerb::Create, "user.company").unwrap();
        assert_eq!(sha.len(), 40, "expected 40-char SHA, got {sha:?}");

        // Second commit with no change → noop, empty SHA.
        let again = commit_path(dir.path(), &file, MemoryVerb::Update, "user.company").unwrap();
        assert_eq!(again, "");
    }
}
