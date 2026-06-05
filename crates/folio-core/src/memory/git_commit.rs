use std::path::Path;
use std::process::Command;

pub const COMMIT_DEBOUNCE_SECS: u64 = 30;
pub const COMMIT_PREFIX: &str = "folio-memory";

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

pub fn message(verb: MemoryVerb, slug: &str) -> String {
    format!("{COMMIT_PREFIX}: {} {}", verb.as_str(), slug)
}

pub fn is_git_repo(dir: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn commit_path(
    dir: &Path,
    path: &Path,
    verb: MemoryVerb,
    slug: &str,
) -> crate::error::Result<String> {
    if !is_git_repo(dir) {
        return Ok(String::new());
    }

    let add = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("add")
        .arg(path)
        .output()
        .map_err(|e| crate::error::FolioError::Storage(format!("git add: {e}")))?;
    if !add.status.success() {
        return Err(crate::error::FolioError::Storage(format!(
            "git add failed: {}",
            String::from_utf8_lossy(&add.stderr)
        )));
    }

    let diff = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["diff", "--cached", "--quiet"])
        .output()
        .map_err(|e| crate::error::FolioError::Storage(format!("git diff: {e}")))?;
    if diff.status.success() {
        return Ok(String::new());
    }

    let msg = message(verb, slug);
    let commit = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["commit", "-m", &msg])
        .output()
        .map_err(|e| crate::error::FolioError::Storage(format!("git commit: {e}")))?;
    if !commit.status.success() {
        return Err(crate::error::FolioError::Storage(format!(
            "git commit failed: {}",
            String::from_utf8_lossy(&commit.stderr)
        )));
    }

    let head = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| crate::error::FolioError::Storage(format!("git rev-parse: {e}")))?;
    Ok(String::from_utf8_lossy(&head.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_uses_the_canonical_prefix() {
        assert_eq!(
            message(MemoryVerb::Update, "user.company"),
            "folio-memory: UPDATE user.company"
        );
        assert_eq!(
            message(MemoryVerb::Create, "person.alice"),
            "folio-memory: CREATE person.alice"
        );
        assert_eq!(
            message(MemoryVerb::Delete, "ui.theme"),
            "folio-memory: DELETE ui.theme"
        );
        assert_eq!(
            message(MemoryVerb::Supersede, "x"),
            "folio-memory: SUPERSEDE x"
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

        for args in [
            &["init", "-q", "-b", "main"][..],
            &["config", "user.email", "test@folio.local"][..],
            &["config", "user.name", "Folio Test"][..],
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
        std::fs::write(&file, "Folio\n").unwrap();
        let sha = commit_path(dir.path(), &file, MemoryVerb::Create, "user.company").unwrap();
        assert_eq!(sha.len(), 40, "expected 40-char SHA, got {sha:?}");

        let again = commit_path(dir.path(), &file, MemoryVerb::Update, "user.company").unwrap();
        assert_eq!(again, "");
    }
}
