//! Pre-meeting brief generation (GET-197).
//!
//! For each upcoming external meeting on the EventKit calendar, assembles
//! a short brief from local notes and memories, then surfaces 2-3 bullets
//! in the meeting-detection HUD the instant the meeting starts.
//!
//! ## Generation
//!
//! `generate` is a blocking call (disk I/O + one LLM round-trip):
//! 1. Pull relevant memories from the memory store (search by attendee names
//!    / email-derived tokens).
//! 2. Scan recent session summaries for any mention of the attendees.
//! 3. Feed both to the LLM with a tight prompt: 2-3 bullets, each ≤15 words,
//!    covering "where we left off", "open items", and "what matters now".
//! 4. Return `MeetingBrief { bullets, sources_count }` — the HUD renders
//!    the bullets and shows a citation footer if sources_count > 0.
//!
//! ## External-only gate
//!
//! Pass a non-empty `attendees` slice. When it is empty (solo standups,
//! all-day blocks, no calendar event matched) the function returns `None`
//! immediately so the HUD stays simple.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::llm::provider::LlmProvider;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, OpenAiProvider};
use crate::memory::MemoryStore;
use crate::storage::scan_recordings;

/// One bullet in a pre-meeting brief.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefBullet {
    pub text: String,
    /// Session label this bullet was grounded in, if traceable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
}

/// A generated pre-meeting brief (GET-197).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingBrief {
    pub bullets: Vec<BriefBullet>,
    /// How many distinct notes / memories were consulted.
    pub sources_count: usize,
    /// Attendee tokens that were searched (for the citation footer).
    pub attendees_searched: Vec<String>,
}

const BRIEF_PROMPT: &str = "You are preparing a short pre-meeting brief. \
The context below is pulled from the user's local meeting notes and remembered facts. \
Using ONLY this context, write 2-3 bullet points that help the user walk into the \
meeting prepared. Each bullet must cover one of: where we left off, open items, or \
what matters now for this meeting. Keep each bullet under 15 words. \
If the context is too thin for 2 bullets, write 1. \
Return ONLY the bullets, one per line, each starting with '• '. \
Do not add headings, explanations, or any other text.";

/// Generate a pre-meeting brief from local context.
///
/// Returns `None` when `attendees` is empty (no external participants →
/// not a true external meeting) or when there is no relevant context to
/// brief on (no past notes, no memories matching the attendees).
pub async fn generate(
    attendees: &[String],
    output_dir: &Path,
    memory_store: &MemoryStore,
    api_key: &str,
    model: &str,
) -> Option<MeetingBrief> {
    if attendees.is_empty() {
        return None;
    }

    // Build a search query from attendee tokens: names (before @) + domains.
    let tokens: Vec<String> = attendees
        .iter()
        .flat_map(|a| {
            if let Some(at) = a.find('@') {
                let name = &a[..at];
                let domain = &a[at + 1..];
                vec![
                    name.replace(['.', '_', '-'], " "),
                    domain.split('.').next().unwrap_or(domain).to_string(),
                ]
            } else {
                vec![a.clone()]
            }
        })
        .filter(|t| t.len() >= 3)
        .collect();

    if tokens.is_empty() {
        return None;
    }
    let query = tokens.join(" ");

    let mut context = String::new();
    let mut sources_count = 0;

    // Recent meeting summaries mentioning the attendees.
    let mut recordings = scan_recordings(output_dir);
    recordings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    for r in recordings.iter().take(20) {
        let dir = Path::new(&r.session_dir);
        let summary = crate::llm::AgentRunStore::list(dir)
            .ok()
            .and_then(|runs| runs.into_iter().find(|run| run.agent_id == "summarize"))
            .map(|run| run.response);
        let Some(summary) = summary else { continue };
        let lower = summary.to_lowercase();
        // Include only summaries that mention at least one attendee token.
        if !tokens.iter().any(|t| lower.contains(&t.to_lowercase())) {
            continue;
        }
        let title = r.suggested_title.as_deref().unwrap_or(&r.label);
        context.push_str(&format!("## Past meeting: {title}\n"));
        // Truncate long summaries to keep context tight.
        let snippet = if summary.len() > 600 {
            &summary[..600]
        } else {
            &summary
        };
        context.push_str(snippet);
        context.push_str("\n\n");
        sources_count += 1;
        if sources_count >= 3 {
            break;
        }
    }

    // Memories matching the attendee tokens.
    if let Ok(memories) = memory_store.search(&query, None, &[], 6) {
        if !memories.is_empty() {
            context.push_str("## Remembered facts\n");
            for m in &memories {
                context.push_str(&format!("- {}\n", m.content));
            }
            sources_count += memories.len();
        }
    }

    if context.trim().is_empty() {
        return None;
    }

    // LLM call.
    let user_msg = format!(
        "Attendees: {}\n\n{}\n\nWrite the brief bullets now:",
        attendees.join(", "),
        context.trim()
    );
    let provider = OpenAiProvider::new(api_key.to_string());
    let resp = provider
        .chat(ChatRequest {
            model: model.to_string(),
            system_prompt: BRIEF_PROMPT.to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: user_msg,
                tool_calls: None,
                tool_call_id: None,
            }],
            temperature: Some(0.3),
            max_tokens: Some(200),
            tools: None,
        })
        .await
        .ok()?;

    let bullets: Vec<BriefBullet> = resp
        .text
        .lines()
        .filter_map(|l| {
            let trimmed = l.trim();
            if trimmed.starts_with('•') {
                let text = trimmed.trim_start_matches('•').trim().to_string();
                if !text.is_empty() {
                    return Some(BriefBullet {
                        text,
                        source_label: None,
                    });
                }
            }
            None
        })
        .take(3)
        .collect();

    if bullets.is_empty() {
        return None;
    }

    info!(
        bullet_count = bullets.len(),
        sources = sources_count,
        "meeting brief generated"
    );
    Some(MeetingBrief {
        bullets,
        sources_count,
        attendees_searched: tokens,
    })
}
