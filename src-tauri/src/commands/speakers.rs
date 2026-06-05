use std::path::Path;

use attune_core::diarization::{
    local_device_uuid, now_ms, recording_uuid, SessionSpeakers, SpeakerLabel,
};
use attune_core::speaker_memory::{self, NameTarget, SpeakerRegistry};
use uuid::Uuid;

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
        speaker.auto_named = false;
        clear_suggestion(speaker);
        speakers.write(dir).map_err(|e| e.to_string())?;
        Ok(speakers.labels())
    })
    .await
    .map_err(|e| format!("rename_session_speaker task panicked: {e}"))?
}

#[tauri::command]
pub async fn confirm_session_speaker(
    session_dir: String,
    cluster: i32,
) -> Result<Vec<SpeakerLabel>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<SpeakerLabel>, String> {
        let dir = Path::new(&session_dir);
        let mut speakers = SessionSpeakers::read(dir)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "this recording has no diarized speakers".to_string())?;
        let speaker = speakers
            .get_mut(cluster)
            .ok_or_else(|| format!("no speaker with cluster id {cluster}"))?;

        let name = speaker
            .suggested_name
            .clone()
            .ok_or_else(|| "no pending suggestion for this speaker".to_string())?;
        let id = speaker
            .suggested_registry_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| "suggestion has no valid registry id".to_string())?;

        if !speaker.embedding.is_empty() {
            let mut registry = speaker_memory::load_default().map_err(|e| e.to_string())?;
            registry
                .add_exemplar(id, &speaker.embedding, recording_uuid(dir), now_ms())
                .map_err(|e| e.to_string())?;
            speaker_memory::save_default(&registry).map_err(|e| e.to_string())?;
        }

        speaker.name = Some(name);
        speaker.registry_id = Some(id.to_string());
        speaker.auto_named = false;
        clear_suggestion(speaker);
        speakers.write(dir).map_err(|e| e.to_string())?;
        Ok(speakers.labels())
    })
    .await
    .map_err(|e| format!("confirm_session_speaker task panicked: {e}"))?
}

#[tauri::command]
pub async fn reject_session_speaker(
    session_dir: String,
    cluster: i32,
) -> Result<Vec<SpeakerLabel>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<SpeakerLabel>, String> {
        let dir = Path::new(&session_dir);
        let mut speakers = SessionSpeakers::read(dir)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "this recording has no diarized speakers".to_string())?;
        let speaker = speakers
            .get_mut(cluster)
            .ok_or_else(|| format!("no speaker with cluster id {cluster}"))?;

        if let Some(id) = speaker
            .suggested_registry_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok())
        {
            if !speaker.embedding.is_empty() {
                let mut registry = speaker_memory::load_default().map_err(|e| e.to_string())?;

                if registry.record(id).is_some() {
                    registry
                        .add_negative_exemplar(id, &speaker.embedding, now_ms())
                        .map_err(|e| e.to_string())?;
                    speaker_memory::save_default(&registry).map_err(|e| e.to_string())?;
                }
            }
        }

        clear_suggestion(speaker);
        speakers.write(dir).map_err(|e| e.to_string())?;
        Ok(speakers.labels())
    })
    .await
    .map_err(|e| format!("reject_session_speaker task panicked: {e}"))?
}

fn clear_suggestion(speaker: &mut attune_core::diarization::SessionSpeaker) {
    speaker.suggested_name = None;
    speaker.suggested_registry_id = None;
    speaker.suggested_score = None;
}

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
