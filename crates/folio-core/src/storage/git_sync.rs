use std::path::Path;
use std::process::Command;

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct GitSyncSummary {
    pub is_repo: bool,

    pub branch: String,

    pub pull_log: String,

    pub push_log: String,

    pub committed: bool,

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

    let branch_out = run(vault_dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    if branch_out.status_ok {
        out.branch = branch_out.stdout.trim().to_string();
    }

    let pull = run(vault_dir, &["pull", "--rebase", "--autostash", "--no-edit"]);
    out.pull_log = combined(&pull);
    if !pull.status_ok {
        return out;
    }

    let add = run(vault_dir, &["add", "-A"]);
    if !add.status_ok {
        out.pull_log.push_str("\n[git add -A failed]\n");
        out.pull_log.push_str(&combined(&add));
        return out;
    }

    let staged = run(vault_dir, &["diff", "--cached", "--quiet"]);
    if !staged.status_ok {
        let commit = run(
            vault_dir,
            &["commit", "-m", "folio sync", "--no-verify", "--no-gpg-sign"],
        );
        if !commit.status_ok {
            out.pull_log.push_str("\n[git commit failed]\n");
            out.pull_log.push_str(&combined(&commit));
            return out;
        }
        out.committed = true;
    }

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
}
