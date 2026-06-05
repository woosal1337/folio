use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::llm::agent_run::AgentRunStore;
use crate::storage::session::scan_recordings;
use crate::transcription::SessionTranscript;

#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct NoteSearchHit {
    pub session_dir: String,
    pub label: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    pub snippet: String,

    pub matched_in: String,
}

const SNIPPET_RADIUS: usize = 60;

pub fn search_notes(output_dir: &Path, query: &str) -> Vec<NoteSearchHit> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }

    let mut hits = Vec::new();
    for rec in scan_recordings(output_dir) {
        let title = rec
            .title
            .clone()
            .or_else(|| rec.suggested_title.clone())
            .filter(|t| !t.trim().is_empty());

        let mut candidates: Vec<(&str, String)> = Vec::new();
        if let Some(t) = &title {
            candidates.push(("title", t.clone()));
        }
        if let Some(summary) = read_summary_text(&rec.session_dir) {
            candidates.push(("summary", summary));
        }
        if let Some(notes) = read_live_notes_text(&rec.session_dir) {
            candidates.push(("notes", notes));
        }
        if let Some(transcript) = read_transcript_text(&rec.session_dir) {
            candidates.push(("transcript", transcript));
        }

        for (field, text) in candidates {
            if let Some(snippet) = find_snippet(&text, &needle, SNIPPET_RADIUS) {
                hits.push(NoteSearchHit {
                    session_dir: rec.session_dir.to_string_lossy().into_owned(),
                    label: rec.label.clone(),
                    title: title.clone(),
                    snippet,
                    matched_in: field.to_string(),
                });
                break;
            }
        }
    }
    hits
}

fn read_summary_text(session_dir: &Path) -> Option<String> {
    let runs = AgentRunStore::list(session_dir).ok()?;
    runs.into_iter()
        .find(|r| r.agent_id == "summarize")
        .map(|r| r.response)
}

fn read_live_notes_text(session_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(session_dir.join("live_notes.json")).ok()?;
    let lines: Vec<crate::live_notes::RawNoteLine> = serde_json::from_slice(&bytes).ok()?;
    let notes = crate::live_notes::parse_lines(&lines);
    if notes.is_empty() {
        return None;
    }
    Some(crate::live_notes::render_markdown(&notes))
}

fn read_transcript_text(session_dir: &Path) -> Option<String> {
    let path = session_dir.join("transcript.json");
    let transcript = SessionTranscript::read_json(&path).ok()?;
    let mut out = String::new();
    for channel in &transcript.channels {
        for seg in &channel.segments {
            out.push_str(&seg.text);
            out.push(' ');
        }
    }
    if out.trim().is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn find_snippet(haystack: &str, needle_lc: &str, radius: usize) -> Option<String> {
    if needle_lc.is_empty() {
        return None;
    }
    let hay_chars: Vec<char> = haystack.chars().collect();
    let hay_lc: Vec<char> = hay_chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let needle: Vec<char> = needle_lc.chars().collect();
    if needle.len() > hay_lc.len() {
        return None;
    }

    let match_at =
        (0..=hay_lc.len() - needle.len()).find(|&i| hay_lc[i..i + needle.len()] == needle[..])?;

    let start = match_at.saturating_sub(radius);
    let end = (match_at + needle.len() + radius).min(hay_chars.len());
    let core: String = hay_chars[start..end].iter().collect();

    let collapsed = core.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut snippet = String::new();
    if start > 0 {
        snippet.push('…');
    }
    snippet.push_str(&collapsed);
    if end < hay_chars.len() {
        snippet.push('…');
    }
    Some(snippet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_snippet_is_case_insensitive() {
        let s = find_snippet("The Quarterly Revenue was strong", "revenue", 5).unwrap();
        assert!(s.to_lowercase().contains("revenue"));
    }

    #[test]
    fn find_snippet_adds_ellipses_when_truncated() {
        let long = "a ".repeat(200);
        let hay = format!("{long}needle{long}");
        let s = find_snippet(&hay, "needle", 10).unwrap();
        assert!(s.starts_with('…') && s.ends_with('…'));
        assert!(s.contains("needle"));
    }

    #[test]
    fn find_snippet_no_match_returns_none() {
        assert!(find_snippet("hello world", "absent", 5).is_none());
    }

    #[test]
    fn find_snippet_collapses_whitespace() {
        let s = find_snippet("line one\n\n   line two needle here", "needle", 40).unwrap();
        assert!(!s.contains('\n'));
        assert!(!s.contains("  "));
    }

    #[test]
    fn find_snippet_handles_multibyte_without_panicking() {
        let s = find_snippet("café résumé naïve coöperate", "résumé", 3).unwrap();
        assert!(s.to_lowercase().contains("résumé"));
    }

    #[test]
    fn search_notes_blank_query_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(search_notes(tmp.path(), "   ").is_empty());
    }

    #[test]
    fn search_notes_finds_text_in_transcript() {
        use crate::transcription::{ChannelTranscript, SessionTranscript, TranscriptSegment};
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("2026-05-29-meeting");
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("live_notes.json"), b"[]").unwrap();
        let transcript = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "mic".to_string(),
                language: None,
                segments: vec![TranscriptSegment {
                    start_seconds: 0.0,
                    end_seconds: 1.0,
                    text: "We agreed to the pelican migration plan".to_string(),
                    speaker: None,
                    language: None,
                }],
            }],
        };
        transcript.write_json(&dir.join("transcript.json")).unwrap();

        let hits = search_notes(tmp.path(), "pelican migration");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].matched_in, "transcript");
        assert!(hits[0].snippet.to_lowercase().contains("pelican migration"));
    }
}
