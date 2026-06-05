use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;

use crate::llm::types::{ToolCall, ToolDef};
use crate::memory::{MemoryKind, MemoryStore, NewMemory};
use crate::storage::{NewTask, TaskStore};
use crate::transcription::{locate, SessionTranscript};

pub const MENTIONED_ONCE_TAG: &str = "mentioned-once";

const TASK_TOOL_AGENTS: &[&str] = &["extract-tasks"];

const MEMORY_WRITE_TOOL_AGENTS: &[&str] = &["extract-memories"];

fn memory_search_for_all() -> bool {
    true
}

pub fn tools_for_agent(agent_id: &str) -> Option<Vec<ToolDef>> {
    let mut tools = Vec::new();
    if memory_search_for_all() {
        tools.push(search_memory_tool_def());
    }
    if TASK_TOOL_AGENTS.contains(&agent_id) {
        tools.push(create_task_tool_def());
    }
    if MEMORY_WRITE_TOOL_AGENTS.contains(&agent_id) {
        tools.push(remember_tool_def());
    }
    if tools.is_empty() {
        None
    } else {
        Some(tools)
    }
}

fn create_task_tool_def() -> ToolDef {
    ToolDef {
        name: "create_task".to_string(),
        description: "Create a new to-do task in the user's task list. \
            Call once per distinct action item found in the meeting transcript."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Short imperative phrase describing the task." },
                "owner": { "type": "string", "description": "Person or team responsible. Omit if not stated." },
                "due":   { "type": "string", "description": "Date or timeframe. Omit if not stated." },
                "notes": { "type": "string", "description": "Optional one-sentence context." }
            },
            "required": ["title"],
            "additionalProperties": false
        }),
    }
}

fn remember_tool_def() -> ToolDef {
    ToolDef {
        name: "remember".to_string(),
        description: "Capture a lasting fact about the user, their projects, or the people they work with. \
Call once per fact. Use `claim` for facts about the user, `pref` for preferences, `person` for someone they collaborate with, \
`observe` for free-form context with no obvious key. Conflicting facts on the same key supersede automatically; do not try to deduplicate."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["claim", "pref", "person", "observe"],
                    "description": "claim / pref / person / observe — see the agent prompt for guidance."
                },
                "key": {
                    "type": "string",
                    "description": "Dotted handle (e.g. `user.company`, `ui.theme`, `person.alice`). Required for claim/pref/person; omit for observe."
                },
                "content": {
                    "type": "string",
                    "description": "The fact in one sentence, present tense."
                },
                "evidence": {
                    "type": "string",
                    "description": "Short quoted snippet from the transcript that supports the fact."
                },
                "confidence": {
                    "type": "number",
                    "description": "0.0-1.0; under 0.6 means \"plausible but unsure\"."
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "1-4 short lowercase tags."
                }
            },
            "required": ["kind", "content"],
            "additionalProperties": false
        }),
    }
}

fn search_memory_tool_def() -> ToolDef {
    ToolDef {
        name: "search_memory".to_string(),
        description: "Look up what the system already knows about the user. \
CALL THIS WHENEVER: you need to verify a name/role/company, check whether a topic has come up before, \
or avoid re-asking something the user has stated previously. Returns up to `limit` currently-valid memories ranked by relevance."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Free-text search. Use the key (e.g. `user.company`) or the topic (e.g. `quarterly planning`)."
                },
                "kinds": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["claim", "pref", "person", "observe"] },
                    "description": "Optional. Restrict to these kinds. Omit to search all."
                },
                "limit": {
                    "type": "integer",
                    "description": "Optional. Max rows to return (default 5)."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }),
    }
}

#[derive(serde::Serialize, Default)]
pub struct ToolResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct CreateTaskArgs {
    title: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    due: Option<String>,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Deserialize)]
struct RememberArgs {
    kind: String,
    #[serde(default)]
    key: Option<String>,
    content: String,
    #[serde(default)]
    evidence: Option<String>,
    #[serde(default = "default_remember_confidence")]
    confidence: f32,
    #[serde(default)]
    tags: Vec<String>,
}

fn default_remember_confidence() -> f32 {
    0.8
}

#[derive(Deserialize)]
struct SearchMemoryArgs {
    query: String,
    #[serde(default)]
    kinds: Vec<String>,
    #[serde(default)]
    limit: Option<usize>,
}

pub fn dispatch_tool_call(
    call: &ToolCall,
    tasks_path: &Path,
    memory_store: Arc<MemoryStore>,
    session_dir: &str,
    session_label: Option<&str>,
    transcript: Option<&SessionTranscript>,
) -> ToolResult {
    match call.name.as_str() {
        "create_task" => dispatch_create_task(call, tasks_path, session_dir, session_label),
        "remember" => dispatch_remember(call, memory_store, session_dir, session_label, transcript),
        "search_memory" => dispatch_search_memory(call, memory_store),
        other => ToolResult {
            success: false,
            error: Some(format!("unknown tool: {other}")),
            ..ToolResult::default()
        },
    }
}

fn dispatch_create_task(
    call: &ToolCall,
    tasks_path: &Path,
    session_dir: &str,
    session_label: Option<&str>,
) -> ToolResult {
    match serde_json::from_str::<CreateTaskArgs>(&call.arguments) {
        Ok(args) => {
            let store = TaskStore::new(tasks_path.to_path_buf());
            let new_task = NewTask {
                title: args.title,
                status: None,
                owner: args.owner.filter(|s| !s.trim().is_empty()),
                due: args.due.filter(|s| !s.trim().is_empty()),
                notes: args.notes.filter(|s| !s.trim().is_empty()),
                source_session_dir: Some(session_dir.to_string()),
                source_session_label: session_label.map(|s| s.to_string()),
                agent_origin: true,
            };
            match store.create(new_task) {
                Ok(task) => ToolResult {
                    success: true,
                    id: Some(task.id),
                    ..ToolResult::default()
                },
                Err(e) => ToolResult {
                    success: false,
                    error: Some(e.to_string()),
                    ..ToolResult::default()
                },
            }
        }
        Err(e) => ToolResult {
            success: false,
            error: Some(format!("could not parse arguments: {e}")),
            ..ToolResult::default()
        },
    }
}

fn dispatch_remember(
    call: &ToolCall,
    store: Arc<MemoryStore>,
    session_dir: &str,
    session_label: Option<&str>,
    transcript: Option<&SessionTranscript>,
) -> ToolResult {
    let args = match serde_json::from_str::<RememberArgs>(&call.arguments) {
        Ok(a) => a,
        Err(e) => {
            return ToolResult {
                success: false,
                error: Some(format!("could not parse arguments: {e}")),
                ..ToolResult::default()
            }
        }
    };
    let kind = match MemoryKind::parse(&args.kind) {
        Some(k) => k,
        None => {
            return ToolResult {
                success: false,
                error: Some(format!("unknown kind: {}", args.kind)),
                ..ToolResult::default()
            }
        }
    };
    let evidence = args.evidence.filter(|s| !s.trim().is_empty());

    let mut tags = args.tags;
    if let (Some(t), Some(ev)) = (transcript, evidence.as_deref()) {
        if locate::support_count(t, ev) == 1 && !tags.iter().any(|x| x == MENTIONED_ONCE_TAG) {
            tags.push(MENTIONED_ONCE_TAG.to_string());
        }
    }

    let new_memory = NewMemory {
        kind,
        key: args.key.filter(|s| !s.trim().is_empty()),
        content: args.content,
        evidence,
        confidence: args.confidence,
        tags,
        source_session_dir: Some(session_dir.to_string()),
        source_session_label: session_label.map(|s| s.to_string()),
    };
    match store.create(new_memory) {
        Ok(outcome) => {
            let memory = outcome.into_memory();
            ToolResult {
                success: true,
                id: Some(memory.id),
                ..ToolResult::default()
            }
        }
        Err(e) => ToolResult {
            success: false,
            error: Some(e.to_string()),
            ..ToolResult::default()
        },
    }
}

fn dispatch_search_memory(call: &ToolCall, store: Arc<MemoryStore>) -> ToolResult {
    let args = match serde_json::from_str::<SearchMemoryArgs>(&call.arguments) {
        Ok(a) => a,
        Err(e) => {
            return ToolResult {
                success: false,
                error: Some(format!("could not parse arguments: {e}")),
                ..ToolResult::default()
            }
        }
    };
    let kinds: Vec<MemoryKind> = args
        .kinds
        .iter()
        .filter_map(|s| MemoryKind::parse(s))
        .collect();
    let limit = args.limit.unwrap_or(5);

    match store.search(&args.query, None, &kinds, limit) {
        Ok(memories) => {
            let projected: Vec<serde_json::Value> = memories
                .into_iter()
                .map(|m| {
                    json!({
                        "id": m.id,
                        "kind": m.kind.as_str(),
                        "key": m.key,
                        "content": m.content,
                        "valid_from": m.valid_from.to_rfc3339(),
                    })
                })
                .collect();
            ToolResult {
                success: true,
                data: Some(json!({ "results": projected })),
                ..ToolResult::default()
            }
        }
        Err(e) => ToolResult {
            success: false,
            error: Some(e.to_string()),
            ..ToolResult::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::NewMemory;
    use tempfile::TempDir;

    fn arc_store(dir: &std::path::Path) -> Arc<MemoryStore> {
        Arc::new(MemoryStore::open(dir).unwrap())
    }

    #[test]
    fn tools_for_agent_attaches_correct_set() {
        let summarize = tools_for_agent("summarize").unwrap();
        assert!(summarize.iter().any(|t| t.name == "search_memory"));
        assert!(!summarize.iter().any(|t| t.name == "create_task"));
        assert!(!summarize.iter().any(|t| t.name == "remember"));

        let tasks = tools_for_agent("extract-tasks").unwrap();
        assert!(tasks.iter().any(|t| t.name == "create_task"));
        assert!(tasks.iter().any(|t| t.name == "search_memory"));

        let memories = tools_for_agent("extract-memories").unwrap();
        assert!(memories.iter().any(|t| t.name == "remember"));
        assert!(memories.iter().any(|t| t.name == "search_memory"));
    }

    #[test]
    fn dispatch_create_task_writes_to_store() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let store = arc_store(&dir.path().join("memory"));
        let call = ToolCall {
            id: "call_1".into(),
            name: "create_task".into(),
            arguments: r#"{"title":"Send recap","owner":"Ege"}"#.into(),
        };
        let r = dispatch_tool_call(
            &call,
            &tasks_path,
            store,
            "/sessions/abc",
            Some("2026-05-25-team"),
            None,
        );
        assert!(r.success, "got error: {:?}", r.error);
        assert!(r.id.is_some());
    }

    #[test]
    fn dispatch_remember_creates_memory() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let store = arc_store(&dir.path().join("memory"));
        let call = ToolCall {
            id: "call_m".into(),
            name: "remember".into(),
            arguments: r#"{"kind":"claim","key":"user.company","content":"Folio","confidence":0.9,"tags":["company"]}"#.into(),
        };
        let r = dispatch_tool_call(
            &call,
            &tasks_path,
            store.clone(),
            "/sessions/abc",
            Some("2026-05-25"),
            None,
        );
        assert!(r.success, "got error: {:?}", r.error);
        let memories = store.list(&Default::default()).unwrap();
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].content, "Folio");
        assert!(memories[0].source_session_dir.is_some());
    }

    #[test]
    fn dispatch_remember_tags_single_utterance_claims() {
        use crate::transcription::{ChannelTranscript, SessionTranscript, TranscriptSegment};
        let seg = |t: &str, s: f64| TranscriptSegment {
            start_seconds: s,
            end_seconds: s + 2.0,
            text: t.to_string(),
            speaker: None,
            language: None,
        };
        let transcript = SessionTranscript {
            channels: vec![ChannelTranscript {
                channel: "system".into(),
                language: None,
                segments: vec![
                    seg("Anyway I once skydived over Dubai years ago.", 0.0),
                    seg("Let's get back to the launch timeline.", 30.0),
                ],
            }],
        };
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let store = arc_store(&dir.path().join("memory"));
        let call = ToolCall {
            id: "call_o".into(),
            name: "remember".into(),
            arguments: r#"{"kind":"observe","content":"User skydived over Dubai","evidence":"I once skydived over Dubai years ago"}"#.into(),
        };
        let r = dispatch_tool_call(
            &call,
            &tasks_path,
            store.clone(),
            "/sessions/abc",
            None,
            Some(&transcript),
        );
        assert!(r.success, "got error: {:?}", r.error);
        let memories = store.list(&Default::default()).unwrap();
        assert!(
            memories[0].tags.iter().any(|t| t == MENTIONED_ONCE_TAG),
            "expected the single-utterance aside to be tagged: {:?}",
            memories[0].tags
        );
    }

    #[test]
    fn dispatch_search_memory_returns_hits() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let store = arc_store(&dir.path().join("memory"));
        store
            .create(NewMemory {
                kind: MemoryKind::Claim,
                key: Some("user.company".into()),
                content: "Folio".into(),
                ..NewMemory::default()
            })
            .unwrap();
        let call = ToolCall {
            id: "call_s".into(),
            name: "search_memory".into(),
            arguments: r#"{"query":"company"}"#.into(),
        };
        let r = dispatch_tool_call(&call, &tasks_path, store, "/sessions/abc", None, None);
        assert!(r.success, "got error: {:?}", r.error);
        let data = r.data.expect("results");
        assert!(data["results"].as_array().is_some_and(|a| !a.is_empty()));
    }
}
