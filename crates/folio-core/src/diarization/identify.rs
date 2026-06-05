use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::diarization::embedding::{embed_speakers, embed_wav_file, SpeakerEmbedder};
use crate::diarization::label::assign_to_transcript;
use crate::diarization::models::DiarizationModelStore;
use crate::diarization::runtime::{read_wav_as_mono, DiarizationError, DiarizationRuntime};
use crate::diarization::session_speakers::{SessionSpeaker, SessionSpeakers};
use crate::diarization::{DiarizationOptions, DiarizationOutcome};
use crate::speaker_memory::{MatchOutcome, SpeakerRegistry};
use crate::transcription::SessionTranscript;

#[derive(Debug, Clone, Default)]
pub struct SpeakerIdentification {
    pub outcome: DiarizationOutcome,

    pub speakers: SessionSpeakers,
}

pub fn identify_session_speakers(
    session_dir: &Path,
    transcript: &mut SessionTranscript,
    opts: &DiarizationOptions,
    registry: &SpeakerRegistry,
) -> Result<SpeakerIdentification, DiarizationError> {
    let store = DiarizationModelStore::default_location();
    let runtime = DiarizationRuntime::from_store(&store, opts)?;

    let system_wav = session_dir.join("system.wav");
    if !system_wav.is_file() {
        return Ok(SpeakerIdentification::default());
    }

    let rate = runtime.sample_rate();
    let samples = read_wav_as_mono(&system_wav, rate)?;
    let diarized = runtime.diarize_samples(&samples)?;

    let outcome = assign_to_transcript(transcript, &diarized);

    let embedder = SpeakerEmbedder::from_store(&store, opts.num_threads)?;
    let embeddings = embed_speakers(&embedder, &samples, &diarized);

    let mut speakers: Vec<SessionSpeaker> = Vec::new();
    for (cluster, embedding) in embeddings {
        let resolved = resolve_name(registry, &embedding);
        speakers.push(SessionSpeaker {
            cluster,
            name: resolved.name,
            registry_id: resolved.registry_id,
            auto_named: resolved.auto_named,
            embedding,
            suggested_name: resolved.suggested_name,
            suggested_registry_id: resolved.suggested_registry_id,
            suggested_score: resolved.suggested_score,
        });
    }

    for cluster in diarized.iter().map(|d| d.speaker) {
        if !speakers.iter().any(|s| s.cluster == cluster) {
            speakers.push(SessionSpeaker {
                cluster,
                name: None,
                registry_id: None,
                auto_named: false,
                embedding: Vec::new(),
                suggested_name: None,
                suggested_registry_id: None,
                suggested_score: None,
            });
        }
    }
    speakers.sort_by_key(|s| s.cluster);
    speakers.dedup_by_key(|s| s.cluster);

    Ok(SpeakerIdentification {
        outcome,
        speakers: SessionSpeakers {
            version: 1,
            speakers,
        },
    })
}

pub fn anchor_self_from_session(
    registry: &mut SpeakerRegistry,
    session_dir: &Path,
    opts: &DiarizationOptions,
) -> Result<bool, DiarizationError> {
    let store = DiarizationModelStore::default_location();
    let embedder = SpeakerEmbedder::from_store(&store, opts.num_threads)?;

    let mic_speech = session_dir.join("mic.speech.wav");
    let mic = if mic_speech.is_file() {
        mic_speech
    } else {
        session_dir.join("mic.wav")
    };
    if !mic.is_file() {
        return Ok(false);
    }

    let Some(embedding) = embed_wav_file(&embedder, &mic)? else {
        return Ok(false);
    };
    registry
        .anchor_self(
            &embedding,
            recording_uuid(session_dir),
            local_device_uuid(),
            now_ms(),
        )
        .map_err(|e| DiarizationError::Runtime(format!("anchor_self: {e}")))?;
    Ok(true)
}

fn stable_uuid(seed: &[u8]) -> Uuid {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(seed);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

pub fn recording_uuid(session_dir: &Path) -> Uuid {
    let name = session_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("session");
    stable_uuid(format!("folio-recording:{name}").as_bytes())
}

pub fn local_device_uuid() -> Uuid {
    let home = std::env::var("HOME").unwrap_or_else(|_| "folio-local-device".to_string());
    stable_uuid(format!("folio-device:{home}").as_bytes())
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Default)]
struct ResolvedSpeaker {
    name: Option<String>,
    registry_id: Option<String>,
    auto_named: bool,
    suggested_name: Option<String>,
    suggested_registry_id: Option<String>,
    suggested_score: Option<f32>,
}

fn resolve_name(registry: &SpeakerRegistry, embedding: &[f32]) -> ResolvedSpeaker {
    match registry.match_embedding(embedding) {
        MatchOutcome::SelfUser { .. } => ResolvedSpeaker::default(),
        MatchOutcome::AutoName { id, .. } => ResolvedSpeaker {
            name: registry.record(id).map(|r| r.display_name.clone()),
            registry_id: Some(id.to_string()),
            auto_named: true,
            ..Default::default()
        },
        MatchOutcome::Confirm { id, score } => ResolvedSpeaker {
            suggested_name: registry.record(id).map(|r| r.display_name.clone()),
            suggested_registry_id: Some(id.to_string()),
            suggested_score: Some(score),
            ..Default::default()
        },
        MatchOutcome::New => ResolvedSpeaker::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speaker_memory::{NameTarget, EMBED_DIM};

    #[test]
    fn confirm_tier_match_becomes_a_suggestion_not_a_name() {
        let mut reg = SpeakerRegistry::new();
        let emb = vec![0.3_f32; EMBED_DIM];
        let id = reg
            .name_speaker(
                NameTarget::New {
                    display_name: "Alice".into(),
                },
                &emb,
                Uuid::nil(),
                Uuid::nil(),
                Some(0),
                0,
            )
            .unwrap();

        let resolved = resolve_name(&reg, &emb);

        assert_eq!(resolved.name, None);
        assert!(!resolved.auto_named);

        assert_eq!(resolved.suggested_name.as_deref(), Some("Alice"));
        assert_eq!(
            resolved.suggested_registry_id.as_deref(),
            Some(id.to_string().as_str())
        );
        assert!(resolved.suggested_score.unwrap() >= 0.60);
    }

    #[test]
    fn no_match_yields_neither_name_nor_suggestion() {
        let reg = SpeakerRegistry::new();
        let resolved = resolve_name(&reg, &vec![0.1_f32; EMBED_DIM]);
        assert!(resolved.name.is_none());
        assert!(resolved.suggested_name.is_none());
        assert!(resolved.suggested_registry_id.is_none());
    }
}
