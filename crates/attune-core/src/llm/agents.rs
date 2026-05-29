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
        write_followup_email(),
        qa(),
        autoname(),
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
        description: "Structured note: Overview, Key Points, Action Items, Context.".to_string(),
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

fn write_followup_email() -> Agent {
    Agent {
        id: "write-followup-email".to_string(),
        name: "Follow-up email".to_string(),
        description: "Draft a ready-to-send recap email with action items.".to_string(),
        system_prompt: WRITE_FOLLOWUP_EMAIL_PROMPT.to_string(),
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

fn autoname() -> Agent {
    Agent {
        id: "autoname".to_string(),
        name: "Auto-name".to_string(),
        description: "Propose a short title, 1-3 tags, and a one-line subtitle.".to_string(),
        system_prompt: AUTONAME_PROMPT.to_string(),
    }
}

const SUMMARIZE_PROMPT: &str = "You are a meeting note-taker. Given the \
transcript of one meeting — and, when present, the notes the user typed \
live during the call — produce a clean, skimmable note in Markdown with \
EXACTLY these four sections, using these stable headings in this order:\n\
\n\
## Meeting Overview\n\
2-4 sentences on what the meeting was about and how it went.\n\
\n\
## Key Points\n\
Bulleted substantive points discussed. Group related bullets under short \
**bold sub-labels** when it aids skimming.\n\
\n\
## Action Items\n\
Every action item as a bullet, with the owner in (parentheses) when named. \
Fold in the user's live `/action` notes AND any commitments in the \
transcript — if the same item appears in both, merge it into one bullet \
rather than listing it twice.\n\
\n\
## Additional Context\n\
Anything useful that doesn't fit above — decisions reached, open \
questions, names, links. Include the user's `/decision` and `/question` \
live notes here when not already covered above.\n\
\n\
Rules:\n\
  - Use the exact headings above, in that order. If a section has nothing, \
write \"None.\" under its heading — never drop the heading.\n\
  - Do not invent content unsupported by the transcript or the user's notes.\n\
  - Be honest about thin input: if the transcript is brief or noisy, say so \
in the Overview (e.g. \"The transcript was brief, so this summary is \
necessarily limited.\") and keep the other sections short.\n\
  - Follow the LANGUAGE rule at the bottom of these instructions for the \
language of your response.";

const EXTRACT_TASKS_PROMPT: &str = "You are a task-extraction agent. \
Read the meeting transcript and identify every explicit action item.\n\
\n\
For each action item, call the `create_task` tool exactly once. Pass:\n\
  - title: short imperative phrase (e.g. \"Send revised contract to legal\")\n\
  - owner: the person responsible if named (e.g. \"Ege\"); omit if not stated\n\
  - due: any date or timeframe mentioned (e.g. \"Friday\", \"next sprint\", \"2026-06-01\"); omit if not stated\n\
  - notes: at most one sentence of context only if it materially helps a future reader\n\
  - evidence: a verbatim quoted snippet from the transcript that supports the task. Required — the UI uses this to ground the task and surface an \"unverified\" badge if the snippet cannot be located in the transcript.\n\
  - confidence: 0.0-1.0. 1.0 = explicit commitment in plain words; under 0.6 = inferred or hedged. The UI tags items below 0.6 as \"unverified\".\n\
\n\
Rules:\n\
  - Only create tasks for explicit commitments. Do not infer tasks that no one agreed to do.\n\
  - One tool call per action item. Do not bundle multiple tasks into one call.\n\
  - Do not deduplicate against existing tasks — the caller handles that.\n\
  - Always include `evidence` and `confidence`. Items missing either are dropped at the guard layer (#031).\n\
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
- <decision> (rationale: <one-sentence reason if stated>) [evidence: \"<verbatim transcript snippet>\"] [confidence: 0.0-1.0]\n\
\n\
A decision is something the participants resolved to do or not do, or a \
fact they agreed to treat as settled. Speculation, brainstorming, and \
open questions do NOT count as decisions. The `evidence` snippet must \
appear verbatim in the transcript — the UI surfaces an \"unverified\" \
badge for decisions whose snippet cannot be located (#031). \
If no decisions were reached, say \"No clear decisions found.\"";

const WRITE_FOLLOWUP_EMAIL_PROMPT: &str = "You draft a follow-up email after \
a meeting. Given the transcript and the notes the user typed live during \
the call, write a concise, ready-to-send recap email.\n\
\n\
Output EXACTLY this shape and nothing else (no markdown headings, no \
preamble, no code fences):\n\
\n\
Subject: <one-line subject under 80 characters>\n\
\n\
<greeting line>\n\
\n\
<2-4 sentence recap of what was discussed and decided>\n\
\n\
Action items:\n\
- <action> (<owner>) — <due if stated>\n\
\n\
<sign-off line>\n\
\n\
Rules:\n\
  - Ground every line in the transcript or the user's notes. Do not invent \
attendees, commitments, or dates.\n\
  - If owners or attendees are not named, use a neutral greeting (\"Hi \
all,\") and omit the owner parentheses.\n\
  - Keep it tight and professional. The only bullets allowed are under \
\"Action items:\".\n\
  - If there are no action items, omit the \"Action items:\" block entirely.\n\
  - If the transcript is too thin to recap honestly, say so in one line \
instead of inventing content.\n\
  - Follow the LANGUAGE rule at the bottom of these instructions.";

const QA_PROMPT: &str = "You are an assistant answering questions about \
a meeting transcript. The user's first message contains the full \
transcript. Subsequent messages are their questions about it.\n\
\n\
Answer strictly from the transcript content. If the answer is not in \
the transcript, say \"That is not covered in this transcript.\" Do not \
guess or hallucinate.\n\
\n\
Be concise. Cite a quoted snippet from the transcript when helpful.";

const AUTONAME_PROMPT: &str = "You are a meeting auto-namer. Read the \
transcript and propose a short title, 1-3 tags, and a one-line \
subtitle the user can quickly recognise weeks later.\n\
\n\
Respond with ONLY a JSON object — no prose, no markdown fences, no \
comments — matching this exact shape:\n\
\n\
{\n\
  \"title\": \"short title under 60 characters\",\n\
  \"tags\": [\"lowercase\", \"single-word-or-hyphenated\"],\n\
  \"subtitle\": \"one-line context under 80 characters\"\n\
}\n\
\n\
Rules:\n\
  - title is concrete and specific. \"Pricing sync with Lila\" beats \"Meeting\".\n\
  - tags is 1 to 3 lowercase tokens, each <=20 chars. Prefer recurring topics \
(\"pricing\", \"onboarding\", \"hiring\") over one-off proper nouns.\n\
  - subtitle adds one sentence of context. No emojis, no hashtags.\n\
  - Follow the LANGUAGE rule at the bottom of these instructions for \
the language of `title`, `subtitle`, and any free-text tags. Tag \
tokens that are proper nouns (people, places, products) stay in their \
original form.\n\
  - When the transcript is too short or noisy to name reliably, return \
{\"title\":\"\",\"tags\":[],\"subtitle\":\"\"}.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_in_known_order() {
        let agents = defaults();
        assert_eq!(agents.len(), 7);
        let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "summarize",
                "extract-tasks",
                "extract-memories",
                "find-decisions",
                "write-followup-email",
                "qa",
                "autoname",
            ]
        );
    }

    #[test]
    fn by_id_lookup_works() {
        assert_eq!(by_id("summarize").unwrap().name, "Summarize");
        assert_eq!(by_id("extract-tasks").unwrap().name, "Extract Tasks");
        assert_eq!(by_id("extract-memories").unwrap().name, "Extract Memories");
        assert_eq!(by_id("autoname").unwrap().name, "Auto-name");
        assert_eq!(
            by_id("write-followup-email").unwrap().name,
            "Follow-up email"
        );
        assert!(by_id("nonexistent").is_none());
    }

    #[test]
    fn summarize_prompt_declares_the_four_structured_sections() {
        let p = summarize().system_prompt;
        for heading in [
            "## Meeting Overview",
            "## Key Points",
            "## Action Items",
            "## Additional Context",
        ] {
            assert!(p.contains(heading), "missing heading: {heading}");
        }
        // Honest about thin transcripts.
        assert!(p.to_lowercase().contains("brief"));
        // Folds in the user's live notes.
        assert!(p.contains("/action"));
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
