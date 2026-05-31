//! sherpa-onnx diarization runtime. Stubbed in P0.
//!
//! Owns the loaded ONNX models + a clustering pool that gets fed
//! `SpeechSegment`s by the consumer (P1). The actual sherpa-onnx
//! pipeline wiring lands in P2 (mic anchor) and P3 (system clustering)
//! per the execution plan.
//!
//! For P0 the runtime exposes the surface its callers will use, but
//! every method returns `DiarizationError::NotImplemented`. This is
//! deliberate: it keeps the type signatures honest while the rest of
//! the pipeline lands, and lets the Tauri commands compile against a
//! real shape from day one.

use std::path::Path;

use thiserror::Error;

use crate::diarization::models::DiarizationModelStore;

/// Errors specific to the diarization runtime. Lives next to the
/// runtime so callers don't have to round-trip through the workspace
/// `AttuneError` for diarization-specific signals like "model not
/// downloaded yet".
#[derive(Debug, Error)]
pub enum DiarizationError {
    #[error(
        "required diarization models are not on disk; call DiarizationModelStore::download first"
    )]
    ModelsNotDownloaded,
    #[error("diarization pipeline is not yet implemented (P0 stub)")]
    NotImplemented,
    #[error("sherpa-onnx pipeline failed: {0}")]
    Runtime(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Loaded diarization pipeline. Cheap to clone (`Arc` of the sherpa
/// handles once it's real).
#[derive(Debug)]
pub struct DiarizationRuntime {
    // P0 placeholder. Real fields land in P2 / P3:
    //
    //   segmentation: sherpa_onnx::OfflineSpeakerSegmentation,
    //   embedding: sherpa_onnx::SpeakerEmbeddingExtractor,
    //   anchor: parking_lot::RwLock<Option<MicAnchor>>,
    //   pool: parking_lot::RwLock<ClusterPool>,
    //
    // Held as PhantomData of the store path for now so the type still
    // means something at the boundary.
    _store_root: std::path::PathBuf,
}

impl DiarizationRuntime {
    /// Load both ONNX models off disk. Returns
    /// [`DiarizationError::ModelsNotDownloaded`] when either model is
    /// missing — the caller is expected to drive the download flow
    /// (see `DiarizationModelStore::download`) before retrying.
    pub fn load(store: &DiarizationModelStore) -> Result<Self, DiarizationError> {
        if !store.is_ready() {
            return Err(DiarizationError::ModelsNotDownloaded);
        }
        // P0: the rest of the load is a no-op. The sherpa-onnx handles
        // are constructed in P2 once the consumer surface lands.
        Ok(Self {
            _store_root: store.root().to_path_buf(),
        })
    }

    /// Embed a mono-16k speech segment and return its 256-dim
    /// embedding. P0 stub.
    pub fn embed_segment(&self, _samples_16k: &[f32]) -> Result<Vec<f32>, DiarizationError> {
        Err(DiarizationError::NotImplemented)
    }

    /// Where the runtime expects models to live. Surfaced for the
    /// onboarding download UI so it can show "downloading to …".
    pub fn models_dir(&self) -> &Path {
        &self._store_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_errors_when_models_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = DiarizationModelStore::new(dir.path());
        let err = DiarizationRuntime::load(&store).expect_err("missing models should fail load");
        assert!(matches!(err, DiarizationError::ModelsNotDownloaded));
    }

    #[test]
    fn load_succeeds_when_both_models_present() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = DiarizationModelStore::new(dir.path());
        for m in crate::diarization::DiarizationModel::ALL {
            std::fs::write(store.path_for(*m), b"x").unwrap();
        }
        let rt = DiarizationRuntime::load(&store).expect("both present should load");
        assert_eq!(rt.models_dir(), store.root());
    }

    #[test]
    fn embed_is_not_implemented_in_p0() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = DiarizationModelStore::new(dir.path());
        for m in crate::diarization::DiarizationModel::ALL {
            std::fs::write(store.path_for(*m), b"x").unwrap();
        }
        let rt = DiarizationRuntime::load(&store).unwrap();
        let err = rt.embed_segment(&[0.0_f32; 16_000]).expect_err("P0 stub");
        assert!(matches!(err, DiarizationError::NotImplemented));
    }
}
