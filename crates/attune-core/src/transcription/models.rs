//! Local Whisper model registry, on-disk layout, and downloader.
//!
//! Models live under the platform's standard application-support
//! directory (`~/Library/Application Support/Attune/models/` on macOS),
//! one `ggml-<id>.bin` per variant. The set of supported variants is a
//! small, stable enum because we only surface ones we know whisper-rs
//! can load.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use ts_rs::TS;

use crate::error::{AttuneError, Result};

const HF_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
/// Connection timeout for the download. The model file itself can run
/// 30+ min on slow links — we only bound the dial, not the body.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// One of the GGML model variants whisper.cpp publishes. Sizes are the
/// approximate on-disk size of the .bin file.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "kebab-case")]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    #[serde(rename = "large-v3")]
    LargeV3,
}

impl WhisperModel {
    /// Parse the kebab-case string used in `Settings.local_whisper_model`.
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "tiny" => Self::Tiny,
            "base" => Self::Base,
            "small" => Self::Small,
            "medium" => Self::Medium,
            "large-v3" => Self::LargeV3,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Base => "base",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::LargeV3 => "large-v3",
        }
    }

    /// Approximate on-disk size, used to drive the download progress
    /// UI when the server doesn't return a Content-Length header (rare
    /// for HuggingFace, but possible behind some proxies).
    pub fn approx_bytes(self) -> u64 {
        match self {
            Self::Tiny => 75 * 1024 * 1024,
            Self::Base => 142 * 1024 * 1024,
            Self::Small => 466 * 1024 * 1024,
            Self::Medium => 1_500 * 1024 * 1024,
            Self::LargeV3 => 3_100 * 1024 * 1024,
        }
    }

    fn filename(self) -> String {
        format!("ggml-{}.bin", self.id())
    }

    fn url(self) -> String {
        format!("{HF_BASE_URL}/{}", self.filename())
    }
}

/// On-disk status of a model file. Carries the path even when the file
/// is missing so callers can show "would be downloaded to <path>".
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct WhisperModelStatus {
    pub id: String,
    pub path: PathBuf,
    pub present: bool,
    pub bytes_on_disk: Option<u64>,
    pub approx_total_bytes: u64,
}

/// Progress reported by [`WhisperModelStore::download`].
#[derive(Debug, Clone, Copy)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Knows where the model directory lives and how to download a model.
pub struct WhisperModelStore {
    root: PathBuf,
}

impl WhisperModelStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Default store rooted at the platform's standard config directory.
    pub fn default_location() -> Self {
        Self::new(default_models_dir())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_for(&self, model: WhisperModel) -> PathBuf {
        self.root.join(model.filename())
    }

    /// Report on-disk status without doing any IO besides a single
    /// `metadata()` call.
    pub fn status(&self, model: WhisperModel) -> WhisperModelStatus {
        let path = self.path_for(model);
        let meta = fs::metadata(&path).ok();
        let present = meta.is_some();
        WhisperModelStatus {
            id: model.id().to_string(),
            path,
            present,
            bytes_on_disk: meta.map(|m| m.len()),
            approx_total_bytes: model.approx_bytes(),
        }
    }

    /// Download `model` to disk, streaming the response body so the
    /// caller can show progress. Writes to a `.part` file and renames
    /// into place on completion — a crash mid-download leaves a partial
    /// file the caller can discover and clean up rather than a
    /// `ggml-<id>.bin` that whisper-rs would fail to load on next
    /// launch.
    pub fn download<F: FnMut(DownloadProgress)>(
        &self,
        model: WhisperModel,
        mut on_progress: F,
    ) -> Result<PathBuf> {
        fs::create_dir_all(&self.root).map_err(|e| {
            AttuneError::Storage(format!(
                "could not create models dir {}: {e}",
                self.root.display()
            ))
        })?;

        let target = self.path_for(model);
        let tmp = target.with_extension("bin.part");
        info!(model = model.id(), url = %model.url(), target = %target.display(), "downloading whisper model");

        let client = reqwest::blocking::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            // No total timeout — large-v3 can take 30+ minutes on a
            // slow link and that is fine. We rely on the connect
            // timeout + the user being able to cancel the app.
            .build()
            .map_err(|e| {
                AttuneError::Storage(format!("could not build download client: {e}"))
            })?;

        let mut response = client
            .get(model.url())
            .send()
            .map_err(|e| AttuneError::Storage(format!("model download failed: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(AttuneError::Storage(format!(
                "model download returned {status} for {}",
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
            downloaded += n as u64;
            on_progress(DownloadProgress { downloaded, total });
        }

        // Make sure everything hit the disk before the rename.
        file.sync_all()
            .map_err(|e| AttuneError::Storage(format!("download sync error: {e}")))?;
        drop(file);

        fs::rename(&tmp, &target).map_err(|e| {
            AttuneError::Storage(format!(
                "could not finalize model file {}: {e}",
                target.display()
            ))
        })?;

        info!(
            target = %target.display(),
            bytes = downloaded,
            "whisper model download complete",
        );
        Ok(target)
    }

    /// Best-effort cleanup of stale .part files from a previous failed
    /// download. Logged, never returned as an error.
    pub fn clean_partials(&self) {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("part") {
                if let Err(e) = fs::remove_file(&path) {
                    warn!(path = %path.display(), error = %e, "could not remove stale model .part file");
                } else {
                    debug!(path = %path.display(), "removed stale model .part file");
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
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".local")
            .join("share")
            .join("attune")
            .join("models")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_round_trips() {
        for m in [
            WhisperModel::Tiny,
            WhisperModel::Base,
            WhisperModel::Small,
            WhisperModel::Medium,
            WhisperModel::LargeV3,
        ] {
            assert_eq!(WhisperModel::from_id(m.id()), Some(m));
        }
    }

    #[test]
    fn unknown_id_returns_none() {
        assert_eq!(WhisperModel::from_id("turbo"), None);
        assert_eq!(WhisperModel::from_id(""), None);
    }

    #[test]
    fn status_reports_absent_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = WhisperModelStore::new(dir.path());
        let status = store.status(WhisperModel::Tiny);
        assert!(!status.present);
        assert_eq!(status.bytes_on_disk, None);
        assert_eq!(status.id, "tiny");
    }

    #[test]
    fn status_reports_present_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = WhisperModelStore::new(dir.path());
        std::fs::create_dir_all(store.root()).unwrap();
        let path = store.path_for(WhisperModel::Tiny);
        std::fs::write(&path, b"not a real model").unwrap();
        let status = store.status(WhisperModel::Tiny);
        assert!(status.present);
        assert_eq!(status.bytes_on_disk, Some(16));
    }
}
