use std::path::{Path, PathBuf};

use crate::error::{FolioError, Result};

pub fn canonicalize_under(root: &Path, candidate: &Path) -> Result<PathBuf> {
    let canon_root = std::fs::canonicalize(root).map_err(|e| {
        FolioError::Storage(format!(
            "could not canonicalize root {}: {e}",
            root.display()
        ))
    })?;
    let canon_target = std::fs::canonicalize(candidate).map_err(|e| {
        FolioError::Storage(format!(
            "could not canonicalize {}: {e}",
            candidate.display()
        ))
    })?;
    if !canon_target.starts_with(&canon_root) {
        return Err(FolioError::Storage(format!(
            "refused: {} is not under {}",
            canon_target.display(),
            canon_root.display()
        )));
    }
    Ok(canon_target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_paths_under_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("inside.txt");
        std::fs::write(&child, b"hello").unwrap();
        let canon = canonicalize_under(dir.path(), &child).unwrap();
        assert!(canon.ends_with("inside.txt"));
    }

    #[test]
    fn rejects_paths_outside_the_root() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let outside = other.path().join("outside.txt");
        std::fs::write(&outside, b"hello").unwrap();
        let err = canonicalize_under(root.path(), &outside).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("refused"));
    }

    #[test]
    fn rejects_missing_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let err = canonicalize_under(dir.path(), &missing).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("could not canonicalize"));
    }

    #[test]
    fn rejects_symlink_escape() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        std::fs::write(&secret, b"shh").unwrap();
        let link = root.path().join("escape");
        if std::os::unix::fs::symlink(&secret, &link).is_ok() {
            let err = canonicalize_under(root.path(), &link).unwrap_err();
            assert!(format!("{err}").contains("refused"));
        }
    }
}
