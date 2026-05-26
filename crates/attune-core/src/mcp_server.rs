//! MCP server tool surface. v2 finding 067 / GET-28. Hero feature.
//!
//! Local stdio MCP server `attune-mcp` exposes Attune's library +
//! memory + tasks + transcript to coding agents (Claude Code, Cursor,
//! Codex). No incumbent can ship this — they do not have local files.
//!
//! This module owns the typed tool surface + the JSON-RPC method
//! catalogue. The actual stdio reader/writer + the dispatch into
//! `attune-core` lives in the `attune-mcp` binary that the follow-up
//! ships under `crates/attune-mcp/`. The binary depends on this
//! module so the schema stays in one place.

use serde::{Deserialize, Serialize};

/// The seven tools the MCP server exposes. Method names map 1:1 to
/// MCP `tools/call` names so external agents see a stable surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpTool {
    SearchMemory,
    FindDecision,
    ListTasks,
    CreateTask,
    GetTranscript,
    RecentMeetings,
    QuoteSegment,
}

impl McpTool {
    pub fn method_name(self) -> &'static str {
        match self {
            McpTool::SearchMemory => "search_memory",
            McpTool::FindDecision => "find_decision",
            McpTool::ListTasks => "list_tasks",
            McpTool::CreateTask => "create_task",
            McpTool::GetTranscript => "get_transcript",
            McpTool::RecentMeetings => "recent_meetings",
            McpTool::QuoteSegment => "quote_segment",
        }
    }

    pub fn from_method_name(method: &str) -> Option<McpTool> {
        Some(match method {
            "search_memory" => McpTool::SearchMemory,
            "find_decision" => McpTool::FindDecision,
            "list_tasks" => McpTool::ListTasks,
            "create_task" => McpTool::CreateTask,
            "get_transcript" => McpTool::GetTranscript,
            "recent_meetings" => McpTool::RecentMeetings,
            "quote_segment" => McpTool::QuoteSegment,
            _ => return None,
        })
    }

    pub fn description(self) -> &'static str {
        match self {
            McpTool::SearchMemory => "Search the user's Attune memory store (claims, prefs, people, observations) by free-text query.",
            McpTool::FindDecision => "Find decisions matching a free-text query across every recorded meeting.",
            McpTool::ListTasks => "List the user's Attune tasks filtered by optional status.",
            McpTool::CreateTask => "Create a new task in the user's Attune kanban with title + optional owner / due.",
            McpTool::GetTranscript => "Fetch the full transcript for a recording by its label.",
            McpTool::RecentMeetings => "List the user's most recent recordings (label, duration, has_transcript, timestamp).",
            McpTool::QuoteSegment => "Quote a specific (start, end) segment from a recording's transcript.",
        }
    }
}

/// Returns every tool the server registers. Used by the binary's
/// `tools/list` handler.
pub fn catalogue() -> &'static [McpTool] {
    &[
        McpTool::SearchMemory,
        McpTool::FindDecision,
        McpTool::ListTasks,
        McpTool::CreateTask,
        McpTool::GetTranscript,
        McpTool::RecentMeetings,
        McpTool::QuoteSegment,
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchMemoryParams {
    pub query: String,
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindDecisionParams {
    pub query: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListTasksParams {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateTaskParams {
    pub title: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetTranscriptParams {
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentMeetingsParams {
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuoteSegmentParams {
    pub label: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_names_round_trip() {
        for tool in catalogue() {
            assert_eq!(McpTool::from_method_name(tool.method_name()), Some(*tool));
        }
        assert!(McpTool::from_method_name("nope").is_none());
    }

    #[test]
    fn catalogue_is_unique() {
        let mut names: Vec<&str> = catalogue().iter().map(|t| t.method_name()).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before);
    }

    #[test]
    fn catalogue_has_seven_tools() {
        assert_eq!(catalogue().len(), 7);
    }

    #[test]
    fn descriptions_are_user_visible_prose() {
        for tool in catalogue() {
            let desc = tool.description();
            assert!(desc.ends_with('.'), "description must end with a period");
            assert!(desc.len() > 30, "description must be substantive");
        }
    }

    #[test]
    fn params_round_trip_via_json() {
        let p = CreateTaskParams {
            title: "Send deck".into(),
            owner: Some("Ege".into()),
            due: Some("Friday".into()),
            notes: None,
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: CreateTaskParams = serde_json::from_str(&s).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn search_memory_params_default_kinds_and_limit() {
        let parsed: SearchMemoryParams = serde_json::from_str(r#"{"query": "Acme"}"#).unwrap();
        assert!(parsed.kinds.is_empty());
        assert!(parsed.limit.is_none());
    }
}
