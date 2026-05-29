//! Enhanced-notes templates (GET-164).
//!
//! Each template is a named summarize *format*: a full system prompt
//! that produces its own section structure. The default ("generic") is
//! the four-section note from GET-147; the others tailor the headings to
//! a meeting type (standup, 1:1, sales call, interview).
//!
//! The per-note choice persists in `<session_dir>/template.txt`; the
//! `run_agent` summarize path reads it and swaps the system prompt. This
//! is intentionally lighter than the vault `MeetingTemplate` system
//! (which also drives auto-fired agents and briefing cards) — here we
//! only need "which summary shape does the user want for this note."

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::llm::agents::SUMMARIZE_PROMPT;

/// The template applied when a note has made no explicit choice.
pub const DEFAULT_ID: &str = "generic";

/// A selectable enhanced-notes format surfaced to the UI.
#[derive(Clone, Debug, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct NoteTemplate {
    /// Stable kebab-case identifier, stored in `template.txt`.
    pub id: String,
    pub name: String,
    /// One-line description for the picker.
    pub description: String,
    /// The summarize system prompt this template feeds the model. Not
    /// serialised to the UI — the picker only needs id/name/description.
    #[serde(skip)]
    #[ts(skip)]
    pub system_prompt: String,
}

/// All built-in templates, in display order. "generic" is first so it
/// reads as the default at the top of the picker.
pub fn defaults() -> Vec<NoteTemplate> {
    vec![
        NoteTemplate {
            id: DEFAULT_ID.to_string(),
            name: "General".to_string(),
            description: "Overview, Key Points, Action Items, Context.".to_string(),
            system_prompt: SUMMARIZE_PROMPT.to_string(),
        },
        NoteTemplate {
            id: "standup".to_string(),
            name: "Standup".to_string(),
            description: "What's done, in progress, and blocked.".to_string(),
            system_prompt: format!("{STANDUP_PROMPT}{COMMON_RULES}"),
        },
        NoteTemplate {
            id: "one-on-one".to_string(),
            name: "1:1".to_string(),
            description: "Discussion, feedback, growth, follow-ups.".to_string(),
            system_prompt: format!("{ONE_ON_ONE_PROMPT}{COMMON_RULES}"),
        },
        NoteTemplate {
            id: "sales-call".to_string(),
            name: "Sales call".to_string(),
            description: "Pain points, needs, objections, next steps.".to_string(),
            system_prompt: format!("{SALES_CALL_PROMPT}{COMMON_RULES}"),
        },
        NoteTemplate {
            id: "interview".to_string(),
            name: "Interview".to_string(),
            description: "Background, strengths, concerns, recommendation.".to_string(),
            system_prompt: format!("{INTERVIEW_PROMPT}{COMMON_RULES}"),
        },
    ]
}

/// Look up a template by id. Returns `None` for an unknown id so the
/// caller can fall back to the default.
pub fn by_id(id: &str) -> Option<NoteTemplate> {
    defaults().into_iter().find(|t| t.id == id)
}

/// Resolve a template id (or `None`) to its summarize system prompt,
/// always falling back to the default. This is the single entry point
/// `run_agent` uses for the summarize agent.
pub fn prompt_for(id: Option<&str>) -> String {
    id.and_then(by_id)
        .or_else(|| by_id(DEFAULT_ID))
        .map(|t| t.system_prompt)
        .unwrap_or_else(|| SUMMARIZE_PROMPT.to_string())
}

const COMMON_RULES: &str = "\n\nRules:\n\
  - Use the exact headings above, in that order. If a section has nothing, \
write \"None.\" under its heading — never drop the heading.\n\
  - Do not invent content unsupported by the transcript or the user's notes.\n\
  - Fold in the user's live notes (typed during the call) where they fit.\n\
  - Be honest about thin input: if the transcript is brief or noisy, say so \
and keep sections short.\n\
  - Follow the LANGUAGE rule at the bottom of these instructions for the \
language of your response.";

const STANDUP_PROMPT: &str = "You are a standup note-taker. Produce a clean, \
skimmable Markdown note from the transcript with EXACTLY these headings in \
this order:\n\
\n\
## Done\n\
What was completed since the last standup, as bullets, owner in (parentheses) \
when named.\n\
\n\
## In Progress\n\
What people are actively working on, as bullets with owners.\n\
\n\
## Blockers\n\
Anything blocking progress, with who is blocked and on what.\n\
\n\
## Action Items\n\
Concrete next steps as bullets, owner in (parentheses) when named.";

const ONE_ON_ONE_PROMPT: &str = "You are a 1:1 note-taker. Produce a clean, \
skimmable Markdown note from the transcript with EXACTLY these headings in \
this order:\n\
\n\
## Discussion\n\
The main topics covered, as grouped bullets.\n\
\n\
## Feedback\n\
Feedback exchanged in either direction, attributed when clear.\n\
\n\
## Growth & Goals\n\
Career, development, and goal-related points worth remembering.\n\
\n\
## Follow-ups\n\
Commitments and next steps as bullets, owner in (parentheses) when named.";

const SALES_CALL_PROMPT: &str = "You are a sales-call note-taker. Produce a \
clean, skimmable Markdown note from the transcript with EXACTLY these \
headings in this order:\n\
\n\
## Summary\n\
2-4 sentences on the prospect and how the call went.\n\
\n\
## Pain Points & Needs\n\
What the prospect is struggling with and what they're looking for.\n\
\n\
## Objections & Questions\n\
Concerns raised and open questions to address.\n\
\n\
## Next Steps\n\
Concrete follow-up commitments as bullets, owner in (parentheses) and any \
dates mentioned.";

const INTERVIEW_PROMPT: &str = "You are an interview note-taker. Produce a \
clean, skimmable Markdown note from the transcript with EXACTLY these \
headings in this order:\n\
\n\
## Background\n\
The candidate's relevant experience and context as discussed.\n\
\n\
## Strengths\n\
Positive signals, with a brief grounding for each.\n\
\n\
## Concerns\n\
Red flags or gaps, with a brief grounding for each.\n\
\n\
## Recommendation\n\
A short, honest read on fit and suggested follow-up — only what the \
transcript supports.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_unique_ids() {
        let mut ids: Vec<String> = defaults().into_iter().map(|t| t.id).collect();
        let before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate template id");
    }

    #[test]
    fn generic_is_the_default_summarize_prompt() {
        let g = by_id(DEFAULT_ID).unwrap();
        assert_eq!(g.system_prompt, SUMMARIZE_PROMPT);
    }

    #[test]
    fn prompt_for_unknown_falls_back_to_default() {
        assert_eq!(prompt_for(Some("nope")), SUMMARIZE_PROMPT);
        assert_eq!(prompt_for(None), SUMMARIZE_PROMPT);
    }

    #[test]
    fn each_template_has_a_nonempty_prompt() {
        for t in defaults() {
            assert!(!t.system_prompt.is_empty(), "{} has empty prompt", t.id);
        }
    }

    #[test]
    fn standup_prompt_declares_its_sections() {
        let p = by_id("standup").unwrap().system_prompt;
        for h in [
            "## Done",
            "## In Progress",
            "## Blockers",
            "## Action Items",
        ] {
            assert!(p.contains(h), "standup prompt missing {h}");
        }
    }
}
