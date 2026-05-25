//! Vault snapshot export — bundles the user's recordings folder,
//! memory folder, tasks file, and a copy of the active settings into
//! a single zip the user can drop into any sync provider (iCloud,
//! Dropbox, git-annex, …) or hand-carry between machines.
//!
//! The zip layout is deliberately flat + obvious so anyone with stock
//! `unzip` can browse it without needing the app:
//!
//! ```text
//! attune-snapshot-YYYY-MM-DD-HHMMSS/
//!   manifest.json
//!   settings.json
//!   tasks.json                  (when present)
//!   recordings/<session-name>/...
//!   memory/<file>.md
//! ```
//!
//! All zip entries use Deflate compression because the recordings
//! tree's headline artifacts (.wav / .json.zst) are already
//! incompressible — Store mode here would not shrink the file but
//! Deflate gracefully no-ops on already-compressed blobs.
//!
//! v2 roadmap finding 057 (GET-92).

use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;
use ts_rs::TS;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::{AttuneError, Result};

/// Inputs the caller wires from `Settings` / `AppState`. We accept
/// explicit paths rather than reaching into a global so the function
/// stays unit-testable from `attune-core` alone.
#[derive(Debug, Clone)]
pub struct SnapshotPaths {
    pub recordings_dir: PathBuf,
    pub memory_dir: PathBuf,
    pub tasks_path: PathBuf,
    /// On-disk location of the active `settings.json`. We re-read it
    /// here rather than serialising the in-memory `Settings` struct
    /// so the snapshot's settings byte-for-byte matches the disk
    /// state.
    pub settings_path: PathBuf,
}

/// Result of a successful export.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct SnapshotSummary {
    pub destination: PathBuf,
    /// Number of files inside the zip (not counting `manifest.json`).
    pub files: usize,
    /// Compressed size of the resulting zip in bytes.
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
struct Manifest {
    schema_version: u32,
    created_at: String,
    recordings_dir: PathBuf,
    memory_dir: PathBuf,
    tasks_path: PathBuf,
    settings_path: PathBuf,
    files: usize,
}

const SCHEMA_VERSION: u32 = 1;

/// Build a snapshot zip at `destination`. The destination is created
/// (or truncated) by this function; the parent directory must exist.
pub fn export(destination: &Path, paths: &SnapshotPaths) -> Result<SnapshotSummary> {
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            return Err(AttuneError::Storage(format!(
                "snapshot destination parent {} does not exist",
                parent.display()
            )));
        }
    }
    let file = File::create(destination).map_err(|e| {
        AttuneError::Storage(format!(
            "could not create snapshot {}: {e}",
            destination.display()
        ))
    })?;
    let writer = BufWriter::new(file);
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let mut files = 0_usize;

    if paths.settings_path.is_file() {
        copy_into_zip(&mut zip, &paths.settings_path, "settings.json", options)?;
        files += 1;
    }
    if paths.tasks_path.is_file() {
        copy_into_zip(&mut zip, &paths.tasks_path, "tasks.json", options)?;
        files += 1;
    }
    if paths.recordings_dir.is_dir() {
        files += copy_tree_into_zip(&mut zip, &paths.recordings_dir, "recordings/", options)?;
    }
    if paths.memory_dir.is_dir() {
        files += copy_tree_into_zip(&mut zip, &paths.memory_dir, "memory/", options)?;
    }

    // Manifest is emitted last so its file count is accurate.
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        created_at: Utc::now().to_rfc3339(),
        recordings_dir: paths.recordings_dir.clone(),
        memory_dir: paths.memory_dir.clone(),
        tasks_path: paths.tasks_path.clone(),
        settings_path: paths.settings_path.clone(),
        files,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| AttuneError::Storage(format!("manifest serialize: {e}")))?;
    zip.start_file("manifest.json", options)
        .map_err(|e| AttuneError::Storage(format!("manifest start_file: {e}")))?;
    zip.write_all(&manifest_bytes)
        .map_err(|e| AttuneError::Storage(format!("manifest write: {e}")))?;

    zip.finish()
        .map_err(|e| AttuneError::Storage(format!("snapshot finalize: {e}")))?;

    let bytes = fs::metadata(destination)
        .map(|m| m.len())
        .map_err(|e| AttuneError::Storage(format!("snapshot stat: {e}")))?;

    Ok(SnapshotSummary {
        destination: destination.to_path_buf(),
        files,
        bytes,
    })
}

/// Suggested default filename — placed at the root of the user's
/// home directory by the Tauri command before the save dialog opens.
pub fn default_filename(now: &chrono::DateTime<Utc>) -> String {
    format!("attune-snapshot-{}.zip", now.format("%Y-%m-%d-%H%M%S"))
}

fn copy_into_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    src: &Path,
    dest_name: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    let mut reader =
        BufReader::new(File::open(src).map_err(|e| {
            AttuneError::Storage(format!("open for snapshot {}: {e}", src.display()))
        })?);
    zip.start_file(dest_name, options)
        .map_err(|e| AttuneError::Storage(format!("zip start_file {dest_name}: {e}")))?;
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| AttuneError::Storage(format!("read {}: {e}", src.display())))?;
        if n == 0 {
            break;
        }
        zip.write_all(&buf[..n])
            .map_err(|e| AttuneError::Storage(format!("zip write {dest_name}: {e}")))?;
    }
    Ok(())
}

/// Walk `root` depth-first, copying every regular file into the zip
/// under `prefix`. Returns the number of files copied.
fn copy_tree_into_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    root: &Path,
    prefix: &str,
    options: SimpleFileOptions,
) -> Result<usize> {
    let mut count = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir)
            .map_err(|e| AttuneError::Storage(format!("read_dir {}: {e}", dir.display())))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let zip_name = format!("{prefix}{rel}");
            copy_into_zip(zip, &path, &zip_name, options)?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    #[test]
    fn export_zips_every_known_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let recordings = dir.path().join("recordings");
        let session = recordings.join("2026-05-25-test");
        fs::create_dir_all(&session).unwrap();
        fs::write(session.join("mic.wav"), b"FAKE-WAV").unwrap();
        fs::write(session.join("transcript.json.zst"), b"FAKE-TRANSCRIPT").unwrap();
        let memory = dir.path().join("memory");
        fs::create_dir_all(&memory).unwrap();
        fs::write(memory.join("claim_abc.md"), b"---\nid: x\n---\n").unwrap();
        let tasks_path = dir.path().join("tasks.json");
        fs::write(&tasks_path, b"[]").unwrap();
        let settings_path = dir.path().join("settings.json");
        fs::write(&settings_path, b"{\"theme\":\"light\"}").unwrap();

        let dest = dir.path().join("snapshot.zip");
        let summary = export(
            &dest,
            &SnapshotPaths {
                recordings_dir: recordings.clone(),
                memory_dir: memory.clone(),
                tasks_path: tasks_path.clone(),
                settings_path: settings_path.clone(),
            },
        )
        .unwrap();

        assert!(dest.exists());
        // settings.json + tasks.json + 2 recordings + 1 memory = 5
        assert_eq!(summary.files, 5);
        assert_eq!(summary.destination, dest);
        assert!(summary.bytes > 0);

        let mut archive = ZipArchive::new(File::open(&dest).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "settings.json"));
        assert!(names.iter().any(|n| n == "tasks.json"));
        assert!(names
            .iter()
            .any(|n| n == "recordings/2026-05-25-test/mic.wav"));
        assert!(names
            .iter()
            .any(|n| n == "recordings/2026-05-25-test/transcript.json.zst"));
        assert!(names.iter().any(|n| n == "memory/claim_abc.md"));
        assert!(names.iter().any(|n| n == "manifest.json"));

        // Manifest is well-formed JSON and remembers the file count.
        let mut manifest = String::new();
        archive
            .by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        assert!(manifest.contains("\"schema_version\": 1"));
        assert!(manifest.contains("\"files\": 5"));
    }

    #[test]
    fn export_skips_missing_inputs_without_erroring() {
        let dir = tempfile::tempdir().unwrap();
        // Nothing exists — every input path is a phantom.
        let dest = dir.path().join("empty.zip");
        let summary = export(
            &dest,
            &SnapshotPaths {
                recordings_dir: dir.path().join("missing-recordings"),
                memory_dir: dir.path().join("missing-memory"),
                tasks_path: dir.path().join("missing-tasks.json"),
                settings_path: dir.path().join("missing-settings.json"),
            },
        )
        .unwrap();
        assert!(dest.exists());
        assert_eq!(summary.files, 0);
        // The manifest is still there even when no inputs exist.
        let mut archive = ZipArchive::new(File::open(&dest).unwrap()).unwrap();
        assert_eq!(archive.len(), 1);
        assert_eq!(archive.by_index(0).unwrap().name(), "manifest.json");
    }

    #[test]
    fn default_filename_is_iso_dated() {
        let stamp = chrono::DateTime::parse_from_rfc3339("2026-05-25T17:34:09Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            default_filename(&stamp),
            "attune-snapshot-2026-05-25-173409.zip"
        );
    }
}
