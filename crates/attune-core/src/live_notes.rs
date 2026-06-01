//! Live notes / commands pane. v2 finding 009 / GET-33.
//!
//! Split-pane Record view: live transcript on the left, free-form
//! markdown on the right with `/action`, `/decision`, `/question`
//! slash-commands. Lens 5 demands every bullet be anchored to its
//! audio timestamp; this module owns the timestamping + parser so
//! the renderer can render and the post-meeting pipeline can pick
//! the bullets up unchanged.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../src/shared/types/")]
pub enum NoteKind {
    Plain,
    Action,
    Decision,
    Question,
    Highlight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct LiveNote {
    /// Recording-relative timestamp the note was anchored to.
    pub anchor_seconds: f64,
    pub kind: NoteKind,
    pub text: String,
}

/// A raw editor line as the live-notes UI holds it: the verbatim text
/// (slash command included) plus the timestamp the line was anchored
/// to. This is the lossless round-trip shape persisted to disk so the
/// editor can repopulate on resume; [`parse_line`] turns it into the
/// grouped [`LiveNote`] shape only at render time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct RawNoteLine {
    pub text: String,
    pub anchor_seconds: f64,
}

/// Parse raw editor lines into anchored notes, dropping blanks and bare
/// command keywords. Each line keeps its own anchor.
pub fn parse_lines(lines: &[RawNoteLine]) -> Vec<LiveNote> {
    lines
        .iter()
        .filter_map(|l| parse_line(&l.text, l.anchor_seconds))
        .collect()
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
        // A bare command keyword with no body (e.g. `/action` after
        // trimming trailing whitespace) is an empty command — map it
        // to an empty body so the is_empty() guard below drops it,
        // rather than treating "/action" as plain note text.
        None if is_command_keyword(trimmed) => (NoteKind::Plain, ""),
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

fn is_command_keyword(s: &str) -> bool {
    matches!(s, "/action" | "/decision" | "/question" | "/highlight")
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

/// One section the user sketched by typing a markdown header in their live
/// notes (GET-195): the heading plus any plain lines they wrote under it.
/// The summarize agent fleshes each of these out from the transcript,
/// growing the user's structure instead of imposing the fixed template.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteOutlineSection {
    /// Heading text with the leading `#`s stripped (e.g. "Risks").
    pub heading: String,
    /// The user's own plain lines under this heading, in order — seed
    /// material the agent should expand, not discard.
    pub user_lines: Vec<String>,
}

/// Extract the user-authored section outline from raw note lines: every
/// markdown header (`#`..`######` followed by a space) opens a section, and
/// the plain (non-command, non-header) lines beneath it are that section's
/// seed text. Slash-command lines (`/action` …) are left to the grouped
/// renderer. Returns empty when the user typed no headers — the signal to
/// fall back to the default summary template.
pub fn extract_outline(lines: &[RawNoteLine]) -> Vec<NoteOutlineSection> {
    let mut sections: Vec<NoteOutlineSection> = Vec::new();
    for line in lines {
        let trimmed = line.text.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(heading) = parse_header(trimmed) {
            sections.push(NoteOutlineSection {
                heading,
                user_lines: Vec::new(),
            });
        } else if !is_command_keyword_prefix(trimmed) {
            // A plain line seeds the current section (if the user has
            // opened one). Lines before the first header are the user's
            // freeform notes and stay with the grouped renderer.
            if let Some(section) = sections.last_mut() {
                section.user_lines.push(trimmed.to_string());
            }
        }
    }
    sections
}

/// Parse a markdown ATX header line (`## Risks`) → its text ("Risks").
/// Requires 1–6 leading `#` then whitespace, matching the editor's render.
fn parse_header(line: &str) -> Option<String> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.starts_with(char::is_whitespace) {
        return None; // "##Risks" is not a header (matches CommonMark)
    }
    let heading = rest.trim();
    (!heading.is_empty()).then(|| heading.to_string())
}

/// True for a line that is a slash command (handled by the grouped render).
fn is_command_keyword_prefix(line: &str) -> bool {
    let head = line
        .split_once(char::is_whitespace)
        .map_or(line, |(c, _)| c);
    is_command_keyword(head)
}

/// Render a user outline as a scaffold block for the summarize agent: the
/// user's headings in order, each with their seed lines as bullets. Empty
/// string when there are no sections.
pub fn render_outline_scaffold(sections: &[NoteOutlineSection]) -> String {
    let mut out = String::new();
    for section in sections {
        out.push_str("## ");
        out.push_str(&section.heading);
        out.push('\n');
        for line in &section.user_lines {
            out.push_str("- ");
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out.trim().to_string()
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
    fn parse_lines_anchors_each_line_independently_and_drops_blanks() {
        let lines = vec![
            RawNoteLine {
                text: "/action ship the build".into(),
                anchor_seconds: 5.0,
            },
            RawNoteLine {
                text: "".into(),
                anchor_seconds: 7.0,
            },
            RawNoteLine {
                text: "plain thought".into(),
                anchor_seconds: 9.0,
            },
        ];
        let notes = parse_lines(&lines);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].kind, NoteKind::Action);
        assert_eq!(notes[0].text, "ship the build");
        assert!((notes[0].anchor_seconds - 5.0).abs() < 1e-6);
        assert_eq!(notes[1].kind, NoteKind::Plain);
        assert!((notes[1].anchor_seconds - 9.0).abs() < 1e-6);
    }

    #[test]
    fn render_markdown_groups_by_kind() {
        let notes = vec![
            LiveNote {
                anchor_seconds: 5.0,
                kind: NoteKind::Action,
                text: "ship".into(),
            },
            LiveNote {
                anchor_seconds: 12.0,
                kind: NoteKind::Decision,
                text: "go".into(),
            },
            LiveNote {
                anchor_seconds: 20.0,
                kind: NoteKind::Action,
                text: "send deck".into(),
            },
        ];
        let md = render_markdown(&notes);
        assert!(md.contains("## Action items"));
        assert!(md.contains("## Decisions"));
        assert!(md.matches("- `").count() == 3);
    }

    fn raw(text: &str) -> RawNoteLine {
        RawNoteLine {
            text: text.into(),
            anchor_seconds: 0.0,
        }
    }

    #[test]
    fn extract_outline_groups_plain_lines_under_user_headers() {
        let lines = vec![
            raw("freeform before any header"), // pre-header → ignored by outline
            raw("## Risks"),
            raw("vendor lock-in"),
            raw("/action chase the SLA"), // command → not part of the outline
            raw("### Decisions"),
            raw("ship Friday"),
            raw("revisit pricing"),
        ];
        let outline = extract_outline(&lines);
        assert_eq!(outline.len(), 2);
        assert_eq!(outline[0].heading, "Risks");
        assert_eq!(outline[0].user_lines, vec!["vendor lock-in".to_string()]);
        assert_eq!(outline[1].heading, "Decisions");
        assert_eq!(
            outline[1].user_lines,
            vec!["ship Friday".to_string(), "revisit pricing".to_string()]
        );
    }

    #[test]
    fn extract_outline_empty_without_headers() {
        let lines = vec![raw("just a plain thought"), raw("/action do it")];
        assert!(extract_outline(&lines).is_empty());
    }

    #[test]
    fn parse_header_requires_space_and_bounds() {
        assert_eq!(parse_header("## Risks"), Some("Risks".to_string()));
        assert_eq!(parse_header("# A"), Some("A".to_string()));
        assert_eq!(parse_header("###### Deep"), Some("Deep".to_string()));
        assert_eq!(parse_header("##Risks"), None); // no space
        assert_eq!(parse_header("####### Too deep"), None); // 7 hashes
        assert_eq!(parse_header("#"), None); // empty heading
        assert_eq!(parse_header("not a header"), None);
    }

    #[test]
    fn render_outline_scaffold_keeps_headings_and_seed_lines() {
        let sections = vec![
            NoteOutlineSection {
                heading: "Risks".into(),
                user_lines: vec!["vendor lock-in".into()],
            },
            NoteOutlineSection {
                heading: "Next steps".into(),
                user_lines: vec![],
            },
        ];
        let md = render_outline_scaffold(&sections);
        assert!(md.contains("## Risks"));
        assert!(md.contains("- vendor lock-in"));
        assert!(md.contains("## Next steps"));
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
