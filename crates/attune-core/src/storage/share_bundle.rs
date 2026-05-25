//! Sealed share bundle (.attune-share).
//!
//! Exports a single recording's session directory as a zip with a
//! manifest at the root carrying a SHA-256 hash of every file inside.
//! Recipients can verify the bundle wasn't tampered with by recomputing
//! the hashes; the manifest also records the source path + creation
//! time so the file is self-describing on disk.
//!
//! v2 roadmap finding 052 / GET-69. ed25519 signing is a follow-up —
//! the SHA-256 manifest shipped here is the tamper-evident
//! foundation; adding a signature is a one-field manifest extension
//! once we have a signing-key story.
//!
//! The zip is Deflate-compressed; the .wav/.json.zst payloads are
//! already incompressible so Deflate no-ops on them and the small
//! files (transcript JSON, manifest) get the saving.

use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use ts_rs::TS;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::{AttuneError, Result};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct ShareBundleSummary {
    pub destination: PathBuf,
    pub files: usize,
    pub bytes: u64,
    /// SHA-256 of the manifest contents, hex-encoded. The bundle's
    /// fingerprint the recipient can quote in messages ("I sent you
    /// share bundle abc123…").
    pub manifest_sha256: String,
}

#[derive(Serialize)]
struct ManifestEntry {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Serialize)]
struct Manifest {
    schema_version: u32,
    created_at: String,
    source_session: String,
    files: Vec<ManifestEntry>,
}

const SCHEMA_VERSION: u32 = 1;

/// Build the bundle at `destination`. The destination is created
/// (or truncated) by this function; the parent directory must exist.
pub fn export(session_dir: &Path, destination: &Path) -> Result<ShareBundleSummary> {
    if !session_dir.is_dir() {
        return Err(AttuneError::Storage(format!(
            "session_dir {} is not a directory",
            session_dir.display()
        )));
    }
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            return Err(AttuneError::Storage(format!(
                "share-bundle destination parent {} does not exist",
                parent.display()
            )));
        }
    }

    let file = File::create(destination).map_err(|e| {
        AttuneError::Storage(format!(
            "could not create share bundle {}: {e}",
            destination.display()
        ))
    })?;
    let writer = BufWriter::new(file);
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let mut entries: Vec<ManifestEntry> = Vec::new();
    let mut count = 0;
    let mut total_bytes: u64 = 0;

    let mut stack = vec![session_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let read = fs::read_dir(&dir)
            .map_err(|e| AttuneError::Storage(format!("read_dir {}: {e}", dir.display())))?;
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let rel = path
                .strip_prefix(session_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let zip_name = format!("recording/{rel}");
            let (bytes, sha) = copy_and_hash(&mut zip, &path, &zip_name, options)?;
            entries.push(ManifestEntry {
                path: zip_name,
                bytes,
                sha256: sha,
            });
            count += 1;
            total_bytes += bytes;
        }
    }

    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        created_at: Utc::now().to_rfc3339(),
        source_session: session_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        files: entries,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| AttuneError::Storage(format!("manifest serialize: {e}")))?;
    let manifest_sha256 = {
        let mut h = Sha256::new();
        h.update(&manifest_bytes);
        hex::encode(h.finalize())
    };
    zip.start_file("manifest.json", options)
        .map_err(|e| AttuneError::Storage(format!("manifest start_file: {e}")))?;
    zip.write_all(&manifest_bytes)
        .map_err(|e| AttuneError::Storage(format!("manifest write: {e}")))?;

    zip.finish()
        .map_err(|e| AttuneError::Storage(format!("share bundle finalize: {e}")))?;

    let zip_bytes = fs::metadata(destination)
        .map(|m| m.len())
        .unwrap_or(total_bytes);

    Ok(ShareBundleSummary {
        destination: destination.to_path_buf(),
        files: count,
        bytes: zip_bytes,
        manifest_sha256,
    })
}

fn copy_and_hash<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    src: &Path,
    dest_name: &str,
    options: SimpleFileOptions,
) -> Result<(u64, String)> {
    let mut reader = BufReader::new(File::open(src).map_err(|e| {
        AttuneError::Storage(format!("open for share bundle {}: {e}", src.display()))
    })?);
    zip.start_file(dest_name, options)
        .map_err(|e| AttuneError::Storage(format!("zip start_file {dest_name}: {e}")))?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| AttuneError::Storage(format!("read {}: {e}", src.display())))?;
        if n == 0 {
            break;
        }
        zip.write_all(&buf[..n])
            .map_err(|e| AttuneError::Storage(format!("zip write {dest_name}: {e}")))?;
        hasher.update(&buf[..n]);
        total += n as u64;
    }
    Ok((total, hex::encode(hasher.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::ZipArchive;

    #[test]
    fn export_writes_recording_files_plus_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("2026-05-25-meeting");
        fs::create_dir_all(&session).unwrap();
        fs::write(session.join("mic.wav"), b"FAKE-WAV").unwrap();
        fs::write(session.join("transcript.json.zst"), b"FAKE-TRANSCRIPT").unwrap();

        let dest = dir.path().join("share.attune-share");
        let s = export(&session, &dest).unwrap();
        assert!(dest.exists());
        assert_eq!(s.files, 2);
        assert!(s.manifest_sha256.len() == 64);

        let mut archive = ZipArchive::new(File::open(&dest).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "recording/mic.wav"));
        assert!(names.iter().any(|n| n == "recording/transcript.json.zst"));
        assert!(names.iter().any(|n| n == "manifest.json"));
    }

    #[test]
    fn export_rejects_missing_session_dir() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        let dest = dir.path().join("share.attune-share");
        let result = export(&missing, &dest);
        assert!(result.is_err());
    }
}
