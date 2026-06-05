use crate::error::Result;
use crate::llm::provider::LlmProvider;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, OpenAiProvider};

pub const THRESHOLD_CHARS: usize = 20_000;

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

pub fn should_apply(agent_id: &str, transcript: &str) -> bool {
    TWO_STAGE_AGENTS.contains(&agent_id) && transcript.len() > THRESHOLD_CHARS
}

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
        return Ok(transcript.to_string());
    }
    Ok(evidence)
}

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
