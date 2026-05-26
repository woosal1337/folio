//! Switcher import from Granola / Otter / Fathom. v2 finding 004 /
//! GET-44. One-click importer for competitor exports — neutralises
//! switching cost, the killer objection from anyone with two years
//! of meeting history.
//!
//! Each provider hands users a different export shape; this module
//! normalises them into an [`ImportedMeeting`] that the materialiser
//! writes into a new Attune session directory with a transcript and
//! agent-run sidecars pre-populated.
//!
//! Today's coverage is shape-detection + normalisation. The
//! per-provider zip readers (Granola's `meetings.json`, Otter's
//! `<meeting>/<transcript.json>` shape, Fathom's `*.csv` rows) live
//! in follow-up modules that feed normalised structs into the same
//! materialiser.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceProvider {
    Granola,
    Otter,
    Fathom,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedMeeting {
    pub source: SourceProvider,
    pub label: String,
    pub title: Option<String>,
    pub started_at: DateTime<Utc>,
    pub duration_seconds: f64,
    pub segments: Vec<ImportedSegment>,
    pub summary_markdown: Option<String>,
    pub action_items: Vec<String>,
}

/// Detect which provider an export archive came from by sniffing the
/// names of files inside. The actual zip reader is upstream; this
/// helper takes a list of entry paths and returns the best guess.
pub fn detect_source(entry_names: &[String]) -> SourceProvider {
    let lc: Vec<String> = entry_names.iter().map(|n| n.to_lowercase()).collect();
    if lc.iter().any(|n| n.contains("granola")) {
        return SourceProvider::Granola;
    }
    if lc.iter().any(|n| n.contains("otter") || n.ends_with(".otr")) {
        return SourceProvider::Otter;
    }
    if lc.iter().any(|n| n.contains("fathom") || n.ends_with(".vtt")) {
        return SourceProvider::Fathom;
    }
    SourceProvider::Generic
}

/// Materialise an imported meeting as a new Attune session directory.
/// Writes:
///   * `<root>/<safe_label>/transcript.json` with the segments.
///   * `<root>/<safe_label>/imported.json` carrying the full normalised
///     bundle so a follow-up agent pass can re-extract tasks /
///     memories / decisions from the canned content.
///
/// Idempotent: re-running on the same label overwrites both files.
pub fn materialise(recordings_root: &Path, meeting: &ImportedMeeting) -> Result<PathBuf> {
    let safe_label = safe_dir_name(&meeting.label);
    let session_dir = recordings_root.join(&safe_label);
    std::fs::create_dir_all(&session_dir).map_err(|e| {
        AttuneError::Storage(format!(
            "could not create session dir {}: {e}",
            session_dir.display()
        ))
    })?;
    write_atomic(
        &session_dir.join("imported.json"),
        &serde_json::to_string_pretty(meeting).map_err(|e| {
            AttuneError::Storage(format!("could not serialise imported meeting: {e}"))
        })?,
    )?;
    let transcript = transcript_json(meeting)?;
    write_atomic(&session_dir.join("transcript.json"), &transcript)?;
    Ok(session_dir)
}

fn transcript_json(meeting: &ImportedMeeting) -> Result<String> {
    #[derive(Serialize)]
    struct ChannelTranscript<'a> {
        channel: &'a str,
        language: Option<&'a str>,
        segments: Vec<TranscriptSegment<'a>>,
    }
    #[derive(Serialize)]
    struct TranscriptSegment<'a> {
        start_seconds: f64,
        end_seconds: f64,
        text: &'a str,
    }
    #[derive(Serialize)]
    struct SessionTranscript<'a> {
        channels: Vec<ChannelTranscript<'a>>,
    }
    let session = SessionTranscript {
        channels: vec![ChannelTranscript {
            channel: "imported",
            language: None,
            segments: meeting
                .segments
                .iter()
                .map(|s| TranscriptSegment {
                    start_seconds: s.start_seconds,
                    end_seconds: s.end_seconds,
                    text: &s.text,
                })
                .collect(),
        }],
    };
    serde_json::to_string_pretty(&session)
        .map_err(|e| AttuneError::Storage(format!("could not serialise transcript: {e}")))
}

fn write_atomic(final_path: &Path, body: &str) -> Result<()> {
    let tmp_path = final_path.with_extension("tmp");
    std::fs::write(&tmp_path, body).map_err(|e| {
        AttuneError::Storage(format!(
            "could not write {}: {e}",
            tmp_path.display()
        ))
    })?;
    std::fs::rename(&tmp_path, final_path).map_err(|e| {
        AttuneError::Storage(format!(
            "could not rename {}: {e}",
            final_path.display()
        ))
    })?;
    Ok(())
}

/// Sanitise a label for use as a directory name. Replaces every
/// path-separator and control character with `_`, collapses repeated
/// underscores, and trims to 80 characters so the resulting path is
/// stable across filesystems.
pub fn safe_dir_name(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut prev_underscore = false;
    for ch in label.chars() {
        let safe = if ch.is_alphanumeric() || ch == '-' || ch == ' ' {
            ch
        } else {
            '_'
        };
        if safe == '_' {
            if prev_underscore {
                continue;
            }
            prev_underscore = true;
        } else {
            prev_underscore = false;
        }
        out.push(safe);
    }
    let out = out.trim_matches('_').to_string();
    out.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meeting() -> ImportedMeeting {
        ImportedMeeting {
            source: SourceProvider::Granola,
            label: "2024-12-04 / pricing tier review".into(),
            title: Some("Pricing tier review".into()),
            started_at: Utc::now(),
            duration_seconds: 1234.0,
            segments: vec![ImportedSegment {
                start_seconds: 0.0,
                end_seconds: 5.0,
                speaker: "Alice".into(),
                text: "Welcome everyone.".into(),
            }],
            summary_markdown: Some("Pricing decisions discussed.".into()),
            action_items: vec!["Update pricing deck".into()],
        }
    }

    #[test]
    fn detect_source_picks_granola_by_filename() {
        let names = vec![
            "GranolaExport_2024-12-04.zip".to_string(),
            "meetings.json".to_string(),
        ];
        assert_eq!(detect_source(&names), SourceProvider::Granola);
    }

    #[test]
    fn detect_source_picks_otter_by_extension() {
        let names = vec!["pricing.otr".to_string()];
        assert_eq!(detect_source(&names), SourceProvider::Otter);
    }

    #[test]
    fn detect_source_picks_fathom_by_vtt() {
        let names = vec!["call_2024.vtt".to_string()];
        assert_eq!(detect_source(&names), SourceProvider::Fathom);
    }

    #[test]
    fn detect_source_falls_back_to_generic() {
        let names = vec!["unknown_export.json".to_string()];
        assert_eq!(detect_source(&names), SourceProvider::Generic);
    }

    #[test]
    fn safe_dir_name_replaces_path_components_and_collapses_underscores() {
        let safe = safe_dir_name("2024-12-04 / pricing tier // review");
        assert!(!safe.contains('/'));
        assert!(!safe.contains("__"));
    }

    #[test]
    fn safe_dir_name_caps_at_80_chars() {
        let long = "a".repeat(200);
        assert!(safe_dir_name(&long).len() <= 80);
    }

    #[test]
    fn materialise_writes_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let session = materialise(dir.path(), &meeting()).unwrap();
        assert!(session.join("imported.json").is_file());
        assert!(session.join("transcript.json").is_file());
    }

    #[test]
    fn materialise_writes_a_legible_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let session = materialise(dir.path(), &meeting()).unwrap();
        let raw = std::fs::read_to_string(session.join("transcript.json")).unwrap();
        assert!(raw.contains("Welcome everyone"));
        assert!(raw.contains("imported"));
    }
}
