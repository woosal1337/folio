//! Cross-call speaker memory (GET-189).
//!
//! The "name a voice once, auto-name it forever" registry — the Google
//! Photos pattern translated to voice. Once a speaker is named, the named
//! identity persists across recordings: future segments whose voice
//! embedding is close enough to a stored exemplar are auto-labelled, and
//! medium-confidence matches surface an "is this <name>?" confirmation.
//!
//! ## What this module owns (P0)
//!
//! - The [`NamedVoiceRecord`] data model: a named identity backed by a
//!   small set of L2-normalized 256-dim voice embeddings ("exemplars"),
//!   plus "not this person" negative exemplars.
//! - The [`SpeakerRegistry`]: the in-memory collection plus its on-disk,
//!   AES-256-GCM-encrypted persistence (biometric data is never stored in
//!   plaintext — it rides the same [`crate::encryption`] envelope the
//!   recordings use).
//! - The three-tier confidence decision ([`SpeakerRegistry::match_embedding`]):
//!   auto-name ≥ [`AUTO_NAME_THRESHOLD`] (with a ≥ [`MIN_EXEMPLARS_FOR_AUTONAME`]
//!   guard), confirm in `[CONFIRM_THRESHOLD, AUTO_NAME_THRESHOLD)`, new
//!   below.
//! - The "You" anchor + not-self filter so the user's own voice bleeding
//!   onto the system stream never spawns a phantom other-speaker.
//!
//! ## What it does NOT own yet
//!
//! - The embedding *source*. Embeddings come from the diarization
//!   WeSpeaker model (`crate::diarization::DiarizationRuntime::embed_segment`),
//!   which is stubbed until diarization P2. This module is deliberately
//!   embedding-source-agnostic: it matches any 256-dim vectors, so it is
//!   fully testable today against deterministic vectors and slots onto
//!   real embeddings the moment the diarization runtime lands.
//! - Cloud sync, the consent modal, and the LLM rename pass — later
//!   phases (P4/P5) in
//!   `obsidian.md/projects/attune/plan/diarization-v1-execution.md` and
//!   the GET-189 ticket. The on-disk format and the `version` counter are
//!   shaped so an HLC-versioned cloud blob is a drop-in extension.
//!
//! Threshold and capacity defaults come from the speaker-verification
//! literature survey in
//! `obsidian.md/projects/attune/research/speaker-memory-cross-call-registry-2026-05-31.md`.

pub mod store;

use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AttuneError, Result};

pub use store::{default_registry_path, load_default, registry_passphrase, save_default};

/// Dimensionality of the WeSpeaker ResNet34-LM embeddings the registry
/// stores. Embeddings of any other length are rejected so a mismatched
/// model can't silently corrupt the cosine math.
pub const EMBED_DIM: usize = 256;

/// Maximum exemplars kept per identity. Bounds blob size and match cost;
/// when full, the oldest exemplar is evicted (see
/// [`SpeakerRegistry::add_exemplar`]).
pub const MAX_EXEMPLARS: usize = 20;

/// Maximum "not this person" negative exemplars kept per identity.
pub const MAX_NEGATIVE_EXEMPLARS: usize = 10;

/// Cosine similarity at/above which a match is auto-named (label applied
/// silently). Deliberately conservative to minimise false-positive
/// privacy disasters.
pub const AUTO_NAME_THRESHOLD: f32 = 0.82;

/// Cosine similarity at/above which a match surfaces an "is this <name>?"
/// confirmation rather than auto-naming.
pub const CONFIRM_THRESHOLD: f32 = 0.60;

/// An identity must have at least this many exemplars before it can
/// auto-name. The single most important guard against false-positive
/// cascades: a one-shot identity only ever *confirms*, never auto-names.
pub const MIN_EXEMPLARS_FOR_AUTONAME: usize = 3;

/// Cosine similarity to the "You" anchor at/above which a segment is
/// treated as the user themselves (mic bleed on the system stream),
/// suppressing it from other-speaker matching.
pub const SELF_MATCH_THRESHOLD: f32 = 0.90;

/// Outcome of matching a query embedding against the registry.
#[derive(Clone, Debug, PartialEq)]
pub enum MatchOutcome {
    /// The segment is the user (≥ [`SELF_MATCH_THRESHOLD`] to the "You"
    /// anchor). Callers suppress it from other-speaker labelling.
    SelfUser { score: f32 },
    /// High confidence and enough exemplars: apply the label silently.
    AutoName { id: Uuid, score: f32 },
    /// Medium confidence: hold as an unnamed speaker and queue an
    /// "is this <name>?" confirmation card.
    Confirm { id: Uuid, score: f32 },
    /// No usable match — a new (unnamed) speaker.
    New,
}

/// One named identity in the registry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedVoiceRecord {
    /// Stable identity UUID.
    pub id: Uuid,
    /// User-provided display name (e.g. "Fatih").
    pub display_name: String,
    /// Creation time, Unix epoch milliseconds.
    pub created_at_ms: i64,
    /// Last-mutation time, Unix epoch milliseconds (HLC-style; drives
    /// last-write-wins merge once cloud sync lands).
    pub updated_at_ms: i64,
    /// Up to [`MAX_EXEMPLARS`] L2-normalized [`EMBED_DIM`]-d embeddings.
    pub exemplars: Vec<Vec<f32>>,
    /// Source recording for each exemplar (parallel to `exemplars`).
    pub exemplar_recording_ids: Vec<Uuid>,
    /// Up to [`MAX_NEGATIVE_EXEMPLARS`] "not this person" embeddings,
    /// recorded when a user rejects an auto/confirm match.
    pub negative_exemplars: Vec<Vec<f32>>,
    /// Device that created the record (for cross-device sync later).
    pub source_device_id: Uuid,
    /// When the user granted consent to remember this voice (BIPA/GDPR).
    pub consent_granted_at_ms: Option<i64>,
    /// True for the user's own "You" anchor.
    pub is_self: bool,
    /// Tombstone for right-to-erasure: when true the biometric exemplars
    /// have been purged and the record no longer matches.
    pub deleted: bool,
    /// Deletion time, Unix epoch milliseconds.
    pub deleted_at_ms: Option<i64>,
}

impl NamedVoiceRecord {
    /// Live = not tombstoned and still carrying at least one exemplar.
    fn is_live(&self) -> bool {
        !self.deleted && !self.exemplars.is_empty()
    }

    /// Best (max) cosine similarity of `query` to any positive exemplar.
    /// `query` must already be L2-normalized. Returns -1.0 when empty.
    fn positive_similarity(&self, query: &[f32]) -> f32 {
        max_cosine(query, &self.exemplars)
    }

    /// Best (max) cosine similarity of `query` to any negative exemplar.
    fn negative_similarity(&self, query: &[f32]) -> f32 {
        max_cosine(query, &self.negative_exemplars)
    }
}

/// Where to attach an exemplar when naming: a brand-new identity or an
/// existing one.
#[derive(Clone, Debug)]
pub enum NameTarget {
    /// Create a new identity with this display name.
    New { display_name: String },
    /// Add to / rename an existing identity.
    Existing { id: Uuid },
}

/// The speaker-memory registry: a set of named identities plus encrypted
/// on-disk persistence.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpeakerRegistry {
    /// Monotonic mutation counter. Bumped on every change; the seed of
    /// the HLC version used for last-write-wins cloud merge later.
    #[serde(default)]
    pub version: u64,
    /// Named identities (including at most one `is_self` anchor).
    #[serde(default)]
    pub records: Vec<NamedVoiceRecord>,
}

impl SpeakerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.records.iter().all(|r| !r.is_live())
    }

    /// Count of live (non-tombstoned, non-empty) identities.
    pub fn live_len(&self) -> usize {
        self.records.iter().filter(|r| r.is_live()).count()
    }

    pub fn record(&self, id: Uuid) -> Option<&NamedVoiceRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    fn record_mut(&mut self, id: Uuid) -> Option<&mut NamedVoiceRecord> {
        self.records.iter_mut().find(|r| r.id == id)
    }

    fn self_anchor(&self) -> Option<&NamedVoiceRecord> {
        self.records.iter().find(|r| r.is_self && r.is_live())
    }

    /// Decide what to do with a query embedding.
    ///
    /// 1. If it is within [`SELF_MATCH_THRESHOLD`] of the "You" anchor,
    ///    it is the user → [`MatchOutcome::SelfUser`].
    /// 2. Otherwise score against every other live identity (best
    ///    positive-exemplar cosine, skipping any identity the query is
    ///    *closer* to a negative exemplar of), and bucket by the
    ///    three-tier thresholds.
    ///
    /// Returns [`MatchOutcome::New`] for a malformed (wrong-dimension)
    /// query rather than erroring — a bad embedding is "not a known
    /// speaker", and callers already handle New.
    pub fn match_embedding(&self, embedding: &[f32]) -> MatchOutcome {
        if embedding.len() != EMBED_DIM {
            return MatchOutcome::New;
        }
        let query = l2_normalize(embedding);

        // Self anchor first: mic bleed onto the system stream must never
        // become a phantom other-speaker.
        if let Some(anchor) = self.self_anchor() {
            let s = anchor.positive_similarity(&query);
            if s >= SELF_MATCH_THRESHOLD {
                return MatchOutcome::SelfUser { score: s };
            }
        }

        let mut best: Option<(Uuid, f32, usize)> = None;
        for r in self.records.iter().filter(|r| r.is_live() && !r.is_self) {
            let pos = r.positive_similarity(&query);
            let neg = r.negative_similarity(&query);
            // Closer to a "not this person" vote than to any positive
            // exemplar → this identity is actively rejected for `query`.
            if neg > pos {
                continue;
            }
            match best {
                Some((_, best_score, _)) if pos <= best_score => {}
                _ => best = Some((r.id, pos, r.exemplars.len())),
            }
        }

        let Some((id, score, exemplar_count)) = best else {
            return MatchOutcome::New;
        };
        if score >= AUTO_NAME_THRESHOLD && exemplar_count >= MIN_EXEMPLARS_FOR_AUTONAME {
            MatchOutcome::AutoName { id, score }
        } else if score >= CONFIRM_THRESHOLD {
            MatchOutcome::Confirm { id, score }
        } else {
            MatchOutcome::New
        }
    }

    /// Name a speaker: create a new identity or add an exemplar to an
    /// existing one. Returns the identity's id. The embedding is
    /// L2-normalized before storage. A wrong-dimension embedding is
    /// rejected with an error.
    pub fn name_speaker(
        &mut self,
        target: NameTarget,
        embedding: &[f32],
        recording_id: Uuid,
        device_id: Uuid,
        consent_granted_at_ms: Option<i64>,
        now_ms: i64,
    ) -> Result<Uuid> {
        validate_dim(embedding)?;
        let normed = l2_normalize(embedding);
        match target {
            NameTarget::Existing { id } => {
                // The "You" anchor is mic-derived only; naming it through
                // the normal path would inject a stranger's voice into it
                // and corrupt self-suppression. Route self exemplars
                // through anchor_self exclusively.
                if self.record(id).is_some_and(|r| r.is_self) {
                    return Err(AttuneError::Storage(format!(
                        "speaker {id}: cannot name the self anchor; use anchor_self"
                    )));
                }
                self.add_normalized_exemplar(id, normed, recording_id, now_ms)?;
                Ok(id)
            }
            NameTarget::New { display_name } => {
                let id = Uuid::new_v4();
                self.records.push(NamedVoiceRecord {
                    id,
                    display_name,
                    created_at_ms: now_ms,
                    updated_at_ms: now_ms,
                    exemplars: vec![normed],
                    exemplar_recording_ids: vec![recording_id],
                    negative_exemplars: Vec::new(),
                    source_device_id: device_id,
                    consent_granted_at_ms,
                    is_self: false,
                    deleted: false,
                    deleted_at_ms: None,
                });
                self.version += 1;
                Ok(id)
            }
        }
    }

    /// Create or update the user's "You" anchor from a mic-derived
    /// embedding. There is at most one self anchor; subsequent calls add
    /// exemplars to it.
    pub fn anchor_self(
        &mut self,
        embedding: &[f32],
        recording_id: Uuid,
        device_id: Uuid,
        now_ms: i64,
    ) -> Result<Uuid> {
        validate_dim(embedding)?;
        let normed = l2_normalize(embedding);
        // Reuse a *live* self anchor.
        if let Some(existing) = self
            .records
            .iter()
            .find(|r| r.is_self && !r.deleted)
            .map(|r| r.id)
        {
            self.add_normalized_exemplar(existing, normed, recording_id, now_ms)?;
            return Ok(existing);
        }
        // Revive a tombstoned self anchor in place. Right-to-erasure on
        // one's own "You" anchor must be reversible (re-consent); a dead
        // anchor would otherwise permanently disable mic-bleed
        // suppression, the core guarantee this module exists to provide.
        if let Some(r) = self.records.iter_mut().find(|r| r.is_self) {
            r.deleted = false;
            r.deleted_at_ms = None;
            r.exemplars = vec![normed];
            r.exemplar_recording_ids = vec![recording_id];
            r.negative_exemplars.clear();
            r.consent_granted_at_ms = Some(now_ms);
            r.updated_at_ms = now_ms;
            let id = r.id;
            self.version += 1;
            return Ok(id);
        }
        let id = Uuid::new_v4();
        self.records.push(NamedVoiceRecord {
            id,
            display_name: "You".to_string(),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            exemplars: vec![normed],
            exemplar_recording_ids: vec![recording_id],
            negative_exemplars: Vec::new(),
            source_device_id: device_id,
            consent_granted_at_ms: Some(now_ms),
            is_self: true,
            deleted: false,
            deleted_at_ms: None,
        });
        self.version += 1;
        Ok(id)
    }

    /// Add an exemplar to an existing identity, evicting the oldest when
    /// the per-identity cap is reached. Errors on unknown id or
    /// wrong-dimension embedding.
    pub fn add_exemplar(
        &mut self,
        id: Uuid,
        embedding: &[f32],
        recording_id: Uuid,
        now_ms: i64,
    ) -> Result<()> {
        validate_dim(embedding)?;
        if self.record(id).is_some_and(|r| r.is_self) {
            return Err(AttuneError::Storage(format!(
                "speaker {id}: cannot add exemplars to the self anchor; use anchor_self"
            )));
        }
        let normed = l2_normalize(embedding);
        self.add_normalized_exemplar(id, normed, recording_id, now_ms)
    }

    fn add_normalized_exemplar(
        &mut self,
        id: Uuid,
        normed: Vec<f32>,
        recording_id: Uuid,
        now_ms: i64,
    ) -> Result<()> {
        {
            let r = self
                .record_mut(id)
                .ok_or_else(|| AttuneError::Storage(format!("speaker {id}: no such identity")))?;
            if r.deleted {
                return Err(AttuneError::Storage(format!(
                    "speaker {id}: identity was deleted"
                )));
            }
            // The two parallel vectors are kept equal-length by every
            // mutator (and re-aligned by `sanitize` on load), so evict
            // both in lockstep.
            if r.exemplars.len() >= MAX_EXEMPLARS {
                r.exemplars.remove(0);
                r.exemplar_recording_ids.remove(0);
            }
            r.exemplars.push(normed);
            r.exemplar_recording_ids.push(recording_id);
            r.updated_at_ms = now_ms;
        }
        self.version += 1;
        Ok(())
    }

    /// Record a "not this person" vote for an identity (used when the
    /// user rejects an auto/confirm match). Capped at
    /// [`MAX_NEGATIVE_EXEMPLARS`], oldest evicted.
    pub fn add_negative_exemplar(
        &mut self,
        id: Uuid,
        embedding: &[f32],
        now_ms: i64,
    ) -> Result<()> {
        validate_dim(embedding)?;
        let normed = l2_normalize(embedding);
        let r = self
            .record_mut(id)
            .ok_or_else(|| AttuneError::Storage(format!("speaker {id}: no such identity")))?;
        // Never re-introduce biometric data into a tombstoned (erased)
        // record — mirrors the guard in add_normalized_exemplar.
        if r.deleted {
            return Err(AttuneError::Storage(format!(
                "speaker {id}: identity was deleted"
            )));
        }
        if r.negative_exemplars.len() >= MAX_NEGATIVE_EXEMPLARS {
            r.negative_exemplars.remove(0);
        }
        r.negative_exemplars.push(normed);
        r.updated_at_ms = now_ms;
        self.version += 1;
        Ok(())
    }

    /// Right-to-erasure: tombstone an identity and purge its biometric
    /// data (positive + negative exemplars). The tombstone is retained so
    /// the deletion can propagate to other devices once sync lands.
    /// Returns true if an identity was found and forgotten.
    pub fn forget(&mut self, id: Uuid, now_ms: i64) -> bool {
        if let Some(r) = self.record_mut(id) {
            if r.deleted {
                return false;
            }
            r.exemplars.clear();
            r.exemplar_recording_ids.clear();
            r.negative_exemplars.clear();
            // Right-to-erasure also drops the human-readable identifier
            // and consent timestamp; only the opaque id + deletion time
            // survive so the tombstone can still propagate the deletion to
            // other devices once sync lands.
            r.display_name.clear();
            r.consent_granted_at_ms = None;
            r.deleted = true;
            r.deleted_at_ms = Some(now_ms);
            r.updated_at_ms = now_ms;
            self.version += 1;
            true
        } else {
            false
        }
    }

    /// Load a registry from an encrypted file. A missing file yields an
    /// empty registry (first run). A present-but-unreadable file (bad
    /// passphrase, corruption) errors rather than silently resetting, so
    /// a key mistake never destroys the user's named voices.
    pub fn load(path: &Path, passphrase: &[u8]) -> Result<Self> {
        let envelope = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => {
                return Err(AttuneError::Storage(format!(
                    "speaker registry read {}: {e}",
                    path.display()
                )))
            }
        };
        let plaintext = crate::encryption::open(passphrase, &envelope)?;
        let mut registry: SpeakerRegistry = serde_json::from_slice(&plaintext)
            .map_err(|e| AttuneError::Storage(format!("speaker registry deserialize: {e}")))?;
        registry.sanitize();
        Ok(registry)
    }

    /// Defensive repair after loading: keep each record's two parallel
    /// vectors (`exemplars` / `exemplar_recording_ids`) in lockstep and
    /// drop any vector whose dimension isn't [`EMBED_DIM`], so a malformed
    /// blob (corruption, or a future cross-device merge) can't desync
    /// provenance or poison the cosine math.
    fn sanitize(&mut self) {
        for r in &mut self.records {
            let n = r.exemplars.len().min(r.exemplar_recording_ids.len());
            r.exemplars.truncate(n);
            r.exemplar_recording_ids.truncate(n);
            let mut i = 0;
            while i < r.exemplars.len() {
                if r.exemplars[i].len() == EMBED_DIM {
                    i += 1;
                } else {
                    r.exemplars.remove(i);
                    r.exemplar_recording_ids.remove(i);
                }
            }
            r.negative_exemplars.retain(|e| e.len() == EMBED_DIM);
        }
    }

    /// Encrypt and persist the registry via the shared atomic-write
    /// helper (temp + fsync + rename, parent dirs created), so a crash
    /// mid-write never truncates the existing registry and no dangling
    /// temp is left behind.
    ///
    // TODO(GET-189 P4): the JSON `Vec<Vec<f32>>` blob is ~1 MB at
    // capacity; switch to a compact (quantized / bincode) encoding before
    // cloud sync, where blob size and bandwidth matter.
    pub fn save(&self, path: &Path, passphrase: &[u8]) -> Result<()> {
        let plaintext = serde_json::to_vec(self)
            .map_err(|e| AttuneError::Storage(format!("speaker registry serialize: {e}")))?;
        let envelope = crate::encryption::seal(passphrase, &plaintext)?;
        crate::storage::atomic_write::atomic_write(path, &envelope)
    }
}

fn validate_dim(embedding: &[f32]) -> Result<()> {
    if embedding.len() != EMBED_DIM {
        return Err(AttuneError::Storage(format!(
            "embedding must be {EMBED_DIM}-d, got {}",
            embedding.len()
        )));
    }
    Ok(())
}

/// L2-normalize a vector. A zero vector is returned unchanged (its cosine
/// with anything is 0, which the thresholds treat as "no match").
fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

/// Cosine similarity between two already-L2-normalized vectors (a dot
/// product). Returns 0.0 on length mismatch.
fn cosine_normalized(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Max cosine similarity of `query` (normalized) to any vector in `set`
/// (each normalized). Returns -1.0 for an empty set so it loses every
/// comparison.
fn max_cosine(query: &[f32], set: &[Vec<f32>]) -> f32 {
    set.iter()
        .map(|e| cosine_normalized(query, e))
        .fold(-1.0_f32, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const REC: Uuid = Uuid::nil();
    const DEV: Uuid = Uuid::nil();

    /// A deterministic normalized embedding pointing mostly along axis
    /// `axis`, with a small perturbation `jitter` along `axis+1`. Two
    /// embeddings on the same axis have cosine ≈ 1; different axes ≈ 0.
    fn emb(axis: usize, jitter: f32) -> Vec<f32> {
        let mut v = vec![0.0f32; EMBED_DIM];
        v[axis % EMBED_DIM] = 1.0;
        v[(axis + 1) % EMBED_DIM] = jitter;
        l2_normalize(&v)
    }

    fn name_new(reg: &mut SpeakerRegistry, name: &str, e: &[f32]) -> Uuid {
        reg.name_speaker(
            NameTarget::New {
                display_name: name.to_string(),
            },
            e,
            REC,
            DEV,
            Some(0),
            0,
        )
        .unwrap()
    }

    #[test]
    fn cosine_and_normalize_behave() {
        let a = emb(3, 0.0);
        let b = emb(3, 0.0);
        let c = emb(50, 0.0);
        assert!((cosine_normalized(&a, &b) - 1.0).abs() < 1e-5);
        assert!(cosine_normalized(&a, &c).abs() < 1e-5);
    }

    #[test]
    fn empty_registry_matches_new() {
        let reg = SpeakerRegistry::new();
        assert_eq!(reg.match_embedding(&emb(1, 0.0)), MatchOutcome::New);
        assert!(reg.is_empty());
    }

    #[test]
    fn wrong_dimension_is_new_not_panic() {
        let reg = SpeakerRegistry::new();
        assert_eq!(reg.match_embedding(&[0.1, 0.2, 0.3]), MatchOutcome::New);
    }

    #[test]
    fn single_exemplar_confirms_but_never_auto_names() {
        // Same voice, cosine ≈ 1.0 — but only one exemplar, so the guard
        // forces Confirm, never AutoName.
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        match reg.match_embedding(&emb(7, 0.0)) {
            MatchOutcome::Confirm { id: got, score } => {
                assert_eq!(got, id);
                assert!(score > AUTO_NAME_THRESHOLD); // high score, still only confirm
            }
            other => panic!("expected Confirm, got {other:?}"),
        }
    }

    #[test]
    fn three_exemplars_unlock_auto_name() {
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        reg.add_exemplar(id, &emb(7, 0.02), REC, 1).unwrap();
        reg.add_exemplar(id, &emb(7, -0.02), REC, 2).unwrap();
        match reg.match_embedding(&emb(7, 0.0)) {
            MatchOutcome::AutoName { id: got, .. } => assert_eq!(got, id),
            other => panic!("expected AutoName, got {other:?}"),
        }
    }

    #[test]
    fn distant_voice_is_new() {
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        for k in 0..3 {
            reg.add_exemplar(id, &emb(7, 0.01 * k as f32), REC, k as i64)
                .unwrap();
        }
        // An orthogonal voice.
        assert_eq!(reg.match_embedding(&emb(120, 0.0)), MatchOutcome::New);
    }

    #[test]
    fn negative_exemplar_blocks_a_match() {
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        for k in 0..3 {
            reg.add_exemplar(id, &emb(7, 0.01 * k as f32), REC, k as i64)
                .unwrap();
        }
        // Before the negative: a near-axis-7 query matches.
        assert!(matches!(
            reg.match_embedding(&emb(7, 0.05)),
            MatchOutcome::AutoName { .. } | MatchOutcome::Confirm { .. }
        ));
        // Record that this exact query is NOT Fatih.
        reg.add_negative_exemplar(id, &emb(7, 0.05), 10).unwrap();
        // Now the same query is closer to the negative than the positives
        // → the identity is skipped → New.
        assert_eq!(reg.match_embedding(&emb(7, 0.05)), MatchOutcome::New);
    }

    #[test]
    fn self_anchor_suppresses_user_bleed() {
        let mut reg = SpeakerRegistry::new();
        reg.anchor_self(&emb(2, 0.0), REC, DEV, 0).unwrap();
        match reg.match_embedding(&emb(2, 0.01)) {
            MatchOutcome::SelfUser { score } => assert!(score >= SELF_MATCH_THRESHOLD),
            other => panic!("expected SelfUser, got {other:?}"),
        }
    }

    #[test]
    fn self_anchor_does_not_swallow_other_speakers() {
        let mut reg = SpeakerRegistry::new();
        reg.anchor_self(&emb(2, 0.0), REC, DEV, 0).unwrap();
        let other = name_new(&mut reg, "Emira", &emb(80, 0.0));
        reg.add_exemplar(other, &emb(80, 0.01), REC, 1).unwrap();
        reg.add_exemplar(other, &emb(80, -0.01), REC, 2).unwrap();
        match reg.match_embedding(&emb(80, 0.0)) {
            MatchOutcome::AutoName { id, .. } => assert_eq!(id, other),
            other => panic!("expected AutoName for Emira, got {other:?}"),
        }
    }

    #[test]
    fn exemplar_set_is_capped_and_evicts_oldest() {
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        for k in 0..(MAX_EXEMPLARS + 5) {
            reg.add_exemplar(id, &emb(7, 0.001 * k as f32), REC, k as i64)
                .unwrap();
        }
        assert_eq!(reg.record(id).unwrap().exemplars.len(), MAX_EXEMPLARS);
        assert_eq!(
            reg.record(id).unwrap().exemplar_recording_ids.len(),
            MAX_EXEMPLARS
        );
    }

    #[test]
    fn forget_purges_biometrics_and_stops_matching() {
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        reg.add_exemplar(id, &emb(7, 0.01), REC, 1).unwrap();
        reg.add_exemplar(id, &emb(7, -0.01), REC, 2).unwrap();
        assert!(reg.forget(id, 99));
        let r = reg.record(id).unwrap();
        assert!(r.deleted);
        assert_eq!(r.deleted_at_ms, Some(99));
        assert!(r.exemplars.is_empty(), "biometric data must be purged");
        assert!(r.negative_exemplars.is_empty());
        // No longer matches anything.
        assert_eq!(reg.match_embedding(&emb(7, 0.0)), MatchOutcome::New);
        // Idempotent.
        assert!(!reg.forget(id, 100));
    }

    #[test]
    fn encrypted_round_trip_preserves_registry() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("registry.enc");
        let pass = b"correct horse battery staple";

        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        reg.add_exemplar(id, &emb(7, 0.02), REC, 1).unwrap();
        reg.anchor_self(&emb(2, 0.0), REC, DEV, 0).unwrap();
        reg.save(&path, pass).unwrap();

        let loaded = SpeakerRegistry::load(&path, pass).unwrap();
        assert_eq!(loaded.records, reg.records);
        assert_eq!(loaded.version, reg.version);
        // The on-disk bytes are ciphertext, not the name in plaintext.
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(5).any(|w| w == b"Fatih"));
    }

    #[test]
    fn load_missing_file_is_empty_registry() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nope.enc");
        let reg = SpeakerRegistry::load(&path, b"pw").unwrap();
        assert!(reg.is_empty());
    }

    #[test]
    fn wrong_passphrase_errors_rather_than_resets() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("registry.enc");
        let mut reg = SpeakerRegistry::new();
        name_new(&mut reg, "Fatih", &emb(7, 0.0));
        reg.save(&path, b"right").unwrap();
        assert!(SpeakerRegistry::load(&path, b"wrong").is_err());
    }

    #[test]
    fn forgotten_self_anchor_can_be_re_anchored() {
        // Regression: forget(self) must not permanently brick anchor_self
        // and silently disable mic-bleed suppression.
        let mut reg = SpeakerRegistry::new();
        let self_id = reg.anchor_self(&emb(2, 0.0), REC, DEV, 0).unwrap();
        assert!(reg.forget(self_id, 1));
        // After erasure, own voice no longer self-matches.
        assert_eq!(reg.match_embedding(&emb(2, 0.0)), MatchOutcome::New);
        // Re-anchoring must succeed (revive in place) and restore
        // suppression.
        let re_id = reg.anchor_self(&emb(2, 0.01), REC, DEV, 2).unwrap();
        assert_eq!(re_id, self_id, "should revive the same anchor in place");
        assert!(matches!(
            reg.match_embedding(&emb(2, 0.0)),
            MatchOutcome::SelfUser { .. }
        ));
        // Only one self record exists.
        assert_eq!(reg.records.iter().filter(|r| r.is_self).count(), 1);
    }

    #[test]
    fn self_anchor_cannot_be_named_via_public_paths() {
        let mut reg = SpeakerRegistry::new();
        let self_id = reg.anchor_self(&emb(2, 0.0), REC, DEV, 0).unwrap();
        // name_speaker(Existing{self}) is rejected.
        assert!(reg
            .name_speaker(
                NameTarget::Existing { id: self_id },
                &emb(80, 0.0),
                REC,
                DEV,
                Some(0),
                1,
            )
            .is_err());
        // add_exemplar(self) is rejected.
        assert!(reg.add_exemplar(self_id, &emb(80, 0.0), REC, 1).is_err());
        // The anchor still holds only its mic-derived exemplar.
        assert_eq!(reg.record(self_id).unwrap().exemplars.len(), 1);
        // A stranger's voice is NOT suppressed as the user.
        assert_eq!(reg.match_embedding(&emb(80, 0.0)), MatchOutcome::New);
    }

    #[test]
    fn add_negative_on_deleted_record_is_rejected() {
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        assert!(reg.forget(id, 1));
        // Must not re-introduce biometric data into an erased record.
        assert!(reg.add_negative_exemplar(id, &emb(7, 0.0), 2).is_err());
        assert!(reg.record(id).unwrap().negative_exemplars.is_empty());
    }

    #[test]
    fn forget_scrubs_display_name_and_consent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("registry.enc");
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        reg.forget(id, 1);
        let r = reg.record(id).unwrap();
        assert!(r.display_name.is_empty(), "name must be erased");
        assert_eq!(r.consent_granted_at_ms, None);
        // And the scrubbed name is not in the persisted blob.
        reg.save(&path, b"pw").unwrap();
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(5).any(|w| w == b"Fatih"));
    }

    #[test]
    fn load_repairs_a_desynced_record() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("registry.enc");
        // Hand-build a structurally-malformed record: more exemplars than
        // recording-ids, plus a wrong-dimension exemplar.
        let mut reg = SpeakerRegistry::new();
        reg.records.push(NamedVoiceRecord {
            id: Uuid::new_v4(),
            display_name: "Broken".into(),
            created_at_ms: 0,
            updated_at_ms: 0,
            exemplars: vec![emb(7, 0.0), emb(7, 0.01), vec![0.1; 3]],
            exemplar_recording_ids: vec![REC], // desynced: 1 id for 3 exemplars
            negative_exemplars: vec![vec![0.2; 7]], // wrong dim
            source_device_id: DEV,
            consent_granted_at_ms: None,
            is_self: false,
            deleted: false,
            deleted_at_ms: None,
        });
        reg.save(&path, b"pw").unwrap();
        let loaded = SpeakerRegistry::load(&path, b"pw").unwrap();
        let r = &loaded.records[0];
        // Aligned to the shorter length, wrong-dim dropped, ids in lockstep.
        assert_eq!(r.exemplars.len(), r.exemplar_recording_ids.len());
        assert!(r.exemplars.iter().all(|e| e.len() == EMBED_DIM));
        assert!(r.negative_exemplars.is_empty());
    }
}
