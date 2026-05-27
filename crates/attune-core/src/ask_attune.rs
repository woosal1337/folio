//! Cross-library Ask Attune chat — RAG pipeline + citation types.
//! v2 finding 021 / GET-27 / Hero.
//!
//! Persistent /chat surface (also Cmd-K-routable) that converses
//! across every transcript, memory, task, and agent run with
//! citations + timestamp jumps. Without it Attune is a notebook;
//! with it Attune is a brain the user talks to.
//!
//! This module owns the source-agnostic citation shape + the
//! retrieved-corpus packer that turns a hit list into the prompt
//! context the LLM sees. The actual retrieval (FTS5 + vec) and the
//! LLM call live in the runner; this is the contract both speak.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const DEFAULT_CONTEXT_TOKEN_BUDGET: usize = 6_000;
pub const APPROX_CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum CitationKind {
    Transcript,
    Memory,
    Task,
    Decision,
    AgentRun,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct Citation {
    pub id: String,
    pub kind: CitationKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_seconds: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_seconds: Option<f64>,
    /// Verbatim snippet — the LLM injects this into its system prompt
    /// and the renderer also shows it under the citation chip.
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackedContext {
    pub citations: Vec<Citation>,
    pub prompt_body: String,
}

/// Pack a candidate list into a prompt context that fits the
/// `token_budget`. Stops adding citations once the running character
/// budget (token_budget × APPROX_CHARS_PER_TOKEN) would be exceeded.
/// Preserves input order so the LLM sees the highest-ranked citations
/// first.
pub fn pack(candidates: Vec<Citation>, token_budget: usize) -> PackedContext {
    let char_budget = token_budget * APPROX_CHARS_PER_TOKEN;
    let mut total = 0usize;
    let mut chosen: Vec<Citation> = Vec::new();
    let mut body = String::new();
    for citation in candidates {
        let block = format_block(&citation);
        if total + block.len() > char_budget && !chosen.is_empty() {
            break;
        }
        total += block.len();
        body.push_str(&block);
        body.push('\n');
        chosen.push(citation);
    }
    PackedContext {
        citations: chosen,
        prompt_body: body,
    }
}

fn format_block(citation: &Citation) -> String {
    let label = match citation.kind {
        CitationKind::Transcript => "TRANSCRIPT",
        CitationKind::Memory => "MEMORY",
        CitationKind::Task => "TASK",
        CitationKind::Decision => "DECISION",
        CitationKind::AgentRun => "AGENT_RUN",
    };
    let timestamp = citation
        .start_seconds
        .map(format_timestamp)
        .unwrap_or_default();
    let session = citation.session_label.as_deref().unwrap_or("(no session)");
    format!(
        "[{label} id={id} session={session}{ts}]\n{snippet}\n",
        id = citation.id,
        snippet = citation.snippet.trim(),
        ts = if timestamp.is_empty() {
            String::new()
        } else {
            format!(" t={timestamp}")
        },
    )
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

    fn citation(id: &str, kind: CitationKind, snippet: &str) -> Citation {
        Citation {
            id: id.into(),
            kind,
            label: format!("Citation {id}"),
            session_label: Some(format!("2026-05-26-{id}")),
            start_seconds: Some(42.0),
            end_seconds: Some(48.0),
            snippet: snippet.into(),
        }
    }

    #[test]
    fn pack_emits_blocks_in_input_order() {
        let candidates = vec![
            citation("a", CitationKind::Memory, "alpha"),
            citation("b", CitationKind::Task, "beta"),
        ];
        let packed = pack(candidates, DEFAULT_CONTEXT_TOKEN_BUDGET);
        assert_eq!(packed.citations.len(), 2);
        let a_pos = packed.prompt_body.find("id=a").unwrap();
        let b_pos = packed.prompt_body.find("id=b").unwrap();
        assert!(a_pos < b_pos);
    }

    #[test]
    fn pack_drops_when_budget_exhausted() {
        let candidates = vec![
            citation("a", CitationKind::Memory, &"a".repeat(100)),
            citation("b", CitationKind::Memory, &"b".repeat(100)),
        ];
        let packed = pack(candidates, 25);
        assert_eq!(packed.citations.len(), 1);
        assert_eq!(packed.citations[0].id, "a");
    }

    #[test]
    fn pack_always_includes_at_least_one_citation_even_over_budget() {
        let candidates = vec![citation("a", CitationKind::Memory, &"x".repeat(1000))];
        let packed = pack(candidates, 1);
        assert_eq!(packed.citations.len(), 1);
    }

    #[test]
    fn pack_keeps_kinds_correctly_labelled() {
        let candidates = vec![
            citation("t", CitationKind::Transcript, "spoken"),
            citation("m", CitationKind::Memory, "remembered"),
            citation("k", CitationKind::Task, "task"),
            citation("d", CitationKind::Decision, "decided"),
            citation("a", CitationKind::AgentRun, "run"),
        ];
        let packed = pack(candidates, DEFAULT_CONTEXT_TOKEN_BUDGET);
        for tag in ["TRANSCRIPT", "MEMORY", "TASK", "DECISION", "AGENT_RUN"] {
            assert!(packed.prompt_body.contains(tag), "missing {tag}");
        }
    }

    #[test]
    fn pack_renders_timestamps_when_present() {
        let candidates = vec![citation("a", CitationKind::Transcript, "with timestamp")];
        let packed = pack(candidates, DEFAULT_CONTEXT_TOKEN_BUDGET);
        assert!(packed.prompt_body.contains("t=0:42"));
    }

    #[test]
    fn pack_omits_timestamp_marker_when_missing() {
        let mut c = citation("a", CitationKind::Memory, "no timing");
        c.start_seconds = None;
        let packed = pack(vec![c], DEFAULT_CONTEXT_TOKEN_BUDGET);
        assert!(!packed.prompt_body.contains("t="));
    }
}
