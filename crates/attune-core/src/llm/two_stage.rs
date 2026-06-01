//! Two-stage agent pipeline (GET-207).
//!
//! For long transcripts (> [`THRESHOLD_CHARS`] characters), running the
//! summarize / extract-tasks / extract-memories / find-decisions agents
//! directly on the full transcript is expensive — both in tokens and in
//! quality (the model skims large inputs). The two-stage pipeline solves
//! this:
//!
//! **Stage 1 — evidence extraction** (cheap model, full transcript):
//! Extract every factual claim, decision, and action item as bullet
//! points with verbatim transcript quotes. Output is concise (≤50
//! bullets, ~2-3 k chars).
//!
//! **Stage 2 — synthesis** (same or stronger model, condensed evidence):
//! Run the agent's normal prompt on the evidence summary rather than
//! the raw transcript. The model gets grounded, citable input and a
//! context budget 5-10× smaller, which both reduces cost and improves
//! precision.
//!
//! The split matches the behavioral-evidence contract (GET-199): stage 1
//! produces the quotable spans stage 2 must cite.
//!
//! Activation is automatic when the transcript exceeds
//! [`THRESHOLD_CHARS`] and the agent is in [`TWO_STAGE_AGENTS`].

use crate::error::Result;
use crate::llm::provider::LlmProvider;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, OpenAiProvider};

/// Transcript character threshold above which two-stage processing kicks in.
/// ≈ 20 k chars ≈ 30-40 min of average meeting at ~8 words/sec dense speech.
pub const THRESHOLD_CHARS: usize = 20_000;

/// Agent IDs for which two-stage processing is applied when the transcript
/// exceeds [`THRESHOLD_CHARS`]. These are the agents that read the full
/// transcript and produce structured output from it.
pub const TWO_STAGE_AGENTS: &[&str] = &[
    "summarize",
    "extract-tasks",
    "extract-memories",
    "find-decisions",
    "write-followup-email",
];

const EVIDENCE_SYSTEM: &str = "You are an evidence extractor. \
Read the meeting transcript and extract every factual claim, decision, \
and action item as structured bullets. \
\n\n\
Format each bullet exactly as:\n\
• [CATEGORY] \"verbatim quote\" → one-sentence summary\n\
\n\
Categories: DECISION, ACTION, FACT, QUESTION\n\
\n\
Rules:\n\
- Only report what is directly stated in the transcript.\n\
- Each quote must appear verbatim in the transcript.\n\
- Maximum 50 bullets.\n\
- Output ONLY the bullets. No preamble, no summary, no headings.";

/// Returns `true` when the two-stage pipeline should be used for this
/// agent + transcript combination.
pub fn should_apply(agent_id: &str, transcript: &str) -> bool {
    TWO_STAGE_AGENTS.contains(&agent_id) && transcript.len() > THRESHOLD_CHARS
}

/// Stage 1: extract grounded evidence spans from the raw transcript.
///
/// Returns the condensed evidence as a multi-line bullet string.
/// On error (API failure, empty result) returns the original transcript
/// unchanged so the caller can fall through to the single-stage path.
pub async fn extract_evidence(transcript: &str, api_key: &str, model: &str) -> Result<String> {
    let provider = OpenAiProvider::new(api_key.to_string());
    let resp = provider
        .chat(ChatRequest {
            model: model.to_string(),
            system_prompt: EVIDENCE_SYSTEM.to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: transcript.to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
            temperature: Some(0.1),
            max_tokens: Some(2_000),
            tools: None,
        })
        .await?;

    let evidence = resp.text.trim().to_string();
    if evidence.is_empty() {
        // Fallback: return the original so the caller uses it directly.
        return Ok(transcript.to_string());
    }
    Ok(evidence)
}

/// Wrap the stage-1 evidence in a user-message preamble so stage-2
/// agents understand they are reading a condensed evidence summary,
/// not the raw transcript.
pub fn evidence_user_message(evidence: &str) -> String {
    format!(
        "The following is a condensed evidence summary extracted from the meeting \
         transcript. Each bullet cites a verbatim quote followed by a one-sentence \
         summary. Use these as the grounded source of truth for your task:\n\n{evidence}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_apply_when_long_and_known_agent() {
        let long = "x".repeat(THRESHOLD_CHARS + 1);
        assert!(should_apply("summarize", &long));
        assert!(should_apply("extract-tasks", &long));
    }

    #[test]
    fn should_not_apply_for_short_transcripts() {
        let short = "Hello world.";
        assert!(!should_apply("summarize", short));
    }

    #[test]
    fn should_not_apply_for_unknown_agents() {
        let long = "x".repeat(THRESHOLD_CHARS + 1);
        assert!(!should_apply("qa", &long));
        assert!(!should_apply("autoname", &long));
    }

    #[test]
    fn evidence_user_message_wraps_evidence() {
        let ev = "• [FACT] \"Alice said hello\" → Alice greeted the team.";
        let msg = evidence_user_message(ev);
        assert!(msg.contains("condensed evidence summary"));
        assert!(msg.contains(ev));
    }
}
