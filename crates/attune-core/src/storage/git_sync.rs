//! Multi-machine sync via the user's own git remote.
//!
//! Attune doesn't run a cloud — if the vault directory is a git
//! repository, sync is the user's. We shell out to the system `git`
//! binary (no libgit2 dep, no in-process libssh, no cred dance) and
//! run the moral equivalent of:
//!
//! ```sh
//! git -C <vault> pull --rebase --autostash
//! git -C <vault> add -A
//! git -C <vault> commit -m "attune sync"  # only if dirty
//! git -C <vault> push
//! ```
//!
//! Returns a structured summary so the UI can show "pulled 3, pushed 2"
//! without parsing porcelain itself.
//!
//! v2 roadmap finding 070 / GET-72.

use std::path::Path;
use std::process::Command;

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct GitSyncSummary {
    /// Was the directory a git repo at all? When false, every other
    /// field is None / 0 / "" — the caller surfaces a hint to the
    /// user instead of a sync result.
    pub is_repo: bool,
    /// Branch the sync targeted.
    pub branch: String,
    /// stdout/stderr of the underlying git pull, useful for the
    /// 'why did it fail' tooltip the UI shows on error.
    pub pull_log: String,
    /// Same for the push half.
    pub push_log: String,
    /// True when the commit step ran (i.e. the working tree had
    /// uncommitted Attune-side changes worth pushing).
    pub committed: bool,
    /// True when the whole flow completed end-to-end. False means
    /// one of pull / commit / push failed; the user reads pull_log /
    /// push_log to find out which.
    pub ok: bool,
}

pub fn sync(vault_dir: &Path) -> GitSyncSummary {
    let mut out = GitSyncSummary {
        is_repo: false,
        branch: String::new(),
        pull_log: String::new(),
        push_log: String::new(),
        committed: false,
        ok: false,
    };

    if !vault_dir.is_dir() {
        return out;
    }
    if !vault_dir.join(".git").exists() {
        return out;
    }
    out.is_repo = true;

    // Read the current branch — useful in the UI even if pull fails.
    let branch_out = run(vault_dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    if branch_out.status_ok {
        out.branch = branch_out.stdout.trim().to_string();
    }

    // 1) Pull with rebase + autostash so a local uncommitted edit
    //    doesn't block the pull.
    let pull = run(vault_dir, &["pull", "--rebase", "--autostash", "--no-edit"]);
    out.pull_log = combined(&pull);
    if !pull.status_ok {
        return out;
    }

    // 2) Stage everything Attune writes (markdown memories, settings
    //    if the user pointed the dir at one, etc.).
    let add = run(vault_dir, &["add", "-A"]);
    if !add.status_ok {
        out.pull_log.push_str("\n[git add -A failed]\n");
        out.pull_log.push_str(&combined(&add));
        return out;
    }

    // 3) Commit if and only if the index has changes. `git diff
    //    --cached --quiet` exits 0 if nothing staged, 1 if something
    //    staged. We use that exit code to decide whether to commit.
    let staged = run(vault_dir, &["diff", "--cached", "--quiet"]);
    if !staged.status_ok {
        let commit = run(
            vault_dir,
            &[
                "commit",
                "-m",
                "attune sync",
                "--no-verify",
                "--no-gpg-sign",
            ],
        );
        if !commit.status_ok {
            out.pull_log.push_str("\n[git commit failed]\n");
            out.pull_log.push_str(&combined(&commit));
            return out;
        }
        out.committed = true;
    }

    // 4) Push to the configured remote/upstream.
    let push = run(vault_dir, &["push"]);
    out.push_log = combined(&push);
    out.ok = push.status_ok;
    out
}

struct ProcessResult {
    status_ok: bool,
    stdout: String,
    stderr: String,
}

fn run(cwd: &Path, args: &[&str]) -> ProcessResult {
    match Command::new("git").current_dir(cwd).args(args).output() {
        Ok(out) => ProcessResult {
            status_ok: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(e) => ProcessResult {
            status_ok: false,
            stdout: String::new(),
            stderr: format!("could not run git {args:?}: {e}"),
        },
    }
}

fn combined(r: &ProcessResult) -> String {
    if r.stderr.trim().is_empty() {
        r.stdout.trim().to_string()
    } else if r.stdout.trim().is_empty() {
        r.stderr.trim().to_string()
    } else {
        format!("{}\n{}", r.stdout.trim(), r.stderr.trim())
    }
}

/// Convenience: check whether a directory looks like a git repo,
/// without running an actual command. The Settings UI uses this to
/// decide whether to show the Sync card.
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_returns_not_repo_for_plain_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = sync(dir.path());
        assert!(!result.is_repo);
        assert!(!result.ok);
    }

    #[test]
    fn is_git_repo_picks_up_dot_git_marker() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_git_repo(dir.path()));
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        assert!(is_git_repo(dir.path()));
    }

    // Sync round-trip against a real local remote would require a
    // local bare repo + a working tree — meaningful but slow. Add as
    // an integration test in a follow-up once the harness exists.
}
