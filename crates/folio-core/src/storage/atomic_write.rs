use std::fs;
use std::io::Write;
use std::path::Path;

use crate::error::{FolioError, Result};

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                FolioError::Storage(format!("create_dir_all {}: {e}", parent.display()))
            })?;
        }
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));
    {
        let mut file = fs::File::create(&tmp)
            .map_err(|e| FolioError::Storage(format!("create {}: {e}", tmp.display())))?;
        file.write_all(bytes)
            .map_err(|e| FolioError::Storage(format!("write {}: {e}", tmp.display())))?;

        file.sync_all()
            .map_err(|e| FolioError::Storage(format!("fsync {}: {e}", tmp.display())))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        FolioError::Storage(format!(
            "rename {} -> {}: {e}",
            tmp.display(),
            path.display()
        ))
    })?;
    Ok(())
}

pub fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| FolioError::Storage(format!("serialize json: {e}")))?;
    atomic_write(path, &bytes)
}

pub fn read_schema_version(stamp_path: &Path) -> u32 {
    fs::read_to_string(stamp_path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

pub fn write_schema_version(stamp_path: &Path, version: u32) -> Result<()> {
    atomic_write(stamp_path, version.to_string().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn atomic_write_round_trips_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.json");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello");

        atomic_write(&path, b"world").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"world");

        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn atomic_write_creates_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/sub/dir/file.txt");
        atomic_write(&path, b"hi").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn schema_version_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let stamp = dir.path().join("store.schema");
        assert_eq!(read_schema_version(&stamp), 0);
        write_schema_version(&stamp, 7).unwrap();
        assert_eq!(read_schema_version(&stamp), 7);
    }

    #[test]
    fn atomic_write_json_serialises_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        let value = serde_json::json!({"a": 1, "b": [true, false]});
        atomic_write_json(&path, &value).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed["a"], 1);
    }
}
