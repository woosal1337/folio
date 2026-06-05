use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AttuneError, Result};

const DEMO_LABEL: &str = "0000-00-00-onboarding-demo";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoTranscriptSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub channel: String,
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoBundle {
    pub label: String,
    pub duration_seconds: f64,
    pub created_at: DateTime<Utc>,
    pub segments: Vec<DemoTranscriptSegment>,
    pub summary_markdown: String,
    pub tasks: Vec<DemoTask>,
    pub memories: Vec<DemoMemory>,
    pub decisions: Vec<DemoDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoTask {
    pub title: String,
    pub owner: Option<String>,
    pub due: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoMemory {
    pub kind: String,
    pub content: String,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoDecision {
    pub statement: String,
    pub rationale: Option<String>,
}

pub fn bundle() -> DemoBundle {
    let started_at = Utc::now();
    DemoBundle {
        label: DEMO_LABEL.to_string(),
        duration_seconds: 92.0,
        created_at: started_at,
        segments: vec![
            seg(0.0, 4.5, "mic", "You", "Welcome to Attune. This is a demo meeting that lasts about ninety seconds."),
            seg(4.5, 9.0, "system", "Alice", "Quick check, can you hear me through the system audio capture?"),
            seg(9.0, 12.5, "mic", "You", "Yep, both channels are recording. Let's go through what's on the agenda."),
            seg(12.5, 21.0, "system", "Alice", "First item, we agreed last week to ship the redesign by Friday. I'll handle the public announcement on the same day."),
            seg(21.0, 28.0, "mic", "You", "Sounds good. I'll merge the migration PR by Wednesday so QA has a full day."),
            seg(28.0, 36.0, "system", "Alice", "Bob is on holiday but he flagged that the legal review of the new pricing tier blocks our launch email."),
            seg(36.0, 44.0, "mic", "You", "I'll chase legal tomorrow morning. If they push back we'll ship the redesign first and the pricing change a week later."),
            seg(44.0, 52.0, "system", "Alice", "Decision then, redesign Friday, pricing change conditional on legal sign-off, no later than the following Friday."),
            seg(52.0, 60.0, "mic", "You", "Agreed. One more thing, I want to start logging customer pain points as memories so we have them next time we open the pricing deck."),
            seg(60.0, 69.0, "system", "Alice", "Good idea. The two themes I keep hearing are confusing tier names and the missing teams plan."),
            seg(69.0, 78.0, "mic", "You", "Noted. Let's wrap, I have another meeting at the top of the hour."),
            seg(78.0, 86.0, "system", "Alice", "Cheers, talk Friday after the launch."),
            seg(86.0, 92.0, "mic", "You", "Thanks Alice, talk soon."),
        ],
        summary_markdown: "## Summary\n\n\
            A quick standup-style meeting between You and Alice ahead of a Friday \
            launch.\n\n\
            ### Highlights\n\
            - Redesign ships Friday; Alice owns the announcement.\n\
            - Migration PR merges by Wednesday so QA has a full day.\n\
            - Pricing tier launch is gated on legal sign-off; if blocked, ships a week later.\n\
            - Two customer pain themes: confusing tier names, missing teams plan.\n".into(),
        tasks: vec![
            DemoTask {
                title: "Merge migration PR".into(),
                owner: Some("You".into()),
                due: Some("Wednesday".into()),
            },
            DemoTask {
                title: "Chase legal on pricing tier review".into(),
                owner: Some("You".into()),
                due: Some("Tomorrow morning".into()),
            },
            DemoTask {
                title: "Publish redesign announcement".into(),
                owner: Some("Alice".into()),
                due: Some("Friday".into()),
            },
        ],
        memories: vec![
            DemoMemory {
                kind: "claim".into(),
                content: "Customers find the current tier names confusing.".into(),
                key: Some("customer.feedback.tier-names".into()),
            },
            DemoMemory {
                kind: "claim".into(),
                content: "There is unmet demand for a teams plan.".into(),
                key: Some("customer.feedback.teams-plan".into()),
            },
            DemoMemory {
                kind: "person".into(),
                content: "Alice owns external announcements for the redesign.".into(),
                key: Some("person.alice".into()),
            },
        ],
        decisions: vec![
            DemoDecision {
                statement: "Ship redesign on Friday.".into(),
                rationale: Some("Migration and QA window fit a Wednesday merge.".into()),
            },
            DemoDecision {
                statement: "Pricing tier launch is conditional on legal sign-off, max one week slip.".into(),
                rationale: Some("Legal review may block; we ship redesign first if so.".into()),
            },
        ],
    }
}

fn seg(start: f64, end: f64, channel: &str, speaker: &str, text: &str) -> DemoTranscriptSegment {
    DemoTranscriptSegment {
        start_seconds: start,
        end_seconds: end,
        channel: channel.into(),
        speaker: speaker.into(),
        text: text.into(),
    }
}

pub fn materialise(recordings_root: &Path) -> Result<PathBuf> {
    let session_dir = recordings_root.join(DEMO_LABEL);
    fs::create_dir_all(&session_dir).map_err(|e| {
        AttuneError::Storage(format!(
            "could not create demo session dir {}: {e}",
            session_dir.display()
        ))
    })?;
    let bundle = bundle();
    let body = serde_json::to_string_pretty(&bundle)
        .map_err(|e| AttuneError::Storage(format!("could not serialise demo bundle: {e}")))?;
    fs::write(session_dir.join("demo.json"), body).map_err(|e| {
        AttuneError::Storage(format!(
            "could not write demo.json into {}: {e}",
            session_dir.display()
        ))
    })?;
    Ok(session_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_has_consistent_segment_ordering() {
        let b = bundle();
        for window in b.segments.windows(2) {
            let (a, b) = (&window[0], &window[1]);
            assert!(
                a.start_seconds <= b.start_seconds,
                "segments must be sorted by start_seconds"
            );
        }
    }

    #[test]
    fn bundle_duration_matches_last_segment_end() {
        let b = bundle();
        let last = b.segments.last().unwrap();
        assert!(
            (b.duration_seconds - last.end_seconds).abs() < 0.5,
            "duration ({}) should match last segment end ({})",
            b.duration_seconds,
            last.end_seconds
        );
    }

    #[test]
    fn bundle_uses_both_channels() {
        let b = bundle();
        let mic = b.segments.iter().filter(|s| s.channel == "mic").count();
        let sys = b.segments.iter().filter(|s| s.channel == "system").count();
        assert!(mic > 0, "demo must include mic segments");
        assert!(sys > 0, "demo must include system segments");
    }

    #[test]
    fn materialise_writes_demo_json() {
        let dir = tempfile::tempdir().unwrap();
        let session = materialise(dir.path()).unwrap();
        assert!(session.join("demo.json").is_file());
        let raw = fs::read_to_string(session.join("demo.json")).unwrap();
        let parsed: DemoBundle = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.label, DEMO_LABEL);
    }

    #[test]
    fn materialise_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let first = materialise(dir.path()).unwrap();
        let second = materialise(dir.path()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn bundle_payload_has_all_four_output_kinds() {
        let b = bundle();
        assert!(!b.summary_markdown.trim().is_empty());
        assert!(!b.tasks.is_empty(), "demo must include at least one task");
        assert!(
            !b.memories.is_empty(),
            "demo must include at least one memory"
        );
        assert!(
            !b.decisions.is_empty(),
            "demo must include at least one decision"
        );
    }
}
