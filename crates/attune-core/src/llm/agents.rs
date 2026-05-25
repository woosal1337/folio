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
    vec![
        summarize(),
        extract_tasks(),
        extract_memories(),
        find_decisions(),
        qa(),
    ]
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

fn extract_memories() -> Agent {
    Agent {
        id: "extract-memories".to_string(),
        name: "Extract Memories".to_string(),
        description:
            "Capture lasting facts about the user, their projects, and the people they work with."
                .to_string(),
        system_prompt: EXTRACT_MEMORIES_PROMPT.to_string(),
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
Read the meeting transcript and identify every explicit action item.\n\
\n\
For each action item, call the `create_task` tool exactly once. Pass:\n\
  - title: short imperative phrase (e.g. \"Send revised contract to legal\")\n\
  - owner: the person responsible if named (e.g. \"Ege\"); omit if not stated\n\
  - due: any date or timeframe mentioned (e.g. \"Friday\", \"next sprint\", \"2026-06-01\"); omit if not stated\n\
  - notes: at most one sentence of context only if it materially helps a future reader\n\
\n\
Rules:\n\
  - Only create tasks for explicit commitments. Do not infer tasks that no one agreed to do.\n\
  - One tool call per action item. Do not bundle multiple tasks into one call.\n\
  - Do not deduplicate against existing tasks — the caller handles that.\n\
  - After all tool calls, finish with a single short sentence summarising what you created (e.g. \"Created 3 tasks.\"). \
If there are no explicit action items, do not call the tool and reply \"No explicit action items found.\"";

const EXTRACT_MEMORIES_PROMPT: &str = "You are a memory-extraction agent. \
Read the meeting transcript and capture facts that should still be true the \
next time the user opens the app. The goal is a small set of high-signal \
memories, not a wholesale rewrite of the transcript.\n\
\n\
Call `remember` once per fact you decide is worth keeping. Each call takes:\n\
  - kind: one of `claim`, `pref`, `person`, `observe`\n\
  - key: a dotted handle for `claim`/`pref`/`person` (e.g. `user.company`, \
`ui.theme`, `person.alice`). Omit for `observe`.\n\
  - content: the fact in one sentence, present tense, written so a future \
agent reading it cold understands what's true.\n\
  - evidence: a short quoted snippet from the transcript that supports it.\n\
  - confidence: 0.0-1.0; under 0.6 means \"plausible but I'm unsure\".\n\
  - tags: 1-4 short lowercase tags for browsability (e.g. `identity`, \
`engineering`, `company`).\n\
\n\
Use the kinds like this:\n\
  - `claim` for facts about the user or their projects (`user.company`, \
`user.role`, `project.attune.status`, `project.attune.next-deadline`).\n\
  - `pref` for stated preferences (`ui.theme`, `comms.style`, \
`meetings.format`).\n\
  - `person` for someone the user works with — key is the canonical handle \
(e.g. `person.alice`), content names their role + any relevant context \
(\"engineering lead on Attune, prefers async\").\n\
  - `observe` for free-form context that has no obvious key but seems \
worth keeping (\"user is preparing a launch demo for next week\").\n\
\n\
Rules:\n\
  - Skip transient facts (meeting agenda, today's blockers, \
small-talk).\n\
  - Skip facts already implied by the transcript's structure (\"this is a \
meeting\", \"the user is speaking\").\n\
  - Conflicting facts are fine — call `remember` with the new value and the \
system will supersede the old one automatically.\n\
  - If nothing is worth keeping, do not call `remember` at all. Reply with \
\"No new memories extracted.\"\n\
\n\
After all calls, finish with a one-sentence summary of what you remembered \
(e.g. \"Captured 4 memories: company, role, and two preferences.\").";

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
    fn five_defaults_in_known_order() {
        let agents = defaults();
        assert_eq!(agents.len(), 5);
        let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "summarize",
                "extract-tasks",
                "extract-memories",
                "find-decisions",
                "qa",
            ]
        );
    }

    #[test]
    fn by_id_lookup_works() {
        assert_eq!(by_id("summarize").unwrap().name, "Summarize");
        assert_eq!(by_id("extract-tasks").unwrap().name, "Extract Tasks");
        assert_eq!(by_id("extract-memories").unwrap().name, "Extract Memories");
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
