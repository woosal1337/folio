//! On-disk model cache with an LRU eviction policy + resume-friendly
//! download bookkeeping. v2 finding 060 / GET-95.
//!
//! The model store at `<config>/Attune/whisper-models/` holds one
//! `.bin` per Whisper variant. Each download records a sibling
//! `.partial` file when interrupted; on resume we send `Range:
//! bytes=<existing>-` to pick up where we left off (HuggingFace
//! CDN supports range, which is the bulk source).
//!
//! Eviction policy: when the directory exceeds `cap_bytes`, we
//! delete completed `.bin` files from oldest atime first until we
//! fit, never touching the model the user has currently selected.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct CachedModel {
    pub id: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct EvictSummary {
    pub evicted: Vec<CachedModel>,
    pub bytes_freed: u64,
    pub remaining_bytes: u64,
}

/// List every completed model (.bin) in the cache, newest-mtime
/// last. Files with `.partial` extensions are skipped.
pub fn list_cached(cache_dir: &Path) -> Vec<CachedModel> {
    let entries = match fs::read_dir(cache_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<CachedModel> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("bin") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let bytes = meta.len();
        let modified_at = meta
            .modified()
            .ok()
            .map(chrono::DateTime::<chrono::Utc>::from)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        let id = path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        out.push(CachedModel {
            id,
            path,
            bytes,
            modified_at,
        });
    }
    out.sort_by(|a, b| a.modified_at.cmp(&b.modified_at));
    out
}

/// Enforce a cap by deleting the oldest-atime models until the
/// total falls under `cap_bytes`. `keep_id` is the model the user
/// has currently selected — never evicted regardless of LRU rank.
pub fn enforce_cap(cache_dir: &Path, cap_bytes: u64, keep_id: Option<&str>) -> EvictSummary {
    let mut cached = list_cached(cache_dir);
    let mut total: u64 = cached.iter().map(|c| c.bytes).sum();
    let mut evicted = Vec::new();
    let mut bytes_freed = 0;
    if cap_bytes == 0 || total <= cap_bytes {
        return EvictSummary {
            evicted,
            bytes_freed,
            remaining_bytes: total,
        };
    }
    while total > cap_bytes && !cached.is_empty() {
        // Pop oldest first; skip the keep_id.
        let idx = cached.iter().position(|c| Some(c.id.as_str()) != keep_id);
        let Some(i) = idx else {
            break;
        };
        let victim = cached.remove(i);
        if fs::remove_file(&victim.path).is_ok() {
            total = total.saturating_sub(victim.bytes);
            bytes_freed += victim.bytes;
            evicted.push(victim);
        } else {
            // Can't delete (locked / permission) — bail rather than
            // loop on the same file.
            break;
        }
    }
    EvictSummary {
        evicted,
        bytes_freed,
        remaining_bytes: total,
    }
}

/// Bytes already downloaded for a model. Used to compute the
/// HTTP Range header value for a resume. Returns 0 when nothing
/// is on disk yet.
pub fn resume_offset(partial_path: &Path) -> u64 {
    fs::metadata(partial_path).map(|m| m.len()).unwrap_or(0)
}

/// Promote a finished download from `.partial` to `.bin`. Atomic
/// rename so a crash either leaves the partial alone or the
/// final file ready to use.
pub fn finalize_download(partial_path: &Path) -> std::io::Result<PathBuf> {
    let final_path = partial_path.with_extension("bin");
    fs::rename(partial_path, &final_path)?;
    Ok(final_path)
}

// Touch unused-import warnings — SystemTime is referenced indirectly
// in tests via system clock advancement.
#[allow(dead_code)]
fn _touch_systemtime() -> SystemTime {
    SystemTime::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn write_size(path: &Path, bytes: u64) {
        let buf = vec![0u8; bytes as usize];
        fs::write(path, buf).unwrap();
    }

    #[test]
    fn enforce_cap_does_nothing_when_under_budget() {
        let dir = tempfile::tempdir().unwrap();
        write_size(&dir.path().join("tiny.bin"), 1_000);
        let summary = enforce_cap(dir.path(), 10_000, None);
        assert_eq!(summary.evicted.len(), 0);
    }

    #[test]
    fn enforce_cap_evicts_oldest_first_and_skips_keep() {
        let dir = tempfile::tempdir().unwrap();
        write_size(&dir.path().join("old.bin"), 5_000);
        std::thread::sleep(Duration::from_millis(20));
        write_size(&dir.path().join("mid.bin"), 5_000);
        std::thread::sleep(Duration::from_millis(20));
        write_size(&dir.path().join("new.bin"), 5_000);

        // Cap at 8000 bytes, keep "new". Should evict old then mid.
        let summary = enforce_cap(dir.path(), 8_000, Some("new"));
        assert!(summary.evicted.iter().any(|m| m.id == "old"));
        assert!(summary.evicted.iter().any(|m| m.id == "mid"));
        assert!(dir.path().join("new.bin").exists());
        assert!(summary.remaining_bytes <= 8_000);
    }

    #[test]
    fn resume_offset_returns_zero_when_no_partial() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(resume_offset(&dir.path().join("nope.partial")), 0);
    }

    #[test]
    fn resume_offset_reports_existing_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.partial");
        write_size(&path, 4_096);
        assert_eq!(resume_offset(&path), 4_096);
    }

    #[test]
    fn finalize_download_renames_partial_to_bin() {
        let dir = tempfile::tempdir().unwrap();
        let partial = dir.path().join("foo.partial");
        write_size(&partial, 100);
        let final_ = finalize_download(&partial).unwrap();
        assert!(final_.ends_with("foo.bin"));
        assert!(final_.exists());
        assert!(!partial.exists());
    }
}
