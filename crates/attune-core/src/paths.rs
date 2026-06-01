//! Path containment helpers. `docs/CODE_STYLE.md` §8.1 requires every
//! command that accepts a path to canonicalise it and verify it lives
//! under the configured root. `strip_prefix`/`starts_with` defences
//! defeat under symlinks; canonicalisation closes that gap.

use std::path::{Path, PathBuf};

use crate::error::{AttuneError, Result};

/// Canonicalise `candidate` and assert it lives under `root` (which
/// is itself canonicalised). Returns the canonicalised candidate on
/// success, or an `AttuneError::Storage` describing the violation.
///
/// Both `root` and `candidate` must exist on disk — `canonicalize`
/// resolves symlinks and verifies presence. Callers that need to
/// validate a not-yet-created child path should canonicalise the
/// parent + verify, then join the leaf themselves.
///
/// # Errors
///
/// Returns `Err` if `candidate` escapes `root` or the path cannot be resolved.
pub fn canonicalize_under(root: &Path, candidate: &Path) -> Result<PathBuf> {
    let canon_root = std::fs::canonicalize(root).map_err(|e| {
        AttuneError::Storage(format!(
            "could not canonicalize root {}: {e}",
            root.display()
        ))
    })?;
    let canon_target = std::fs::canonicalize(candidate).map_err(|e| {
        AttuneError::Storage(format!(
            "could not canonicalize {}: {e}",
            candidate.display()
        ))
    })?;
    if !canon_target.starts_with(&canon_root) {
        return Err(AttuneError::Storage(format!(
            "refused: {} is not under {}",
            canon_target.display(),
            canon_root.display()
        )));
    }
    Ok(canon_target)
}

/// Canonicalise `candidate` and assert it lives under any of the
/// supplied roots. Useful when the call site allows multiple
/// canonical roots (recordings root + memory root, say).
///
/// # Errors
///
/// Returns `Err` if `candidate` cannot be resolved or does not fall under any of `roots`.
pub fn canonicalize_under_any(roots: &[&Path], candidate: &Path) -> Result<PathBuf> {
    let canon_target = std::fs::canonicalize(candidate).map_err(|e| {
        AttuneError::Storage(format!(
            "could not canonicalize {}: {e}",
            candidate.display()
        ))
    })?;
    for root in roots {
        if let Ok(canon_root) = std::fs::canonicalize(root) {
            if canon_target.starts_with(&canon_root) {
                return Ok(canon_target);
            }
        }
    }
    Err(AttuneError::Storage(format!(
        "refused: {} is not under any allowed root ({})",
        canon_target.display(),
        roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
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

    #[test]
    fn under_any_accepts_first_matching_root() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let child = b.path().join("inside.txt");
        std::fs::write(&child, b"hi").unwrap();
        let canon = canonicalize_under_any(&[a.path(), b.path()], &child).unwrap();
        assert!(canon.starts_with(std::fs::canonicalize(b.path()).unwrap()));
    }

    #[test]
    fn under_any_rejects_when_no_root_matches() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let leaf = outside.path().join("x.txt");
        std::fs::write(&leaf, b"hi").unwrap();
        let err = canonicalize_under_any(&[a.path(), b.path()], &leaf).unwrap_err();
        assert!(format!("{err}").contains("not under any allowed root"));
    }
}
