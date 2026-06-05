use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use ts_rs::TS;

use crate::error::{FolioError, Result};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "kebab-case")]
pub enum DiarizationModel {
    Segmentation,

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

    pub fn label(self) -> &'static str {
        match self {
            Self::Segmentation => "Speaker segmentation",
            Self::EmbeddingResnet34Lm => "Speaker embedding",
        }
    }

    fn filename(self) -> &'static str {
        match self {
            Self::Segmentation => "pyannote_segmentation_3_0.onnx",
            Self::EmbeddingResnet34Lm => "wespeaker_en_voxceleb_resnet34_LM.onnx",
        }
    }

    fn url(self) -> &'static str {
        match self {
            Self::Segmentation => {
                "https://huggingface.co/csukuangfj/\
                 sherpa-onnx-pyannote-segmentation-3-0/resolve/main/model.onnx"
            }

            Self::EmbeddingResnet34Lm => {
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/\
                 speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx"
            }
        }
    }

    pub fn approx_bytes(self) -> u64 {
        match self {
            Self::Segmentation => 5_992_913,
            Self::EmbeddingResnet34Lm => 26_530_550,
        }
    }

    fn expected_sha256(self) -> Option<&'static str> {
        match self {
            Self::Segmentation => {
                Some("220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079")
            }
            Self::EmbeddingResnet34Lm => {
                Some("e9848563da86f263117134dfd7ad63c92355b37de492b55e325400c9d9c39012")
            }
        }
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

pub struct DiarizationModelStore {
    root: PathBuf,
}

impl DiarizationModelStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_location() -> Self {
        Self::new(default_models_dir())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_for(&self, model: DiarizationModel) -> PathBuf {
        self.root.join(model.filename())
    }

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

    pub fn status_all(&self) -> Vec<DiarizationModelStatus> {
        DiarizationModel::ALL
            .iter()
            .map(|m| self.status(*m))
            .collect()
    }

    pub fn is_ready(&self) -> bool {
        self.status_all().iter().all(|s| s.present)
    }

    pub fn download<F: FnMut(DownloadProgress)>(
        &self,
        model: DiarizationModel,
        mut on_progress: F,
    ) -> Result<PathBuf> {
        fs::create_dir_all(&self.root).map_err(|e| {
            FolioError::Storage(format!(
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
                FolioError::Storage(format!("could not build diarization download client: {e}"))
            })?;

        let host = crate::cloud_guard::host_of(model.url()).unwrap_or_default();
        crate::cloud_guard::ensure_allowed(host).map_err(|e| FolioError::Storage(e.to_string()))?;

        let mut response = client
            .get(model.url())
            .send()
            .map_err(|e| FolioError::Storage(format!("model download failed: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(FolioError::Storage(format!(
                "diarization model download returned {status} for {}",
                model.url()
            )));
        }
        let total = response.content_length();

        let mut file = fs::File::create(&tmp).map_err(|e| {
            FolioError::Storage(format!(
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
                .map_err(|e| FolioError::Storage(format!("download read error: {e}")))?;
            if n == 0 {
                break;
            }
            use std::io::Write;
            file.write_all(&buffer[..n])
                .map_err(|e| FolioError::Storage(format!("download write error: {e}")))?;
            hasher.update(&buffer[..n]);
            downloaded += n as u64;
            on_progress(DownloadProgress { downloaded, total });
        }

        file.sync_all()
            .map_err(|e| FolioError::Storage(format!("download sync error: {e}")))?;
        drop(file);

        let got = hex::encode(hasher.finalize());
        if let Some(expected) = model.expected_sha256() {
            if !got.eq_ignore_ascii_case(expected) {
                let _ = fs::remove_file(&tmp);
                return Err(FolioError::Storage(format!(
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
            info!(
                model = model.id(),
                sha256 = got,
                "diarization model downloaded; sha256 unverified — paste into expected_sha256 to enable verification",
            );
        }

        fs::rename(&tmp, &target).map_err(|e| {
            FolioError::Storage(format!(
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
            .join("Folio")
            .join("models")
            .join("diarization")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".local")
            .join("share")
            .join("folio")
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
        assert!(DiarizationModel::Segmentation
            .url()
            .contains("huggingface.co"));
        assert!(DiarizationModel::Segmentation
            .url()
            .contains("sherpa-onnx-pyannote-segmentation-3-0"));
        assert!(
            !DiarizationModel::Segmentation
                .url()
                .contains("onnx-community"),
            "must not regress to the sherpa-incompatible onnx-community export"
        );
        assert!(DiarizationModel::EmbeddingResnet34Lm
            .url()
            .contains("k2-fsa/sherpa-onnx"));
        assert!(DiarizationModel::EmbeddingResnet34Lm
            .url()
            .contains("voxceleb_resnet34_LM"));
    }

    #[test]
    fn every_model_has_a_pinned_sha256() {
        for m in DiarizationModel::ALL {
            let hash = m.expected_sha256();
            assert!(hash.is_some(), "{} has no pinned sha256", m.id());
            assert_eq!(
                hash.unwrap().len(),
                64,
                "{} sha256 not 64 hex chars",
                m.id()
            );
        }
    }
}
