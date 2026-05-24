//! Default agents shipped with Attune.
//!
//! Phase 1.5 MVP ships four read-only baked-in agents so the user can
//! actually use the AI feature right after configuring an API key. The
//! v0.1 work in the vault plan (phases 3 + 6) adds on-disk TOML
//! definitions so the user can edit prompts and create custom agents;
//! this module is the source of truth for the *default* prompts that
//! the editor will "Restore default" back to.
//!
//! All four agents are one-shot (single turn over the whole
//! transcript). Multi-turn chat is phase 5/7.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A read-only agent definition surfaced to the UI.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Agent {
    /// Stable kebab-case identifier. Used as the filename for
    /// persisted runs and as the route key in the UI.
    pub id: String,
    pub name: String,
    /// One-sentence description shown under the agent's button.
    pub description: String,
    /// The system prompt the model sees. Kept short — long prompts eat
    /// the context budget and rarely help on transcripts of meetings.
    pub system_prompt: String,
}

/// All default agents, in display order.
pub fn defaults() -> Vec<Agent> {
    vec![summarize(), extract_tasks(), find_decisions(), qa()]
}

/// Look up a default agent by id. Returns `None` if `id` does not
/// match any default (custom agents arrive in phase 3).
pub fn by_id(id: &str) -> Option<Agent> {
    defaults().into_iter().find(|a| a.id == id)
}

fn summarize() -> Agent {
    Agent {
        id: "summarize".to_string(),
        name: "Summarize".to_string(),
        description: "One-paragraph summary plus a bulleted highlight list.".to_string(),
        system_prompt: SUMMARIZE_PROMPT.to_string(),
    }
}

fn extract_tasks() -> Agent {
    Agent {
        id: "extract-tasks".to_string(),
        name: "Extract Tasks".to_string(),
        description: "Pull explicit action items out of the meeting.".to_string(),
        system_prompt: EXTRACT_TASKS_PROMPT.to_string(),
    }
}

fn find_decisions() -> Agent {
    Agent {
        id: "find-decisions".to_string(),
        name: "Find Decisions".to_string(),
        description: "List every decision the participants agreed on.".to_string(),
        system_prompt: FIND_DECISIONS_PROMPT.to_string(),
    }
}

fn qa() -> Agent {
    Agent {
        id: "qa".to_string(),
        name: "Q&A".to_string(),
        description: "Open-ended question answering over the transcript.".to_string(),
        system_prompt: QA_PROMPT.to_string(),
    }
}

const SUMMARIZE_PROMPT: &str = "You are a meeting summariser. \
Given the transcript of one meeting, produce:\n\
\n\
1. A one-paragraph summary in the language of the transcript.\n\
2. A bulleted list of 3-7 highlights: decisions made, action items, open questions.\n\
\n\
Do not invent content not in the transcript. \
If the transcript is too short or noisy to summarise, say so.";

const EXTRACT_TASKS_PROMPT: &str = "You are a task-extraction agent. \
Read the meeting transcript and list every explicit action item.\n\
\n\
Format each task as a markdown checkbox:\n\
- [ ] <task description> — <owner if known> (due <date if mentioned>)\n\
\n\
Action items must be explicit commitments. Do not infer tasks that no one \
actually agreed to do. If no action items exist, say \"No explicit action \
items found.\"";

const FIND_DECISIONS_PROMPT: &str = "You are a decision-tracker. \
Read the meeting transcript and list every decision the participants \
agreed on.\n\
\n\
Format each decision as:\n\
- <decision> (rationale: <one-sentence reason if stated>)\n\
\n\
A decision is something the participants resolved to do or not do, or a \
fact they agreed to treat as settled. Speculation, brainstorming, and \
open questions do NOT count as decisions. \
If no decisions were reached, say \"No clear decisions found.\"";

const QA_PROMPT: &str = "You are an assistant answering questions about \
a meeting transcript. The user's first message contains the full \
transcript. Subsequent messages are their questions about it.\n\
\n\
Answer strictly from the transcript content. If the answer is not in \
the transcript, say \"That is not covered in this transcript.\" Do not \
guess or hallucinate.\n\
\n\
Be concise. Cite a quoted snippet from the transcript when helpful.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_defaults_in_known_order() {
        let agents = defaults();
        assert_eq!(agents.len(), 4);
        let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["summarize", "extract-tasks", "find-decisions", "qa"]
        );
    }

    #[test]
    fn by_id_lookup_works() {
        assert_eq!(by_id("summarize").unwrap().name, "Summarize");
        assert_eq!(by_id("extract-tasks").unwrap().name, "Extract Tasks");
        assert!(by_id("nonexistent").is_none());
    }

    #[test]
    fn all_default_prompts_are_nonempty() {
        for agent in defaults() {
            assert!(
                !agent.system_prompt.is_empty(),
                "{} has empty prompt",
                agent.id
            );
            assert!(!agent.name.is_empty());
            assert!(!agent.description.is_empty());
        }
    }
}
