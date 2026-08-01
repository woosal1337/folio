use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const TRANSCRIPT_FILENAME: &str = "transcript.json";

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

    pub has_transcript: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_title: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_tags: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_subtitle: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_override: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<crate::server::sync_state::SyncState>,
}

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

        let is_note = path.join("live_notes.json").is_file();
        if mic_bytes.is_none() && system_bytes.is_none() && !is_note {
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
        let has_transcript = path.join(TRANSCRIPT_FILENAME).is_file()
            || path.join(format!("{TRANSCRIPT_FILENAME}.zst")).is_file();
        let autoname = read_autoname_run(&path);
        let language_override = read_first_line(&path, "language.txt");
        let title = read_first_line(&path, "title.txt");
        let draft_name = read_first_line(&path, "draft.txt");
        let sync = crate::server::sync_state::load(&path).ok().flatten();
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
            suggested_title: autoname.as_ref().and_then(|n| {
                if n.title.trim().is_empty() {
                    None
                } else {
                    Some(n.title.clone())
                }
            }),
            suggested_tags: autoname
                .as_ref()
                .map(|n| n.tags.clone())
                .unwrap_or_default(),
            suggested_subtitle: autoname.as_ref().and_then(|n| {
                if n.subtitle.trim().is_empty() {
                    None
                } else {
                    Some(n.subtitle.clone())
                }
            }),
            language_override,
            title,
            draft_name,
            sync,
        });
    }

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

pub(crate) fn read_first_line(session_dir: &Path, filename: &str) -> Option<String> {
    let raw = std::fs::read_to_string(session_dir.join(filename)).ok()?;
    let trimmed = raw.lines().next()?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn allocate_draft_name(output_dir: &Path) -> String {
    let path = output_dir.join(".folio").join("draft_counter");
    let last = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let next = last + 1;
    let _ = crate::storage::atomic_write::atomic_write(&path, next.to_string().as_bytes());
    format!("Draft {next}")
}

#[derive(Debug, Clone, Deserialize)]
struct AutonameRun {
    #[serde(default)]
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    subtitle: String,
}

fn read_autoname_run(session_dir: &Path) -> Option<AutonameRun> {
    let path = session_dir.join("agent_runs").join("autoname.json");
    let raw = std::fs::read_to_string(&path).ok()?;

    #[derive(Deserialize)]
    struct OuterRun {
        response: String,
    }
    let outer: OuterRun = serde_json::from_str(&raw).ok()?;
    let response = outer.response;
    let json_slice = extract_json_object(&response)?;
    serde_json::from_str::<AutonameRun>(json_slice).ok()
}

fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
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

    #[test]
    fn scan_sorts_by_created_at_not_label() {
        let dir = TempDir::new().unwrap();

        let old_session = dir.path().join("zulu-session");
        let new_session = dir.path().join("alpha-session");
        std::fs::create_dir(&old_session).unwrap();

        write_minimal_wav(&old_session.join("mic.wav"));

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

    #[test]
    fn scan_lifts_autoname_suggestion_into_summary() {
        let dir = TempDir::new().unwrap();
        let session = dir.path().join("2026-05-25-pricing");
        std::fs::create_dir(&session).unwrap();
        write_minimal_wav(&session.join("mic.wav"));
        let runs = session.join("agent_runs");
        std::fs::create_dir_all(&runs).unwrap();
        let run_json = r#"{
            "agent_id": "autoname",
            "agent_name": "Auto-name",
            "provider": "openai",
            "model": "gpt-4o-mini",
            "response": "{\"title\":\"Pricing sync with Lila\",\"tags\":[\"pricing\",\"sales\"],\"subtitle\":\"Q3 packaging review\"}",
            "prompt_tokens": null,
            "completion_tokens": null,
            "finished_at": "2026-05-25T14:00:00Z"
        }"#;
        std::fs::write(runs.join("autoname.json"), run_json).unwrap();
        let result = scan_recordings(dir.path());
        assert_eq!(result.len(), 1);
        let s = &result[0];
        assert_eq!(s.suggested_title.as_deref(), Some("Pricing sync with Lila"));
        assert_eq!(
            s.suggested_tags,
            vec!["pricing".to_string(), "sales".to_string()]
        );
        assert_eq!(s.suggested_subtitle.as_deref(), Some("Q3 packaging review"));
    }

    #[test]
    fn scan_tolerates_autoname_wrapped_in_prose() {
        let dir = TempDir::new().unwrap();
        let session = dir.path().join("noisy-model");
        std::fs::create_dir(&session).unwrap();
        write_minimal_wav(&session.join("mic.wav"));
        let runs = session.join("agent_runs");
        std::fs::create_dir_all(&runs).unwrap();

        let response_text = "Sure, here's the JSON:\n```json\n{\"title\":\"Standup\",\"tags\":[\"sync\"],\"subtitle\":\"Daily kickoff\"}\n```";
        let run_json = serde_json::json!({
            "agent_id": "autoname",
            "agent_name": "Auto-name",
            "provider": "openai",
            "model": "gpt-4o-mini",
            "response": response_text,
            "prompt_tokens": null,
            "completion_tokens": null,
            "finished_at": "2026-05-25T14:00:00Z",
        });
        std::fs::write(runs.join("autoname.json"), run_json.to_string()).unwrap();
        let result = scan_recordings(dir.path());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].suggested_title.as_deref(), Some("Standup"));
        assert_eq!(result[0].suggested_tags, vec!["sync".to_string()]);
    }

    #[test]
    fn scan_drops_empty_title_suggestion() {
        let dir = TempDir::new().unwrap();
        let session = dir.path().join("too-short");
        std::fs::create_dir(&session).unwrap();
        write_minimal_wav(&session.join("mic.wav"));
        let runs = session.join("agent_runs");
        std::fs::create_dir_all(&runs).unwrap();
        let run_json = serde_json::json!({
            "agent_id": "autoname",
            "agent_name": "Auto-name",
            "provider": "openai",
            "model": "gpt-4o-mini",
            "response": "{\"title\":\"\",\"tags\":[],\"subtitle\":\"\"}",
            "prompt_tokens": null,
            "completion_tokens": null,
            "finished_at": "2026-05-25T14:00:00Z",
        });
        std::fs::write(runs.join("autoname.json"), run_json.to_string()).unwrap();
        let result = scan_recordings(dir.path());
        assert_eq!(result.len(), 1);
        assert!(result[0].suggested_title.is_none());
        assert!(result[0].suggested_subtitle.is_none());
        assert!(result[0].suggested_tags.is_empty());
    }

    #[test]
    fn has_transcript_detects_zstd_compressed_files() {
        let dir = TempDir::new().unwrap();
        let session = dir.path().join("2026-05-25-zstd");
        std::fs::create_dir(&session).unwrap();
        write_minimal_wav(&session.join("mic.wav"));
        std::fs::write(session.join("transcript.json.zst"), b"FAKE").unwrap();
        let result = scan_recordings(dir.path());
        assert_eq!(result.len(), 1);
        assert!(
            result[0].has_transcript,
            "transcript.json.zst should count as a transcript"
        );
    }

    #[test]
    fn has_transcript_still_detects_legacy_uncompressed_files() {
        let dir = TempDir::new().unwrap();
        let session = dir.path().join("2026-05-25-legacy");
        std::fs::create_dir(&session).unwrap();
        write_minimal_wav(&session.join("mic.wav"));
        std::fs::write(session.join("transcript.json"), b"{}").unwrap();
        let result = scan_recordings(dir.path());
        assert_eq!(result.len(), 1);
        assert!(result[0].has_transcript);
    }
}
