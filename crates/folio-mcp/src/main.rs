use std::io::{self, BufRead, Write};

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use folio_core::mcp_server::{
    catalogue, CreateTaskParams, FindDecisionParams, GetTranscriptParams, ListTasksParams, McpTool,
    NotesByDateRangeParams, NotesByPersonParams, QuoteSegmentParams, RecentMeetingsParams,
    SearchMemoryParams,
};
use folio_core::memory::{MemoryKind, MemoryQuery, MemoryStore};
use folio_core::storage::{scan_recordings, NewTask, SettingsStore, TaskStore};
use folio_core::transcription::SessionTranscript;

const MAX_PERSON_SCAN: usize = 200;

fn settings() -> folio_core::storage::Settings {
    SettingsStore::default_location().load()
}

fn req<T: DeserializeOwned>(args: &Value) -> Result<T, String> {
    serde_json::from_value(args.clone()).map_err(|e| format!("invalid arguments: {e}"))
}

fn transcript_for(label: &str) -> Result<SessionTranscript, String> {
    let s = settings();
    let rec = scan_recordings(&s.output_dir)
        .into_iter()
        .find(|r| r.label == label)
        .ok_or_else(|| format!("no recording with label '{label}'"))?;
    SessionTranscript::read_json(&rec.session_dir.join("transcript.json"))
        .map_err(|e| e.to_string())
}

fn parse_bound(s: &str, end_of_day: bool) -> Result<chrono::DateTime<chrono::Utc>, String> {
    use chrono::{NaiveDate, TimeZone, Utc};
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    let date = NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|e| format!("could not parse date '{s}': {e}"))?;
    let naive = if end_of_day {
        date.and_hms_opt(23, 59, 59)
            .expect("23:59:59 is a valid wall-clock time")
    } else {
        date.and_hms_opt(0, 0, 0)
            .expect("00:00:00 is a valid wall-clock time")
    };
    Ok(Utc.from_utc_datetime(&naive))
}

fn handle_call(name: &str, args: &Value) -> Result<Value, String> {
    let tool = McpTool::from_method_name(name).ok_or_else(|| format!("unknown tool '{name}'"))?;
    match tool {
        McpTool::RecentMeetings => {
            let p: RecentMeetingsParams = serde_json::from_value(args.clone())
                .unwrap_or(RecentMeetingsParams { limit: None });
            let mut recs = scan_recordings(&settings().output_dir);
            recs.sort_by_key(|r| std::cmp::Reverse(r.created_at));
            recs.truncate(p.limit.unwrap_or(20));
            Ok(json!(recs))
        }
        McpTool::GetTranscript => {
            let p: GetTranscriptParams = req(args)?;
            let t = transcript_for(&p.label)?;
            Ok(json!({ "label": p.label, "dialogue": t.to_labeled_dialogue(true) }))
        }
        McpTool::QuoteSegment => {
            let p: QuoteSegmentParams = req(args)?;
            let t = transcript_for(&p.label)?;
            let mut quotes = Vec::new();
            for ch in &t.channels {
                for seg in &ch.segments {
                    if seg.end_seconds > p.start_seconds && seg.start_seconds < p.end_seconds {
                        quotes.push(json!({
                            "channel": ch.channel,
                            "start_seconds": seg.start_seconds,
                            "end_seconds": seg.end_seconds,
                            "speaker": seg.speaker,
                            "text": seg.text,
                        }));
                    }
                }
            }
            Ok(json!(quotes))
        }
        McpTool::ListTasks => {
            let p: ListTasksParams =
                serde_json::from_value(args.clone()).unwrap_or(ListTasksParams {
                    status: None,
                    limit: None,
                });
            let mut tasks = TaskStore::new(settings().tasks_path).list();
            if let Some(status) = &p.status {
                tasks.retain(|t| {
                    serde_json::to_value(t.status)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .map(|s| s.eq_ignore_ascii_case(status))
                        .unwrap_or(false)
                });
            }
            if let Some(l) = p.limit {
                tasks.truncate(l);
            }
            Ok(json!(tasks))
        }
        McpTool::CreateTask => {
            let p: CreateTaskParams = req(args)?;
            let new_task: NewTask = serde_json::from_value(json!({
                "title": p.title,
                "owner": p.owner,
                "due": p.due,
                "notes": p.notes,
            }))
            .map_err(|e| format!("could not build task: {e}"))?;
            let created = TaskStore::new(settings().tasks_path)
                .create(new_task)
                .map_err(|e| e.to_string())?;
            Ok(json!(created))
        }
        McpTool::SearchMemory => {
            let p: SearchMemoryParams = req(args)?;
            let store = MemoryStore::open(&settings().memory_dir).map_err(|e| e.to_string())?;
            let kinds: Vec<MemoryKind> = p
                .kinds
                .iter()
                .filter_map(|k| serde_json::from_value(json!(k)).ok())
                .collect();
            let query = MemoryQuery {
                query: Some(p.query.clone()),
                kinds,
                include_archived: false,
                limit: p.limit,
            };
            Ok(json!(store.list(&query).map_err(|e| e.to_string())?))
        }
        McpTool::FindDecision => {
            let p: FindDecisionParams = req(args)?;
            let store = MemoryStore::open(&settings().memory_dir).map_err(|e| e.to_string())?;
            let query = MemoryQuery {
                query: Some(p.query.clone()),
                kinds: vec![MemoryKind::Claim],
                include_archived: false,
                limit: p.limit,
            };
            Ok(json!(store.list(&query).map_err(|e| e.to_string())?))
        }
        McpTool::NotesByDateRange => {
            let p: NotesByDateRangeParams = req(args)?;
            let from = parse_bound(&p.from, false)?;
            let to = parse_bound(&p.to, true)?;
            let mut recs: Vec<_> = scan_recordings(&settings().output_dir)
                .into_iter()
                .filter(|r| r.created_at.map(|c| c >= from && c <= to).unwrap_or(false))
                .collect();
            recs.sort_by_key(|r| std::cmp::Reverse(r.created_at));
            if let Some(l) = p.limit {
                recs.truncate(l);
            }
            Ok(json!(recs))
        }
        McpTool::NotesByPerson => {
            let p: NotesByPersonParams = req(args)?;
            let needle = p.person.to_lowercase();
            let mut recs = scan_recordings(&settings().output_dir);
            recs.sort_by_key(|r| std::cmp::Reverse(r.created_at));
            let mut out = Vec::new();
            for r in recs.into_iter().take(MAX_PERSON_SCAN) {
                if !r.has_transcript {
                    continue;
                }
                let Ok(t) = SessionTranscript::read_json(&r.session_dir.join("transcript.json"))
                else {
                    continue;
                };
                let hit = t.channels.iter().any(|ch| {
                    ch.segments
                        .iter()
                        .any(|s| s.text.to_lowercase().contains(&needle))
                });
                if hit {
                    out.push(json!(r));
                    if let Some(l) = p.limit {
                        if out.len() >= l {
                            break;
                        }
                    }
                }
            }
            Ok(json!(out))
        }
    }
}

fn input_schema(tool: McpTool) -> Value {
    let obj = |props: Value, required: Vec<&str>| json!({ "type": "object", "properties": props, "required": required });
    let limit = json!({ "type": "integer", "description": "Maximum number of results." });
    match tool {
        McpTool::SearchMemory => obj(
            json!({
                "query": { "type": "string", "description": "Free-text query." },
                "kinds": { "type": "array", "items": { "type": "string", "enum": ["observe","claim","pref","person"] } },
                "limit": limit,
            }),
            vec!["query"],
        ),
        McpTool::FindDecision => obj(
            json!({ "query": { "type": "string" }, "limit": limit }),
            vec!["query"],
        ),
        McpTool::ListTasks => obj(
            json!({ "status": { "type": "string", "enum": ["todo","doing","done"] }, "limit": limit }),
            vec![],
        ),
        McpTool::CreateTask => obj(
            json!({
                "title": { "type": "string" },
                "owner": { "type": "string" },
                "due": { "type": "string" },
                "notes": { "type": "string" },
            }),
            vec!["title"],
        ),
        McpTool::GetTranscript => obj(
            json!({ "label": { "type": "string", "description": "Recording label, e.g. 2026-06-06-19-37-25." } }),
            vec!["label"],
        ),
        McpTool::RecentMeetings => obj(json!({ "limit": limit }), vec![]),
        McpTool::QuoteSegment => obj(
            json!({
                "label": { "type": "string" },
                "start_seconds": { "type": "number" },
                "end_seconds": { "type": "number" },
            }),
            vec!["label", "start_seconds", "end_seconds"],
        ),
        McpTool::NotesByPerson => obj(
            json!({ "person": { "type": "string" }, "limit": limit }),
            vec!["person"],
        ),
        McpTool::NotesByDateRange => obj(
            json!({
                "from": { "type": "string", "description": "ISO date or datetime (inclusive)." },
                "to": { "type": "string", "description": "ISO date or datetime (inclusive)." },
                "limit": limit,
            }),
            vec!["from", "to"],
        ),
    }
}

fn tools_list() -> Vec<Value> {
    catalogue()
        .iter()
        .map(|t| {
            json!({
                "name": t.method_name(),
                "description": t.description(),
                "inputSchema": input_schema(*t),
            })
        })
        .collect()
}

fn ok(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn rpc_err(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn main() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let response = match method {
            "initialize" => Some(ok(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "folio", "version": env!("CARGO_PKG_VERSION") },
                }),
            )),
            "tools/list" => Some(ok(id, json!({ "tools": tools_list() }))),
            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let result = match handle_call(name, &args) {
                    Ok(v) => json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default() }],
                    }),
                    Err(e) => json!({
                        "content": [{ "type": "text", "text": format!("error: {e}") }],
                        "isError": true,
                    }),
                };
                Some(ok(id, result))
            }
            "ping" => Some(ok(id, json!({}))),
            _ => {
                if id.is_some() {
                    Some(rpc_err(id, -32601, &format!("method not found: {method}")))
                } else {
                    None
                }
            }
        };

        if let Some(resp) = response {
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
        }
    }
    Ok(())
}
