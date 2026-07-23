use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::llm::provider::LlmProvider;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, OpenAiProvider};
use crate::memory::MemoryStore;
use crate::storage::scan_recordings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefBullet {
    pub text: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingBrief {
    pub bullets: Vec<BriefBullet>,

    pub sources_count: usize,

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

    let mut recordings = scan_recordings(output_dir);
    recordings.sort_by_key(|r| std::cmp::Reverse(r.created_at));
    for r in recordings.iter().take(20) {
        let dir = Path::new(&r.session_dir);
        let summary = crate::llm::AgentRunStore::list(dir)
            .ok()
            .and_then(|runs| runs.into_iter().find(|run| run.agent_id == "summarize"))
            .map(|run| run.response);
        let Some(summary) = summary else { continue };
        let lower = summary.to_lowercase();

        if !tokens.iter().any(|t| lower.contains(&t.to_lowercase())) {
            continue;
        }
        let title = r.suggested_title.as_deref().unwrap_or(&r.label);
        context.push_str(&format!("## Past meeting: {title}\n"));

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
