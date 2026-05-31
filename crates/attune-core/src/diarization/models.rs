//! Diarization model registry, on-disk layout, and downloader.
//!
//! Two models drive the pipeline:
//!
//! 1. **Segmentation** — pyannote-segmentation-3.0 ONNX (MIT). Powerset
//!    7-class model that detects up to three concurrent speakers per
//!    10 s window. ~6 MB on disk.
//! 2. **Embedding** — WeSpeaker ResNet34-LM (Apache-2.0). 256-dim
//!    speaker embeddings, 0.72 % EER on VoxCeleb1-O. ~26 MB on disk.
//!
//! silero-vad is intentionally NOT here — it ships embedded inside the
//! `voice_activity_detector` crate (~1.6 MB inside the binary) and is
//! already plumbed by `audio::vad_filter`.
//!
//! Models live in `~/Library/Application Support/Attune/models/diarization/`
//! on macOS. The downloader writes to `.part` and atomic-renames into
//! place so a mid-download crash leaves a partial file rather than a
//! truncated model the runtime would fail to load on next launch — the
//! same pattern `transcription::models` uses for the Whisper GGMLs.
//!
//! ## sha256 hashes
//!
//! `expected_sha256` is `None` until P0b records the verified hash for
//! the published model file. Until then `download` skips verification.
//! Once the first model is downloaded and verified manually, the hash
//! is hardcoded here and verification becomes mandatory.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use ts_rs::TS;

use crate::error::{AttuneError, Result};

/// Connection timeout for the download. Body timeout is intentionally
/// absent — slow links can take many minutes for the 26 MB embedding
/// model.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// The two ONNX models the diarization pipeline depends on. Variants
/// are stable identifiers used by `Settings.diarization` and the Tauri
/// command surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "kebab-case")]
pub enum DiarizationModel {
    /// pyannote-segmentation-3.0, ONNX export. MIT.
    Segmentation,
    /// WeSpeaker ResNet34-LM (VoxCeleb1-O EER 0.72 %), ONNX. Apache-2.0.
    EmbeddingResnet34Lm,
}

impl DiarizationModel {
    pub const ALL: &'static [Self] = &[Self::Segmentation, Self::EmbeddingResnet34Lm];

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "segmentation" => Self::Segmentation,
            "embedding-resnet34-lm" => Self::EmbeddingResnet34Lm,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Segmentation => "segmentation",
            Self::EmbeddingResnet34Lm => "embedding-resnet34-lm",
        }
    }

    /// Human-readable label for the download-progress UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Segmentation => "Speaker segmentation",
            Self::EmbeddingResnet34Lm => "Speaker embedding",
        }
    }

    /// File the model is written to on disk. Stable across versions of
    /// `attune-core` so a user's existing download survives upgrades.
    fn filename(self) -> &'static str {
        match self {
            Self::Segmentation => "pyannote_segmentation_3_0.onnx",
            Self::EmbeddingResnet34Lm => "wespeaker_en_voxceleb_resnet34_LM.onnx",
        }
    }

    /// Upstream URL. Points at sherpa-onnx's release assets where they
    /// exist (those are the exact files sherpa-onnx's runtime expects);
    /// HuggingFace for the segmentation model since sherpa-onnx
    /// distributes that one as a tar bundle rather than a single ONNX.
    fn url(self) -> &'static str {
        match self {
            // onnx-community publishes a single-file ONNX export. sherpa-onnx's
            // release bundle for the same model also works once extracted, but
            // we prefer the single file to avoid pulling tar/bzip2 deps in.
            Self::Segmentation => {
                "https://huggingface.co/onnx-community/pyannote-segmentation-3.0/\
                 resolve/main/onnx/model.onnx"
            }
            // sherpa-onnx mirrors WeSpeaker's official ONNX. Same SHA-256
            // as the HuggingFace original.
            Self::EmbeddingResnet34Lm => {
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/\
                 speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx"
            }
        }
    }

    /// Approximate on-disk bytes, used to drive the progress UI when
    /// the server omits Content-Length.
    pub fn approx_bytes(self) -> u64 {
        match self {
            Self::Segmentation => 6 * 1024 * 1024,
            Self::EmbeddingResnet34Lm => 26 * 1024 * 1024,
        }
    }

    /// Hex-encoded SHA-256 of the canonical published file. `None`
    /// until P0b records the verified value. When `Some`, [`download`]
    /// rejects mismatches.
    fn expected_sha256(self) -> Option<&'static str> {
        match self {
            Self::Segmentation => None,
            Self::EmbeddingResnet34Lm => None,
        }
    }
}

/// On-disk status of a single model.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct DiarizationModelStatus {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    pub present: bool,
    pub bytes_on_disk: Option<u64>,
    pub approx_total_bytes: u64,
}

/// Progress callback shape — same fields as
/// `transcription::models::DownloadProgress` for callsite parity.
#[derive(Debug, Clone, Copy)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Knows where the diarization model directory lives and how to fetch
/// each model.
pub struct DiarizationModelStore {
    root: PathBuf,
}

impl DiarizationModelStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Default store rooted at the platform's standard config
    /// directory. Sits *alongside* the whisper store (under the same
    /// `models/` parent) so a future cleanup tool can sweep both at
    /// once.
    pub fn default_location() -> Self {
        Self::new(default_models_dir())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_for(&self, model: DiarizationModel) -> PathBuf {
        self.root.join(model.filename())
    }

    /// Single `metadata()` call per model. No bytes read.
    pub fn status(&self, model: DiarizationModel) -> DiarizationModelStatus {
        let path = self.path_for(model);
        let meta = fs::metadata(&path).ok();
        DiarizationModelStatus {
            id: model.id().to_string(),
            label: model.label().to_string(),
            path,
            present: meta.is_some(),
            bytes_on_disk: meta.as_ref().map(|m| m.len()),
            approx_total_bytes: model.approx_bytes(),
        }
    }

    /// Status for every model the runtime needs. The runtime is ready
    /// when every entry's `present` is true.
    pub fn status_all(&self) -> Vec<DiarizationModelStatus> {
        DiarizationModel::ALL
            .iter()
            .map(|m| self.status(*m))
            .collect()
    }

    /// True iff every required model is on disk.
    pub fn is_ready(&self) -> bool {
        self.status_all().iter().all(|s| s.present)
    }

    /// Download `model` to disk, streaming the body and reporting
    /// progress. Writes to `<filename>.part` and atomic-renames into
    /// place so a mid-download crash leaves a partial file (which
    /// [`clean_partials`] can sweep) rather than a truncated ONNX the
    /// runtime would fail to load.
    pub fn download<F: FnMut(DownloadProgress)>(
        &self,
        model: DiarizationModel,
        mut on_progress: F,
    ) -> Result<PathBuf> {
        fs::create_dir_all(&self.root).map_err(|e| {
            AttuneError::Storage(format!(
                "could not create diarization model dir {}: {e}",
                self.root.display()
            ))
        })?;

        let target = self.path_for(model);
        let tmp = target.with_extension("onnx.part");
        info!(
            model = model.id(),
            url = model.url(),
            target = %target.display(),
            "downloading diarization model",
        );

        let client = reqwest::blocking::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|e| {
                AttuneError::Storage(format!("could not build diarization download client: {e}"))
            })?;

        let mut response = client
            .get(model.url())
            .send()
            .map_err(|e| AttuneError::Storage(format!("model download failed: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(AttuneError::Storage(format!(
                "diarization model download returned {status} for {}",
                model.url()
            )));
        }
        let total = response.content_length();

        let mut file = fs::File::create(&tmp).map_err(|e| {
            AttuneError::Storage(format!(
                "could not open download temp file {}: {e}",
                tmp.display()
            ))
        })?;

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut downloaded: u64 = 0;
        loop {
            let n = response
                .read(&mut buffer)
                .map_err(|e| AttuneError::Storage(format!("download read error: {e}")))?;
            if n == 0 {
                break;
            }
            use std::io::Write;
            file.write_all(&buffer[..n])
                .map_err(|e| AttuneError::Storage(format!("download write error: {e}")))?;
            hasher.update(&buffer[..n]);
            downloaded += n as u64;
            on_progress(DownloadProgress { downloaded, total });
        }

        file.sync_all()
            .map_err(|e| AttuneError::Storage(format!("download sync error: {e}")))?;
        drop(file);

        let got = hex::encode(hasher.finalize());
        if let Some(expected) = model.expected_sha256() {
            if !got.eq_ignore_ascii_case(expected) {
                let _ = fs::remove_file(&tmp);
                return Err(AttuneError::Storage(format!(
                    "sha256 mismatch for {} (got {got}, expected {expected})",
                    model.id()
                )));
            }
            debug!(
                model = model.id(),
                sha256 = got,
                "diarization model verified"
            );
        } else {
            // P0: hashes not yet recorded. Log the value so the
            // downloader operator can paste it into `expected_sha256`.
            info!(
                model = model.id(),
                sha256 = got,
                "diarization model downloaded; sha256 unverified — paste into expected_sha256 to enable verification",
            );
        }

        fs::rename(&tmp, &target).map_err(|e| {
            AttuneError::Storage(format!(
                "could not finalize diarization model {}: {e}",
                target.display()
            ))
        })?;

        info!(
            model = model.id(),
            target = %target.display(),
            bytes = downloaded,
            "diarization model download complete",
        );
        Ok(target)
    }

    /// Best-effort sweep of stale `.onnx.part` files from prior crashed
    /// downloads. Logged, never returned as an error.
    pub fn clean_partials(&self) {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("part") {
                if let Err(e) = fs::remove_file(&path) {
                    warn!(path = %path.display(), error = %e, "could not remove stale diarization .part file");
                } else {
                    debug!(path = %path.display(), "removed stale diarization .part file");
                }
            }
        }
    }
}

fn default_models_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Application Support")
            .join("Attune")
            .join("models")
            .join("diarization")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".local")
            .join("share")
            .join("attune")
            .join("models")
            .join("diarization")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_round_trips() {
        for m in DiarizationModel::ALL {
            assert_eq!(DiarizationModel::from_id(m.id()), Some(*m));
        }
    }

    #[test]
    fn unknown_id_returns_none() {
        assert_eq!(DiarizationModel::from_id("speaker-x-mega"), None);
        assert_eq!(DiarizationModel::from_id(""), None);
    }

    #[test]
    fn status_reports_absent_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = DiarizationModelStore::new(dir.path());
        let status = store.status(DiarizationModel::Segmentation);
        assert!(!status.present);
        assert_eq!(status.bytes_on_disk, None);
        assert_eq!(status.id, "segmentation");
        assert!(!store.is_ready());
    }

    #[test]
    fn status_reports_present_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = DiarizationModelStore::new(dir.path());
        let path = store.path_for(DiarizationModel::EmbeddingResnet34Lm);
        let payload: &[u8] = b"placeholder ONNX bytes";
        std::fs::write(&path, payload).unwrap();
        let status = store.status(DiarizationModel::EmbeddingResnet34Lm);
        assert!(status.present);
        assert_eq!(status.bytes_on_disk, Some(payload.len() as u64));
    }

    #[test]
    fn is_ready_only_when_all_models_present() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = DiarizationModelStore::new(dir.path());
        assert!(!store.is_ready());
        std::fs::write(store.path_for(DiarizationModel::Segmentation), b"x").unwrap();
        assert!(!store.is_ready(), "only one of two models present");
        std::fs::write(store.path_for(DiarizationModel::EmbeddingResnet34Lm), b"x").unwrap();
        assert!(store.is_ready(), "both models present");
    }

    #[test]
    fn url_points_at_canonical_publishers() {
        // Sanity check: if either URL changes upstream we want a
        // compile-time literal change, not a silent breakage.
        assert!(DiarizationModel::Segmentation
            .url()
            .contains("huggingface.co"));
        assert!(DiarizationModel::Segmentation
            .url()
            .contains("pyannote-segmentation-3.0"));
        assert!(DiarizationModel::EmbeddingResnet34Lm
            .url()
            .contains("k2-fsa/sherpa-onnx"));
        assert!(DiarizationModel::EmbeddingResnet34Lm
            .url()
            .contains("voxceleb_resnet34_LM"));
    }
}
