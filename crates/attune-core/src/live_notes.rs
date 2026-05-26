//! Live notes / commands pane. v2 finding 009 / GET-33.
//!
//! Split-pane Record view: live transcript on the left, free-form
//! markdown on the right with `/action`, `/decision`, `/question`
//! slash-commands. Lens 5 demands every bullet be anchored to its
//! audio timestamp; this module owns the timestamping + parser so
//! the renderer can render and the post-meeting pipeline can pick
//! the bullets up unchanged.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    Plain,
    Action,
    Decision,
    Question,
    Highlight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveNote {
    /// Recording-relative timestamp the note was anchored to.
    pub anchor_seconds: f64,
    pub kind: NoteKind,
    pub text: String,
}

/// Parse a single line of live notes. Recognises slash commands as
/// a leading `/word` followed by whitespace. Unknown commands fall
/// through to `Plain` with the leading slash kept.
pub fn parse_line(line: &str, anchor_seconds: f64) -> Option<LiveNote> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (kind, body) = match trimmed.split_once(char::is_whitespace) {
        Some(("/action", rest)) => (NoteKind::Action, rest),
        Some(("/decision", rest)) => (NoteKind::Decision, rest),
        Some(("/question", rest)) => (NoteKind::Question, rest),
        Some(("/highlight", rest)) => (NoteKind::Highlight, rest),
        _ => (NoteKind::Plain, trimmed),
    };
    let text = body.trim();
    if text.is_empty() {
        return None;
    }
    Some(LiveNote {
        anchor_seconds,
        kind,
        text: text.to_string(),
    })
}

/// Parse a full notes buffer. The caller passes the current
/// recording-relative timestamp; every non-blank line gets anchored
/// to it. Useful for the renderer's draft-state read.
pub fn parse_buffer(buffer: &str, anchor_seconds: f64) -> Vec<LiveNote> {
    buffer
        .lines()
        .filter_map(|line| parse_line(line, anchor_seconds))
        .collect()
}

/// Render a notes buffer as Markdown that pairs with the editor's
/// briefing card. Each kind gets its own grouped section.
pub fn render_markdown(notes: &[LiveNote]) -> String {
    let mut sections: Vec<(NoteKind, Vec<&LiveNote>)> = Vec::new();
    for kind in [
        NoteKind::Action,
        NoteKind::Decision,
        NoteKind::Question,
        NoteKind::Highlight,
        NoteKind::Plain,
    ] {
        let hits: Vec<&LiveNote> = notes.iter().filter(|n| n.kind == kind).collect();
        if !hits.is_empty() {
            sections.push((kind, hits));
        }
    }
    let mut out = String::new();
    for (kind, items) in sections {
        out.push_str("## ");
        out.push_str(section_heading(kind));
        out.push_str("\n\n");
        for note in items {
            out.push_str("- `");
            out.push_str(&format_timestamp(note.anchor_seconds));
            out.push_str("` ");
            out.push_str(&note.text);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

fn section_heading(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Action => "Action items",
        NoteKind::Decision => "Decisions",
        NoteKind::Question => "Open questions",
        NoteKind::Highlight => "Highlights",
        NoteKind::Plain => "Notes",
    }
}

fn format_timestamp(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_recognises_action_command() {
        let note = parse_line("/action send the deck", 42.0).unwrap();
        assert_eq!(note.kind, NoteKind::Action);
        assert_eq!(note.text, "send the deck");
        assert!((note.anchor_seconds - 42.0).abs() < 1e-6);
    }

    #[test]
    fn parse_line_recognises_decision_command() {
        let note = parse_line("/decision ship Friday", 10.0).unwrap();
        assert_eq!(note.kind, NoteKind::Decision);
    }

    #[test]
    fn parse_line_recognises_question_command() {
        let note = parse_line("/question who owns rollback", 5.0).unwrap();
        assert_eq!(note.kind, NoteKind::Question);
    }

    #[test]
    fn parse_line_falls_through_to_plain_for_unknown_slash_command() {
        let note = parse_line("/spaghetti tonight", 1.0).unwrap();
        assert_eq!(note.kind, NoteKind::Plain);
        assert!(note.text.starts_with("/spaghetti"));
    }

    #[test]
    fn parse_line_ignores_empty_and_blank_lines() {
        assert!(parse_line("", 0.0).is_none());
        assert!(parse_line("   ", 0.0).is_none());
        assert!(parse_line("/action   ", 0.0).is_none());
    }

    #[test]
    fn parse_buffer_handles_multiline_input() {
        let buffer = "/action one\n/decision two\n\n/question three";
        let notes = parse_buffer(buffer, 0.0);
        assert_eq!(notes.len(), 3);
    }

    #[test]
    fn render_markdown_groups_by_kind() {
        let notes = vec![
            LiveNote { anchor_seconds: 5.0, kind: NoteKind::Action, text: "ship".into() },
            LiveNote { anchor_seconds: 12.0, kind: NoteKind::Decision, text: "go".into() },
            LiveNote { anchor_seconds: 20.0, kind: NoteKind::Action, text: "send deck".into() },
        ];
        let md = render_markdown(&notes);
        assert!(md.contains("## Action items"));
        assert!(md.contains("## Decisions"));
        assert!(md.matches("- `").count() == 3);
    }

    #[test]
    fn render_markdown_formats_timestamps_correctly() {
        let notes = vec![LiveNote {
            anchor_seconds: 3725.0,
            kind: NoteKind::Action,
            text: "long meeting".into(),
        }];
        let md = render_markdown(&notes);
        assert!(md.contains("1:02:05"));
    }
}
