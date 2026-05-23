//! Recording-session metadata: scans the on-disk session directories
//! produced by the capture pipeline and reports a summary per session.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Filename of the transcript JSON that the transcription pipeline
/// writes inside each session directory.
pub const TRANSCRIPT_FILENAME: &str = "transcript.json";

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
    /// True iff `<session_dir>/transcript.json` exists. Used by the UI
    /// to mark previously transcribed sessions in the library list.
    pub has_transcript: bool,
}

/// Scan `output_dir` for recording sessions and return one summary per
/// session, sorted newest first by filesystem creation time (falling
/// back to label-descending when the platform did not give us a
/// create-time, which keeps timestamp-named sessions in the right
/// order). Directories without any WAV file are skipped.
///
/// We deliberately do NOT sort by label alone — labels happen to be
/// chronological for sessions Attune created itself (e.g.
/// "2026-05-23-19-15-22"), but imported or hand-named sessions like
/// "2026-05-23-mark-cuban-yahoo-trade" break that assumption. The
/// filesystem mtime/ctime is the canonical source of "when this
/// landed in the library."
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
        let has_transcript = path.join(TRANSCRIPT_FILENAME).is_file();
        out.push(RecordingSummary {
            session_dir: path,
            label,
            duration_seconds,
            mic_bytes,
            system_bytes,
            mic_sample_rate,
            system_sample_rate,
            created_at,
            has_transcript,
        });
    }
    // Newest first. Recordings that have a created_at sort by that;
    // anything missing the timestamp falls back to label-descending
    // and sorts after the dated ones so the user does not lose them
    // entirely.
    out.sort_by(|a, b| match (a.created_at, b.created_at) {
        (Some(x), Some(y)) => y.cmp(&x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => b.label.cmp(&a.label),
    });
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

    /// Imported sessions with non-timestamp labels (e.g. the
    /// "mark-cuban-yahoo-trade" Twitter import) used to sort
    /// lexicographically next to dated sessions, which put them in
    /// the wrong place in the library. The fix: sort by filesystem
    /// creation time. This test creates two sessions where the
    /// alphabetically-later session is the *older* one on disk, and
    /// asserts the newer one comes first.
    #[test]
    fn scan_sorts_by_created_at_not_label() {
        let dir = TempDir::new().unwrap();
        // "alpha" alphabetically beats "zulu", so a label-based sort
        // (descending) would put "zulu" first. We want the newer one
        // (alpha, created last) first regardless.
        let old_session = dir.path().join("zulu-session");
        let new_session = dir.path().join("alpha-session");
        std::fs::create_dir(&old_session).unwrap();
        // Write a 1-frame wav into each so scan_recordings includes them.
        write_minimal_wav(&old_session.join("mic.wav"));
        // Pause a beat so the filesystem can distinguish the two
        // creation times. macOS APFS records ctime at second
        // granularity in some configs.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::create_dir(&new_session).unwrap();
        write_minimal_wav(&new_session.join("mic.wav"));

        let result = scan_recordings(dir.path());
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].label, "alpha-session",
            "newer session should sort first, regardless of label"
        );
        assert_eq!(result[1].label, "zulu-session");
    }

    fn write_minimal_wav(path: &Path) {
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
}
