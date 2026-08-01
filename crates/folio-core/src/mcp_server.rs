use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpTool {
    SearchMemory,
    FindDecision,
    ListTasks,
    CreateTask,
    GetTranscript,
    RecentMeetings,
    QuoteSegment,

    NotesByPerson,

    NotesByDateRange,
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

            McpTool::NotesByPerson => "notes_by_person",
            McpTool::NotesByDateRange => "notes_by_date_range",
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
            "notes_by_person" => McpTool::NotesByPerson,
            "notes_by_date_range" => McpTool::NotesByDateRange,
            _ => return None,
        })
    }

    pub fn description(self) -> &'static str {
        match self {
            McpTool::SearchMemory => "Search the user's Folio memory store (claims, prefs, people, observations) by free-text query.",
            McpTool::FindDecision => "Find decisions matching a free-text query across every recorded meeting.",
            McpTool::ListTasks => "List the user's Folio tasks filtered by optional status.",
            McpTool::CreateTask => "Create a new task in the user's Folio kanban with title + optional owner / due.",
            McpTool::GetTranscript => "Fetch the full transcript for a recording by its label.",
            McpTool::RecentMeetings => "List the user's most recent recordings (label, duration, has_transcript, timestamp).",
            McpTool::QuoteSegment => "Quote a specific (start, end) segment from a recording's transcript.",
            McpTool::NotesByPerson => "Return notes that mention a specific person (by name or email address), newest first.",
            McpTool::NotesByDateRange => "Return notes captured between two ISO-8601 dates (inclusive), newest first.",
        }
    }
}

pub fn catalogue() -> &'static [McpTool] {
    &[
        McpTool::SearchMemory,
        McpTool::FindDecision,
        McpTool::ListTasks,
        McpTool::CreateTask,
        McpTool::GetTranscript,
        McpTool::RecentMeetings,
        McpTool::QuoteSegment,
        McpTool::NotesByPerson,
        McpTool::NotesByDateRange,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotesByPersonParams {
    pub person: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotesByDateRangeParams {
    pub from: String,

    pub to: String,
    #[serde(default)]
    pub limit: Option<usize>,
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
    fn catalogue_has_nine_tools() {
        assert_eq!(catalogue().len(), 9);
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
