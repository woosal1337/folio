use std::path::PathBuf;

use super::SpeakerRegistry;
use crate::error::{FolioError, Result};

const REGISTRY_FILENAME: &str = "speaker-registry.json";

fn default_app_support_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Application Support")
            .join("Folio")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".local").join("share").join("folio")
    }
}

pub fn default_registry_path() -> PathBuf {
    default_app_support_dir().join(REGISTRY_FILENAME)
}

pub fn load_default() -> Result<SpeakerRegistry> {
    SpeakerRegistry::load_plain(&default_registry_path())
}

pub fn save_default(registry: &SpeakerRegistry) -> Result<()> {
    let path = default_registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            FolioError::Storage(format!(
                "could not create speaker registry dir {}: {e}",
                parent.display()
            ))
        })?;
    }
    registry.save_plain(&path)
}
