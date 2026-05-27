//! Structured run-card telemetry. v2 finding 028 / GET-54.
//!
//! Today each agent run persists as a small `<agent_id>.json` with the
//! response text + token counts. That's enough for the inbox preview
//! but not enough to debug "why did the agent extract that task?"
//!
//! This module ships a sidecar telemetry file —
//! `<session_dir>/agent_runs/<agent_id>.telemetry.json` — that holds
//! the observability data the run-card UI needs:
//!
//!   * model + provider + total latency.
//!   * Prompt / completion / total tokens.
//!   * Cost estimate in USD (provider × model × tokens).
//!   * Tool calls with arguments, result, and per-call latency.
//!   * Evidence spans the agent claimed to ground its output in.
//!   * The previous run's response so the UI can diff (edit-and-retry).
//!
//! Additive change: existing `AgentRun` records keep working. The
//! telemetry sidecar is optional and read on demand by the run-card
//! component. Older runs without a telemetry file render the legacy
//! preview unchanged.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AttuneError, Result};
use crate::llm::ProviderId;

const TELEMETRY_SUFFIX: &str = ".telemetry.json";

/// One tool the agent invoked. The UI expands these inline under the
/// run-card so the user can see exactly what `create_task` etc. was
/// called with.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ToolCallTrace {
    pub tool: String,
    /// JSON-encoded arguments the model passed. We keep the original
    /// string (not a serde_json::Value) so the UI can render it
    /// verbatim without re-pretty-printing.
    pub arguments_json: String,
    /// Free-form result string the tool produced (the same value the
    /// model receives back in the loop).
    pub result: String,
    pub latency_ms: u64,
}

/// Verbatim transcript span the agent grounded a claim in. Pair with
/// the locate_span helper (#038 / GET-41) to render a clickable jump.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct EvidenceSpan {
    /// Human-readable label of what this evidence is for ("task: ship
    /// the redesign", "memory: user.company"). Optional for free-form
    /// agents.
    pub label: Option<String>,
    pub span_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct AgentRunTelemetry {
    pub agent_id: String,
    pub provider: ProviderId,
    pub model: String,
    pub latency_ms: u64,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    /// `prompt_tokens + completion_tokens` when both are known; one of
    /// them when only one is reported; None when the provider gave us
    /// no usage block.
    pub total_tokens: Option<u32>,
    /// Best-effort cost in USD computed from the model's published
    /// per-million-token price. None when the model isn't in our table.
    pub estimated_cost_usd: Option<f32>,
    pub tool_calls: Vec<ToolCallTrace>,
    pub evidence_spans: Vec<EvidenceSpan>,
    /// The response text of the run immediately before this one, kept
    /// so the UI can render a diff for the "edit and retry" flow.
    /// None when this is the first run for the agent on this recording.
    pub previous_response: Option<String>,
    pub finished_at: DateTime<Utc>,
}

/// Resolve the on-disk path for an agent's telemetry sidecar.
pub fn telemetry_path(session_dir: &Path, agent_id: &str) -> PathBuf {
    session_dir
        .join("agent_runs")
        .join(format!("{agent_id}{TELEMETRY_SUFFIX}"))
}

/// Persist the telemetry sidecar next to the existing
/// `<agent_id>.json`. Atomic-ish (temp + rename). Creates parent dirs.
pub fn save(session_dir: &Path, telemetry: &AgentRunTelemetry) -> Result<PathBuf> {
    let final_path = telemetry_path(session_dir, &telemetry.agent_id);
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AttuneError::Storage(format!(
                "could not create agent_runs dir at {}: {e}",
                parent.display()
            ))
        })?;
    }
    let tmp_path = final_path.with_extension("telemetry.json.tmp");
    let json = serde_json::to_string_pretty(telemetry)
        .map_err(|e| AttuneError::Storage(format!("could not serialise telemetry: {e}")))?;
    std::fs::write(&tmp_path, json).map_err(|e| {
        AttuneError::Storage(format!(
            "could not write telemetry temp {}: {e}",
            tmp_path.display()
        ))
    })?;
    std::fs::rename(&tmp_path, &final_path).map_err(|e| {
        AttuneError::Storage(format!("could not rename telemetry temp into place: {e}"))
    })?;
    Ok(final_path)
}

/// Read the sidecar back. Returns Ok(None) when the file does not
/// exist — legacy runs without telemetry simply have no card details.
pub fn read(session_dir: &Path, agent_id: &str) -> Result<Option<AgentRunTelemetry>> {
    let path = telemetry_path(session_dir, agent_id);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).map_err(|e| {
        AttuneError::Storage(format!("could not read telemetry {}: {e}", path.display()))
    })?;
    let parsed = serde_json::from_slice::<AgentRunTelemetry>(&bytes).map_err(|e| {
        AttuneError::Storage(format!("could not parse telemetry {}: {e}", path.display()))
    })?;
    Ok(Some(parsed))
}

/// Best-effort cost estimate. The table is intentionally small — the
/// canonical pricing is the provider's billing dashboard, not us.
/// Unknown models return None so the UI shows "—" instead of a wrong
/// number. Prices are USD per 1M input / output tokens; data taken
/// from the provider docs at the time of writing.
pub fn estimate_cost_usd(
    provider: ProviderId,
    model: &str,
    prompt: u32,
    completion: u32,
) -> Option<f32> {
    let (per_million_input, per_million_output): (f32, f32) = match (provider, model) {
        (ProviderId::OpenAi, m) if m.starts_with("gpt-4o-mini") => (0.15, 0.60),
        (ProviderId::OpenAi, m) if m.starts_with("gpt-4o") => (2.50, 10.00),
        (ProviderId::OpenAi, m) if m.starts_with("gpt-4.1-mini") => (0.40, 1.60),
        (ProviderId::OpenAi, m) if m.starts_with("gpt-4.1") => (2.00, 8.00),
        (ProviderId::OpenAi, m) if m.starts_with("o4-mini") => (1.10, 4.40),
        _ => return None,
    };
    let cost =
        (prompt as f32 * per_million_input + completion as f32 * per_million_output) / 1_000_000.0;
    Some(cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AgentRunTelemetry {
        AgentRunTelemetry {
            agent_id: "extract-tasks".into(),
            provider: ProviderId::OpenAi,
            model: "gpt-4o-mini".into(),
            latency_ms: 1234,
            prompt_tokens: Some(800),
            completion_tokens: Some(200),
            total_tokens: Some(1000),
            estimated_cost_usd: Some(0.000_3),
            tool_calls: vec![ToolCallTrace {
                tool: "create_task".into(),
                arguments_json: r#"{"title":"Ship redesign","owner":"Ege"}"#.into(),
                result: "ok 1234".into(),
                latency_ms: 12,
            }],
            evidence_spans: vec![EvidenceSpan {
                label: Some("task: ship redesign".into()),
                span_text: "ship the redesign by Friday".into(),
            }],
            previous_response: Some("Created 2 tasks.".into()),
            finished_at: Utc::now(),
        }
    }

    #[test]
    fn round_trip_via_disk() {
        let dir = tempfile::tempdir().unwrap();
        let saved_path = save(dir.path(), &sample()).unwrap();
        assert!(saved_path.exists());
        let loaded = read(dir.path(), "extract-tasks").unwrap().unwrap();
        assert_eq!(loaded.agent_id, "extract-tasks");
        assert_eq!(loaded.tool_calls.len(), 1);
        assert_eq!(loaded.evidence_spans.len(), 1);
    }

    #[test]
    fn missing_telemetry_returns_none_not_error() {
        let dir = tempfile::tempdir().unwrap();
        let res = read(dir.path(), "nothing-here").unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn cost_estimate_for_known_model() {
        // 800 input + 200 output at gpt-4o-mini:
        //   (800*0.15 + 200*0.60)/1e6 = (120 + 120)/1e6 = 0.00024
        let c = estimate_cost_usd(ProviderId::OpenAi, "gpt-4o-mini", 800, 200).unwrap();
        assert!((c - 0.000_240).abs() < 1e-6, "{c}");
    }

    #[test]
    fn cost_estimate_returns_none_for_unknown_model() {
        assert!(estimate_cost_usd(ProviderId::OpenAi, "made-up-model", 100, 100).is_none());
    }

    #[test]
    fn telemetry_path_is_predictable() {
        let p = telemetry_path(Path::new("/r/foo"), "summarize");
        assert!(p
            .to_string_lossy()
            .ends_with("agent_runs/summarize.telemetry.json"));
    }
}
