//! End-to-end speaker identification for a recorded session (GET-189).
//!
//! Ties the diarizer, the embedding extractor, and the cross-recording
//! [`crate::speaker_memory`] registry together:
//!
//! 1. diarize `<session>/system.wav` and label the transcript in place;
//! 2. compute one representative embedding per diarized cluster;
//! 3. match each embedding against the registry, resolving a display name
//!    for clusters the user has named before.
//!
//! The result is a [`SessionSpeakers`] sidecar (embeddings + resolved
//! names) the caller persists. Naming a speaker (writing back to the
//! registry) is a separate, user-driven step in the command layer — this
//! function only *reads* the registry, never mutates it.

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

/// What an identify pass produced.
#[derive(Debug, Clone, Default)]
pub struct SpeakerIdentification {
    /// Labelling stats (speakers found, segments labelled).
    pub outcome: DiarizationOutcome,
    /// Per-cluster embeddings + resolved names, ready to persist.
    pub speakers: SessionSpeakers,
}

/// Diarize the session, label `transcript` in place, embed each speaker,
/// and resolve names against `registry`.
///
/// Returns an empty result (no speakers) for a mic-only recording with no
/// `system.wav`. Errors only on diarizer/embedder failure or missing
/// models; a registry miss is normal and yields an unnamed speaker.
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

    // Read the system audio once as mono at the model rate (16 kHz) and
    // reuse it for both diarization and embedding so segment times map to
    // sample indices exactly.
    let rate = runtime.sample_rate();
    let samples = read_wav_as_mono(&system_wav, rate)?;
    let diarized = runtime.diarize_samples(&samples)?;

    let outcome = assign_to_transcript(transcript, &diarized);

    // Embed each cluster, then resolve a name from the registry.
    let embedder = SpeakerEmbedder::from_store(&store, opts.num_threads)?;
    let embeddings = embed_speakers(&embedder, &samples, &diarized);

    let mut speakers: Vec<SessionSpeaker> = Vec::new();
    for (cluster, embedding) in embeddings {
        let (name, registry_id, auto_named) = resolve_name(registry, &embedding);
        speakers.push(SessionSpeaker {
            cluster,
            name,
            registry_id,
            auto_named,
            embedding,
        });
    }

    // Include clusters that were labelled but too short to embed, so the
    // UI and rename flow still see every speaker (just not yet teachable).
    for cluster in diarized.iter().map(|d| d.speaker) {
        if !speakers.iter().any(|s| s.cluster == cluster) {
            speakers.push(SessionSpeaker {
                cluster,
                name: None,
                registry_id: None,
                auto_named: false,
                embedding: Vec::new(),
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

/// Best-effort: anchor the user's "You" voice into `registry` from the
/// session's mic track (VAD-filtered `mic.speech.wav` when present, else
/// `mic.wav`). This is what lets future recordings suppress mic bleed on
/// the system stream and tell the user apart from other speakers.
///
/// Returns `Ok(true)` when an anchor exemplar was added (caller should
/// persist the registry), `Ok(false)` when there was no usable mic audio.
/// Does not save — the caller owns persistence.
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

/// A deterministic UUID from a seed (sha256 → first 16 bytes). Avoids the
/// uuid `v5` feature while still giving stable, collision-resistant ids.
fn stable_uuid(seed: &[u8]) -> Uuid {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(seed);
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

/// Stable per-recording UUID derived from the session directory name, so
/// re-running identify on the same recording doesn't multiply provenance.
pub fn recording_uuid(session_dir: &Path) -> Uuid {
    let name = session_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("session");
    stable_uuid(format!("attune-recording:{name}").as_bytes())
}

/// Stable per-install device UUID. Cross-device sync (P4) will replace this
/// with a real device identity; for now provenance only needs determinism.
pub fn local_device_uuid() -> Uuid {
    let home = std::env::var("HOME").unwrap_or_else(|_| "attune-local-device".to_string());
    stable_uuid(format!("attune-device:{home}").as_bytes())
}

/// Current Unix epoch milliseconds.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Turn a registry match into a (name, registry_id, auto_named) triple.
/// Only a high-confidence `AutoName` (or a `SelfUser` hit) applies a name
/// automatically; `Confirm`/`New` stay unnamed so a borderline match never
/// silently mislabels a stranger.
fn resolve_name(
    registry: &SpeakerRegistry,
    embedding: &[f32],
) -> (Option<String>, Option<String>, bool) {
    match registry.match_embedding(embedding) {
        MatchOutcome::SelfUser { .. } => (Some("You".to_string()), None, true),
        MatchOutcome::AutoName { id, .. } => (
            registry.record(id).map(|r| r.display_name.clone()),
            Some(id.to_string()),
            true,
        ),
        MatchOutcome::Confirm { .. } | MatchOutcome::New => (None, None, false),
    }
}
