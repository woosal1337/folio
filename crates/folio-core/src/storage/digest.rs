use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use ts_rs::TS;

use crate::storage::session::scan_recordings;
use crate::storage::tasks::TaskStore;

const ONE_WEEK_SECS: u64 = 7 * 86_400;
const TASK_AGE_THRESHOLD_SECS: u64 = 7 * 86_400;

#[derive(Debug, Clone)]
pub struct DigestPaths {
    pub recordings_dir: PathBuf,
    pub memory_dir: PathBuf,
    pub tasks_path: PathBuf,
    pub digests_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct DigestResult {
    pub path: PathBuf,
    pub recordings: usize,
    pub aged_tasks: usize,
    pub new_memories: usize,
    pub bytes: u64,
}

pub fn generate(paths: &DigestPaths) -> std::io::Result<DigestResult> {
    fs::create_dir_all(&paths.digests_dir)?;
    let now = Utc::now();
    let week_ago = SystemTime::now() - Duration::from_secs(ONE_WEEK_SECS);
    let week_ago_chrono: DateTime<Utc> = week_ago.into();

    let recordings: Vec<_> = scan_recordings(&paths.recordings_dir)
        .into_iter()
        .filter(|r| match r.created_at {
            Some(t) => t > week_ago_chrono,
            None => false,
        })
        .collect();

    let tasks = TaskStore::new(paths.tasks_path.clone()).list();
    let aged_tasks: Vec<_> = tasks
        .into_iter()
        .filter(|t| {
            let status = serde_json::to_string(&t.status)
                .map(|s| s.trim_matches('"').to_string())
                .unwrap_or_default();
            if status == "done" {
                return false;
            }

            true
        })
        .collect();

    let new_memories: Vec<_> = crate::memory::page::read_dir_pages(&paths.memory_dir)
        .into_iter()
        .filter(|m| m.created_at > week_ago_chrono)
        .collect();

    let mut out = String::new();
    let date_label = now.with_timezone(&Local).format("%Y-%m-%d");
    out.push_str(&format!("# Weekly digest — {date_label}\n\n"));

    out.push_str(&format!(
        "_{} recording{} · {} aged task{} · {} new memory{}_\n\n",
        recordings.len(),
        plural(recordings.len()),
        aged_tasks.len(),
        plural(aged_tasks.len()),
        new_memories.len(),
        if new_memories.len() == 1 { "y" } else { "ies" }
    ));

    out.push_str("## Recordings this week\n\n");
    if recordings.is_empty() {
        out.push_str("_None._\n\n");
    } else {
        for r in &recordings {
            let title = r.suggested_title.as_deref().unwrap_or(&r.label);
            let when = r
                .created_at
                .map(|t| t.with_timezone(&Local).format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "unknown".into());
            out.push_str(&format!(
                "- **{when}** · {title} ({} sec)\n",
                r.duration_seconds
            ));
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "## Open tasks aging > {} days\n\n",
        TASK_AGE_THRESHOLD_SECS / 86_400
    ));
    if aged_tasks.is_empty() {
        out.push_str("_None._\n\n");
    } else {
        for t in &aged_tasks {
            let owner = t.owner.as_deref().unwrap_or("-");
            let due = t.due.as_deref().unwrap_or("-");
            out.push_str(&format!(
                "- [ ] {} _(owner: {owner}, due: {due})_\n",
                t.title
            ));
        }
        out.push('\n');
    }

    out.push_str("## New memories\n\n");
    if new_memories.is_empty() {
        out.push_str("_None._\n\n");
    } else {
        for m in &new_memories {
            let key = m.key.as_deref().unwrap_or("observation");
            out.push_str(&format!("- **{key}** — {}\n", m.content.trim()));
        }
        out.push('\n');
    }

    let filename = format!("{date_label}.md");
    let path = paths.digests_dir.join(filename);
    fs::write(&path, &out)?;

    Ok(DigestResult {
        path: path.clone(),
        recordings: recordings.len(),
        aged_tasks: aged_tasks.len(),
        new_memories: new_memories.len(),
        bytes: fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
    })
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

pub fn default_digests_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Documents").join("Folio").join("Digests")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_minimal_wav(path: &std::path::Path) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        writer.write_sample(0i16).unwrap();
        writer.finalize().unwrap();
    }

    #[test]
    fn generate_writes_markdown_with_empty_sections() {
        let dir = tempfile::tempdir().unwrap();
        let recordings = dir.path().join("recordings");
        let memory = dir.path().join("memory");
        let tasks = dir.path().join("tasks.json");
        let digests = dir.path().join("digests");
        fs::create_dir_all(&recordings).unwrap();
        fs::create_dir_all(&memory).unwrap();
        fs::write(&tasks, "[]").unwrap();

        let res = generate(&DigestPaths {
            recordings_dir: recordings,
            memory_dir: memory,
            tasks_path: tasks,
            digests_dir: digests.clone(),
        })
        .unwrap();

        assert!(res.path.starts_with(&digests));
        let body = fs::read_to_string(&res.path).unwrap();
        assert!(body.contains("# Weekly digest"));
        assert!(body.contains("## Recordings this week"));
        assert!(body.contains("## Open tasks"));
        assert!(body.contains("## New memories"));
        assert_eq!(res.recordings, 0);
        assert_eq!(res.new_memories, 0);
    }

    #[test]
    fn generate_includes_recent_recording() {
        let dir = tempfile::tempdir().unwrap();
        let recordings = dir.path().join("recordings");
        let session = recordings.join("2026-05-25-test");
        fs::create_dir_all(&session).unwrap();
        write_minimal_wav(&session.join("mic.wav"));
        let res = generate(&DigestPaths {
            recordings_dir: recordings,
            memory_dir: dir.path().join("memory"),
            tasks_path: dir.path().join("tasks.json"),
            digests_dir: dir.path().join("digests"),
        })
        .unwrap();
        assert_eq!(res.recordings, 1);
        let body = fs::read_to_string(&res.path).unwrap();
        assert!(body.contains("2026-05-25-test"));
    }
}
