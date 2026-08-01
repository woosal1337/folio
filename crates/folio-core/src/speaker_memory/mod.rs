pub mod store;

use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{FolioError, Result};

pub use store::{default_registry_path, load_default, save_default};

pub const EMBED_DIM: usize = 256;

pub const MAX_EXEMPLARS: usize = 20;

pub const MAX_NEGATIVE_EXEMPLARS: usize = 10;

pub const AUTO_NAME_THRESHOLD: f32 = 0.82;

pub const CONFIRM_THRESHOLD: f32 = 0.60;

pub const MIN_EXEMPLARS_FOR_AUTONAME: usize = 3;

pub const SELF_MATCH_THRESHOLD: f32 = 0.90;

#[derive(Clone, Debug, PartialEq)]
pub enum MatchOutcome {
    SelfUser { score: f32 },

    AutoName { id: Uuid, score: f32 },

    Confirm { id: Uuid, score: f32 },

    New,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedVoiceRecord {
    pub id: Uuid,

    pub display_name: String,

    pub created_at_ms: i64,

    pub updated_at_ms: i64,

    pub exemplars: Vec<Vec<f32>>,

    pub exemplar_recording_ids: Vec<Uuid>,

    pub negative_exemplars: Vec<Vec<f32>>,

    pub source_device_id: Uuid,

    pub consent_granted_at_ms: Option<i64>,

    pub is_self: bool,

    pub deleted: bool,

    pub deleted_at_ms: Option<i64>,
}

impl NamedVoiceRecord {
    fn is_live(&self) -> bool {
        !self.deleted && !self.exemplars.is_empty()
    }

    fn positive_similarity(&self, query: &[f32]) -> f32 {
        max_cosine(query, &self.exemplars)
    }

    fn negative_similarity(&self, query: &[f32]) -> f32 {
        max_cosine(query, &self.negative_exemplars)
    }
}

#[derive(Clone, Debug)]
pub enum NameTarget {
    New { display_name: String },

    Existing { id: Uuid },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpeakerRegistry {
    #[serde(default)]
    pub version: u64,

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

    pub fn record(&self, id: Uuid) -> Option<&NamedVoiceRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    fn record_mut(&mut self, id: Uuid) -> Option<&mut NamedVoiceRecord> {
        self.records.iter_mut().find(|r| r.id == id)
    }

    fn self_anchor(&self) -> Option<&NamedVoiceRecord> {
        self.records.iter().find(|r| r.is_self && r.is_live())
    }

    pub fn match_embedding(&self, embedding: &[f32]) -> MatchOutcome {
        if embedding.len() != EMBED_DIM {
            return MatchOutcome::New;
        }
        let query = l2_normalize(embedding);

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
                if self.record(id).is_some_and(|r| r.is_self) {
                    return Err(FolioError::Storage(format!(
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

    pub fn anchor_self(
        &mut self,
        embedding: &[f32],
        recording_id: Uuid,
        device_id: Uuid,
        now_ms: i64,
    ) -> Result<Uuid> {
        validate_dim(embedding)?;
        let normed = l2_normalize(embedding);

        if let Some(existing) = self
            .records
            .iter()
            .find(|r| r.is_self && !r.deleted)
            .map(|r| r.id)
        {
            self.add_normalized_exemplar(existing, normed, recording_id, now_ms)?;
            return Ok(existing);
        }

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

    pub fn add_exemplar(
        &mut self,
        id: Uuid,
        embedding: &[f32],
        recording_id: Uuid,
        now_ms: i64,
    ) -> Result<()> {
        validate_dim(embedding)?;
        if self.record(id).is_some_and(|r| r.is_self) {
            return Err(FolioError::Storage(format!(
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
                .ok_or_else(|| FolioError::Storage(format!("speaker {id}: no such identity")))?;
            if r.deleted {
                return Err(FolioError::Storage(format!(
                    "speaker {id}: identity was deleted"
                )));
            }

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
            .ok_or_else(|| FolioError::Storage(format!("speaker {id}: no such identity")))?;

        if r.deleted {
            return Err(FolioError::Storage(format!(
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

    pub fn forget(&mut self, id: Uuid, now_ms: i64) -> bool {
        if let Some(r) = self.record_mut(id) {
            if r.deleted {
                return false;
            }
            r.exemplars.clear();
            r.exemplar_recording_ids.clear();
            r.negative_exemplars.clear();

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

    pub fn load(path: &Path, passphrase: &[u8]) -> Result<Self> {
        let envelope = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => {
                return Err(FolioError::Storage(format!(
                    "speaker registry read {}: {e}",
                    path.display()
                )))
            }
        };
        let plaintext = crate::encryption::open(passphrase, &envelope)?;
        let mut registry: SpeakerRegistry = serde_json::from_slice(&plaintext)
            .map_err(|e| FolioError::Storage(format!("speaker registry deserialize: {e}")))?;
        registry.sanitize();
        Ok(registry)
    }

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

    pub fn save(&self, path: &Path, passphrase: &[u8]) -> Result<()> {
        let plaintext = serde_json::to_vec(self)
            .map_err(|e| FolioError::Storage(format!("speaker registry serialize: {e}")))?;
        let envelope = crate::encryption::seal(passphrase, &plaintext)?;
        crate::storage::atomic_write::atomic_write(path, &envelope)
    }

    pub fn load_plain(path: &Path) -> Result<Self> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => {
                return Err(FolioError::Storage(format!(
                    "speaker registry read {}: {e}",
                    path.display()
                )))
            }
        };
        let mut registry: SpeakerRegistry = serde_json::from_slice(&bytes)
            .map_err(|e| FolioError::Storage(format!("speaker registry deserialize: {e}")))?;
        registry.sanitize();
        Ok(registry)
    }

    pub fn save_plain(&self, path: &Path) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| FolioError::Storage(format!("speaker registry serialize: {e}")))?;
        crate::storage::atomic_write::atomic_write(path, &bytes)
    }
}

fn validate_dim(embedding: &[f32]) -> Result<()> {
    if embedding.len() != EMBED_DIM {
        return Err(FolioError::Storage(format!(
            "embedding must be {EMBED_DIM}-d, got {}",
            embedding.len()
        )));
    }
    Ok(())
}

fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

fn cosine_normalized(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

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
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        match reg.match_embedding(&emb(7, 0.0)) {
            MatchOutcome::Confirm { id: got, score } => {
                assert_eq!(got, id);
                assert!(score > AUTO_NAME_THRESHOLD);
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

        assert!(matches!(
            reg.match_embedding(&emb(7, 0.05)),
            MatchOutcome::AutoName { .. } | MatchOutcome::Confirm { .. }
        ));

        reg.add_negative_exemplar(id, &emb(7, 0.05), 10).unwrap();

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

        assert_eq!(reg.match_embedding(&emb(7, 0.0)), MatchOutcome::New);

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

        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(5).any(|w| w == b"Fatih"));
    }

    #[test]
    fn plain_round_trip_preserves_registry_without_keychain() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("speaker-registry.json");

        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        reg.add_exemplar(id, &emb(7, 0.02), REC, 1).unwrap();
        reg.anchor_self(&emb(2, 0.0), REC, DEV, 0).unwrap();
        reg.save_plain(&path).unwrap();

        let loaded = SpeakerRegistry::load_plain(&path).unwrap();
        assert_eq!(loaded.records, reg.records);
        assert_eq!(loaded.version, reg.version);

        let raw = std::fs::read(&path).unwrap();
        assert!(raw.windows(5).any(|w| w == b"Fatih"));
    }

    #[test]
    fn load_plain_missing_file_is_empty_registry() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("absent.json");
        let reg = SpeakerRegistry::load_plain(&path).unwrap();
        assert!(reg.records.is_empty());
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
        let mut reg = SpeakerRegistry::new();
        let self_id = reg.anchor_self(&emb(2, 0.0), REC, DEV, 0).unwrap();
        assert!(reg.forget(self_id, 1));

        assert_eq!(reg.match_embedding(&emb(2, 0.0)), MatchOutcome::New);

        let re_id = reg.anchor_self(&emb(2, 0.01), REC, DEV, 2).unwrap();
        assert_eq!(re_id, self_id, "should revive the same anchor in place");
        assert!(matches!(
            reg.match_embedding(&emb(2, 0.0)),
            MatchOutcome::SelfUser { .. }
        ));

        assert_eq!(reg.records.iter().filter(|r| r.is_self).count(), 1);
    }

    #[test]
    fn self_anchor_cannot_be_named_via_public_paths() {
        let mut reg = SpeakerRegistry::new();
        let self_id = reg.anchor_self(&emb(2, 0.0), REC, DEV, 0).unwrap();

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

        assert!(reg.add_exemplar(self_id, &emb(80, 0.0), REC, 1).is_err());

        assert_eq!(reg.record(self_id).unwrap().exemplars.len(), 1);

        assert_eq!(reg.match_embedding(&emb(80, 0.0)), MatchOutcome::New);
    }

    #[test]
    fn add_negative_on_deleted_record_is_rejected() {
        let mut reg = SpeakerRegistry::new();
        let id = name_new(&mut reg, "Fatih", &emb(7, 0.0));
        assert!(reg.forget(id, 1));

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

        reg.save(&path, b"pw").unwrap();
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(5).any(|w| w == b"Fatih"));
    }

    #[test]
    fn load_repairs_a_desynced_record() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("registry.enc");

        let mut reg = SpeakerRegistry::new();
        reg.records.push(NamedVoiceRecord {
            id: Uuid::new_v4(),
            display_name: "Broken".into(),
            created_at_ms: 0,
            updated_at_ms: 0,
            exemplars: vec![emb(7, 0.0), emb(7, 0.01), vec![0.1; 3]],
            exemplar_recording_ids: vec![REC],
            negative_exemplars: vec![vec![0.2; 7]],
            source_device_id: DEV,
            consent_granted_at_ms: None,
            is_self: false,
            deleted: false,
            deleted_at_ms: None,
        });
        reg.save(&path, b"pw").unwrap();
        let loaded = SpeakerRegistry::load(&path, b"pw").unwrap();
        let r = &loaded.records[0];

        assert_eq!(r.exemplars.len(), r.exemplar_recording_ids.len());
        assert!(r.exemplars.iter().all(|e| e.len() == EMBED_DIM));
        assert!(r.negative_exemplars.is_empty());
    }
}
