//! Per-session speaker labels (GET-189): list the diarized speakers of a
//! recording and rename them. A rename updates the recording's sidecar AND
//! teaches the cross-recording registry the voice → name link, so future
//! recordings can auto-detect the same speaker.

use std::path::Path;

use attune_core::diarization::{
    local_device_uuid, now_ms, recording_uuid, SessionSpeakers, SpeakerLabel,
};
use attune_core::speaker_memory::{self, NameTarget, SpeakerRegistry};

/// List the speakers identified for a recording: cluster id, current name
/// (if any), and provenance. Empty when the recording was never diarized.
#[tauri::command]
pub async fn list_session_speakers(session_dir: String) -> Result<Vec<SpeakerLabel>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        match SessionSpeakers::read(Path::new(&session_dir)) {
            Ok(Some(s)) => Ok(s.labels()),
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e.to_string()),
        }
    })
    .await
    .map_err(|e| format!("list_session_speakers task panicked: {e}"))?
}

/// Rename a diarized speaker. Updates this recording's sidecar and — when
/// the cluster carries a voice embedding — teaches the registry the
/// voice → name link so future recordings auto-detect this speaker.
/// Returns the updated label set.
#[tauri::command]
pub async fn rename_session_speaker(
    session_dir: String,
    cluster: i32,
    name: String,
) -> Result<Vec<SpeakerLabel>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<SpeakerLabel>, String> {
        let dir = Path::new(&session_dir);
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Err("name cannot be empty".to_string());
        }

        let mut speakers = SessionSpeakers::read(dir)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "this recording has no diarized speakers to rename".to_string())?;

        let speaker = speakers
            .get_mut(cluster)
            .ok_or_else(|| format!("no speaker with cluster id {cluster}"))?;

        // Teach the registry whenever we have a voice embedding to learn
        // from (a cluster with too little audio carries none).
        if !speaker.embedding.is_empty() {
            let mut registry = speaker_memory::load_default().map_err(|e| e.to_string())?;
            let target = resolve_target(&registry, &trimmed);
            let id = registry
                .name_speaker(
                    target,
                    &speaker.embedding,
                    recording_uuid(dir),
                    local_device_uuid(),
                    Some(now_ms()),
                    now_ms(),
                )
                .map_err(|e| e.to_string())?;
            speaker_memory::save_default(&registry).map_err(|e| e.to_string())?;
            speaker.registry_id = Some(id.to_string());
        }

        speaker.name = Some(trimmed);
        speaker.auto_named = false; // an explicit user rename, not a guess
        speakers.write(dir).map_err(|e| e.to_string())?;
        Ok(speakers.labels())
    })
    .await
    .map_err(|e| format!("rename_session_speaker task panicked: {e}"))?
}

/// Where the exemplar attaches: merge into an existing identity that
/// already carries this exact display name (so naming "Speaker 2" → "Alice"
/// across two recordings builds one Alice, and re-confirming an auto-name
/// reinforces it), otherwise create a brand-new identity. A correction
/// (typing a different name than the auto-guess) therefore starts a fresh
/// identity rather than poisoning the wrongly-guessed one.
fn resolve_target(registry: &SpeakerRegistry, name: &str) -> NameTarget {
    if let Some(r) = registry
        .records
        .iter()
        .find(|r| !r.is_self && !r.deleted && r.display_name.eq_ignore_ascii_case(name))
    {
        NameTarget::Existing { id: r.id }
    } else {
        NameTarget::New {
            display_name: name.to_string(),
        }
    }
}
