//! Recording-session metadata: scans the on-disk session directories
//! produced by the capture pipeline and reports a summary per session.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Metadata about a saved recording session, as discovered on disk.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct RecordingSummary {
    pub session_dir: PathBuf,
    pub label: String,
    pub duration_seconds: i64,
    pub mic_bytes: Option<u64>,
    pub system_bytes: Option<u64>,
    pub mic_sample_rate: Option<u32>,
    pub system_sample_rate: Option<u32>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Scan `output_dir` for recording sessions and return one summary per
/// session, sorted newest first by label (the label is a timestamp).
/// Directories without any WAV file are skipped.
pub fn scan_recordings(output_dir: &Path) -> Vec<RecordingSummary> {
    let Ok(entries) = std::fs::read_dir(output_dir) else {
        return Vec::new();
    };

    let mut out: Vec<RecordingSummary> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let label = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "session".into());
        let mic_path = path.join("mic.wav");
        let system_path = path.join("system.wav");
        let mic_bytes = std::fs::metadata(&mic_path).ok().map(|m| m.len());
        let system_bytes = std::fs::metadata(&system_path).ok().map(|m| m.len());
        if mic_bytes.is_none() && system_bytes.is_none() {
            continue;
        }
        let mic_sample_rate = wav_sample_rate(&mic_path);
        let system_sample_rate = wav_sample_rate(&system_path);
        let duration_seconds = wav_duration_seconds(&mic_path)
            .or_else(|| wav_duration_seconds(&system_path))
            .unwrap_or(0);
        let created_at = entry
            .metadata()
            .ok()
            .and_then(|m| m.created().ok())
            .map(|t| {
                let dt: DateTime<Local> = t.into();
                dt.with_timezone(&Utc)
            });
        out.push(RecordingSummary {
            session_dir: path,
            label,
            duration_seconds,
            mic_bytes,
            system_bytes,
            mic_sample_rate,
            system_sample_rate,
            created_at,
        });
    }
    out.sort_by(|a, b| b.label.cmp(&a.label));
    out
}

fn wav_sample_rate(path: &Path) -> Option<u32> {
    Some(hound::WavReader::open(path).ok()?.spec().sample_rate)
}

fn wav_duration_seconds(path: &Path) -> Option<i64> {
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let frames = reader.duration() as u64;
    if spec.sample_rate == 0 {
        return None;
    }
    Some((frames / spec.sample_rate as u64) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scan_empty_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        assert!(scan_recordings(dir.path()).is_empty());
    }

    #[test]
    fn scan_missing_dir_returns_empty() {
        let path = PathBuf::from("/this/path/does/not/exist");
        assert!(scan_recordings(&path).is_empty());
    }

    #[test]
    fn scan_skips_dirs_without_wavs() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("2026-01-01-12-00-00")).unwrap();
        assert!(scan_recordings(dir.path()).is_empty());
    }
}
