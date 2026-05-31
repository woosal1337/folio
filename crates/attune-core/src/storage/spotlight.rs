//! macOS Spotlight indexing via plain-text sidecars.
//!
//! Spotlight indexes the user's home directory automatically. Rather
//! than bind to NSCoreSpotlight + objc, we write a tiny `index.txt`
//! sidecar inside each recording session_dir with a flattened,
//! greppable view of the transcript + metadata. Spotlight picks it
//! up for free and Cmd-Space "whisper decision" surfaces the
//! matching session.
//!
//! Same approach for memories: the canonical markdown files already
//! sit in a Spotlight-indexed directory, so the only thing we add
//! is the recording sidecar.
//!
//! v2 roadmap finding 075 / GET-77. The full NSCoreSpotlight bridge
//! (which lets us populate result previews with audio thumbnails +
//! attune:// deep-links) stays as the follow-up.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AttuneError, Result};
use crate::transcription::Transcript;

const SIDECAR_FILENAME: &str = "index.txt";

/// Write the Spotlight-friendly sidecar for a recording. Includes the
/// label, transcript text (flattened across channels), and any
/// suggested title/tags from the autoname agent.
pub fn write_recording_sidecar(
    session_dir: &Path,
    label: &str,
    transcript: Option<&Transcript>,
    suggested_title: Option<&str>,
    suggested_tags: &[String],
) -> Result<PathBuf> {
    let mut out = String::new();
    out.push_str("Attune meeting\n");
    out.push_str(&format!("Label: {label}\n"));
    if let Some(t) = suggested_title {
        out.push_str(&format!("Title: {t}\n"));
    }
    if !suggested_tags.is_empty() {
        out.push_str(&format!("Tags: {}\n", suggested_tags.join(", ")));
    }
    out.push('\n');
    if let Some(t) = transcript {
        if let Some(lang) = t.language.as_deref() {
            out.push_str(&format!("Language: {lang}\n\n"));
        }
        for seg in &t.segments {
            out.push_str(seg.text.trim());
            out.push('\n');
        }
    } else {
        out.push_str("(transcript pending)\n");
    }
    let path = session_dir.join(SIDECAR_FILENAME);
    fs::write(&path, out).map_err(|e| {
        AttuneError::Storage(format!(
            "could not write spotlight sidecar {}: {e}",
            path.display()
        ))
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::TranscriptSegment;

    fn sample_transcript() -> Transcript {
        Transcript {
            language: Some("en".into()),
            segments: vec![
                TranscriptSegment {
                    start_seconds: 0.0,
                    end_seconds: 1.0,
                    text: "We decided to ship Pro at $79.".into(),
                    speaker: None,
                    language: None,
                },
                TranscriptSegment {
                    start_seconds: 1.0,
                    end_seconds: 2.0,
                    text: "Alice will draft the announcement.".into(),
                    speaker: None,
                    language: None,
                },
            ],
        }
    }

    #[test]
    fn write_sidecar_includes_label_title_tags_and_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_recording_sidecar(
            dir.path(),
            "2026-05-25-pricing",
            Some(&sample_transcript()),
            Some("Pricing sync"),
            &["pricing".to_string(), "decision".to_string()],
        )
        .unwrap();
        assert!(path.exists());
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("Pricing sync"));
        assert!(body.contains("Tags: pricing, decision"));
        assert!(body.contains("ship Pro at $79"));
        assert!(body.contains("Alice will draft"));
    }

    #[test]
    fn write_sidecar_handles_missing_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_recording_sidecar(dir.path(), "raw", None, None, &[]).unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("(transcript pending)"));
    }
}
