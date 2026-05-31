//! Export a note as a clean, self-contained Markdown file (GET-166).
//!
//! Local-first sharing: assemble the note's title, date, enhanced-notes
//! summary, the user's live notes, and the transcript into one Markdown
//! document and write it next to the recording. The caller then reveals
//! it or hands it to the OS share sheet. Nothing leaves the machine, so
//! it's safe under `privacy_mode` with no cloud egress.

use std::path::{Path, PathBuf};

use crate::error::{AttuneError, Result};
use crate::llm::agent_run::AgentRunStore;
use crate::storage::atomic_write::atomic_write;
use crate::storage::session::scan_recordings;
use crate::transcription::SessionTranscript;

/// Build the Markdown document from already-loaded pieces. Pure so the
/// formatting is unit-testable. Sections with no content are omitted.
pub fn render_markdown(
    title: &str,
    date_line: Option<&str>,
    summary: Option<&str>,
    live_notes: Option<&str>,
    transcript: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(title.trim());
    out.push('\n');
    if let Some(date) = date_line {
        if !date.trim().is_empty() {
            out.push('\n');
            out.push('*');
            out.push_str(date.trim());
            out.push('*');
            out.push('\n');
        }
    }
    if let Some(s) = summary.map(str::trim).filter(|s| !s.is_empty()) {
        out.push_str("\n## Enhanced notes\n\n");
        out.push_str(s);
        out.push('\n');
    }
    if let Some(n) = live_notes.map(str::trim).filter(|s| !s.is_empty()) {
        out.push_str("\n## My notes\n\n");
        out.push_str(n);
        out.push('\n');
    }
    if let Some(t) = transcript.map(str::trim).filter(|s| !s.is_empty()) {
        out.push_str("\n## Transcript\n\n");
        out.push_str(t);
        out.push('\n');
    }
    out
}

/// Render a transcript into readable Markdown: one labelled block per
/// channel ("You" for mic, "Others" for system), segments joined into a
/// paragraph. `None` when the transcript is missing or empty.
fn transcript_markdown(session_dir: &Path) -> Option<String> {
    let transcript = SessionTranscript::read_json(&session_dir.join("transcript.json")).ok()?;
    let mut blocks: Vec<String> = Vec::new();
    for channel in &transcript.channels {
        let text = channel
            .segments
            .iter()
            .map(|s| s.text.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            continue;
        }
        let speaker = match channel.channel.as_str() {
            "mic" => "You",
            "system" => "Others",
            other => other,
        };
        blocks.push(format!("**{speaker}:** {text}"));
    }
    if blocks.is_empty() {
        None
    } else {
        Some(blocks.join("\n\n"))
    }
}

/// Read + render the user's live notes for a note, if any.
fn live_notes_markdown(session_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(session_dir.join("live_notes.json")).ok()?;
    let lines: Vec<crate::live_notes::RawNoteLine> = serde_json::from_slice(&bytes).ok()?;
    let notes = crate::live_notes::parse_lines(&lines);
    if notes.is_empty() {
        return None;
    }
    Some(crate::live_notes::render_markdown(&notes))
}

/// Assemble + write the note's Markdown export to
/// `<session_dir>/<label>.md` and return its path. Reads the title (user
/// title → autoname → label), the enhanced-notes summary, live notes,
/// and the transcript from disk.
pub fn write_markdown(output_dir: &Path, session_dir: &Path) -> Result<PathBuf> {
    let summary_meta = scan_recordings(output_dir)
        .into_iter()
        .find(|r| r.session_dir == session_dir);
    let (title, date_line, label) = match &summary_meta {
        Some(rec) => {
            let title = rec
                .title
                .clone()
                .or_else(|| rec.suggested_title.clone())
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| rec.label.clone());
            let date_line = rec.created_at.map(|d| d.format("%B %-d, %Y").to_string());
            (title, date_line, rec.label.clone())
        }
        None => {
            let label = session_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("note")
                .to_string();
            (label.clone(), None, label)
        }
    };

    let summary_text = AgentRunStore::list(session_dir)
        .ok()
        .and_then(|runs| runs.into_iter().find(|r| r.agent_id == "summarize"))
        .map(|r| r.response);
    let live = live_notes_markdown(session_dir);
    let transcript = transcript_markdown(session_dir);

    let markdown = render_markdown(
        &title,
        date_line.as_deref(),
        summary_text.as_deref(),
        live.as_deref(),
        transcript.as_deref(),
    );

    let path = session_dir.join(format!("{label}.md"));
    atomic_write(&path, markdown.as_bytes())
        .map_err(|e| AttuneError::Storage(format!("could not write note export: {e}")))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_present_sections_only() {
        let md = render_markdown(
            "Budget sync",
            Some("May 28, 2026"),
            Some("## Overview\nWe talked money."),
            None,
            Some("**You:** hello"),
        );
        assert!(md.starts_with("# Budget sync"));
        assert!(md.contains("*May 28, 2026*"));
        assert!(md.contains("## Enhanced notes"));
        assert!(md.contains("## Transcript"));
        assert!(!md.contains("## My notes"));
    }

    #[test]
    fn render_title_only_is_valid() {
        let md = render_markdown("Title", None, None, None, None);
        assert_eq!(md.trim(), "# Title");
    }

    #[test]
    fn write_markdown_produces_a_file_with_the_title() {
        use crate::transcription::{ChannelTranscript, SessionTranscript, TranscriptSegment};
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("2026-05-28-budget");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("live_notes.json"), b"[]").unwrap();
        std::fs::write(dir.join("title.txt"), b"Budget sync").unwrap();
        let transcript = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "mic".to_string(),
                language: None,
                segments: vec![TranscriptSegment {
                    start_seconds: 0.0,
                    end_seconds: 1.0,
                    text: "we approved the plan".to_string(),
                    speaker: None,
                    language: None,
                }],
            }],
        };
        transcript.write_json(&dir.join("transcript.json")).unwrap();

        let path = write_markdown(tmp.path(), &dir).unwrap();
        assert_eq!(path, dir.join("2026-05-28-budget.md"));
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("# Budget sync"));
        assert!(body.contains("**You:** we approved the plan"));
    }
}
