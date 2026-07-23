use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use ts_rs::TS;

use crate::storage::session::TRANSCRIPT_FILENAME;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct PurgeSummary {
    pub sessions_inspected: usize,
    pub wavs_deleted: usize,
    pub bytes_freed: u64,

    pub failed: Vec<PathBuf>,
}

pub fn purge_old_wavs(recordings_dir: &Path, older_than_days: u32) -> PurgeSummary {
    let mut summary = PurgeSummary {
        sessions_inspected: 0,
        wavs_deleted: 0,
        bytes_freed: 0,
        failed: Vec::new(),
    };
    let entries = match fs::read_dir(recordings_dir) {
        Ok(e) => e,
        Err(_) => return summary,
    };
    let threshold = Duration::from_secs(older_than_days as u64 * 86_400);
    let now = SystemTime::now();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        summary.sessions_inspected += 1;

        if !path.join(TRANSCRIPT_FILENAME).is_file() && !path.join("transcript.json.zst").is_file()
        {
            continue;
        }

        if let Some(sync) = crate::server::sync_state::load(&path).ok().flatten() {
            if sync.upload_state != crate::server::sync_state::UploadPhase::Complete {
                continue;
            }
        }

        let mut wavs = Vec::with_capacity(2);
        for name in ["mic.wav", "system.wav"] {
            let wav = path.join(name);
            if wav.is_file() {
                wavs.push(wav);
            }
        }
        if wavs.is_empty() {
            continue;
        }

        let mut newest = SystemTime::UNIX_EPOCH;
        for wav in &wavs {
            if let Ok(meta) = fs::metadata(wav) {
                if let Ok(mt) = meta.modified() {
                    if mt > newest {
                        newest = mt;
                    }
                }
            }
        }
        let age = now.duration_since(newest).unwrap_or(Duration::ZERO);
        if age < threshold {
            continue;
        }

        for wav in &wavs {
            let bytes = fs::metadata(wav).ok().map(|m| m.len()).unwrap_or(0);
            match fs::remove_file(wav) {
                Ok(()) => {
                    summary.wavs_deleted += 1;
                    summary.bytes_freed += bytes;
                }
                Err(_) => {
                    summary.failed.push(path.clone());
                }
            }
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn touch_old(p: &Path, days_ago: u64) {
        let mut f = fs::File::create(p).unwrap();
        f.write_all(b"FAKE").unwrap();
        let mtime = SystemTime::now() - Duration::from_secs(days_ago * 86_400 + 3600);
        let ft = filetime::FileTime::from_system_time(mtime);
        filetime::set_file_mtime(p, ft).unwrap();
    }

    #[test]
    fn purge_skips_when_no_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("s1");
        fs::create_dir(&session).unwrap();
        touch_old(&session.join("mic.wav"), 30);

        let s = purge_old_wavs(dir.path(), 7);
        assert_eq!(s.sessions_inspected, 1);
        assert_eq!(s.wavs_deleted, 0);
        assert!(session.join("mic.wav").exists());
    }

    #[test]
    fn purge_skips_when_too_young() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("s1");
        fs::create_dir(&session).unwrap();
        touch_old(&session.join("mic.wav"), 1);
        fs::write(session.join(TRANSCRIPT_FILENAME), "{}").unwrap();

        let s = purge_old_wavs(dir.path(), 7);
        assert_eq!(s.wavs_deleted, 0);
        assert!(session.join("mic.wav").exists());
    }

    #[test]
    fn purge_deletes_old_wavs_when_transcript_exists() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("s1");
        fs::create_dir(&session).unwrap();
        touch_old(&session.join("mic.wav"), 30);
        touch_old(&session.join("system.wav"), 30);
        fs::write(session.join(TRANSCRIPT_FILENAME), "{}").unwrap();

        let s = purge_old_wavs(dir.path(), 7);
        assert_eq!(s.wavs_deleted, 2);
        assert!(s.bytes_freed >= 4);
        assert!(!session.join("mic.wav").exists());
        assert!(!session.join("system.wav").exists());
    }

    #[test]
    fn purge_accepts_zstd_transcripts_too() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("s1");
        fs::create_dir(&session).unwrap();
        touch_old(&session.join("mic.wav"), 30);
        fs::write(session.join("transcript.json.zst"), b"zstd").unwrap();

        let s = purge_old_wavs(dir.path(), 7);
        assert_eq!(s.wavs_deleted, 1);
    }
}
