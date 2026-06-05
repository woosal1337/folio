use std::path::PathBuf;

use keyring::Entry;
use uuid::Uuid;

use super::SpeakerRegistry;
use crate::error::{AttuneError, Result};

const KEYCHAIN_SERVICE: &str = "com.attune.app.speaker-registry";
const KEYCHAIN_ACCOUNT: &str = "registry-passphrase";

const REGISTRY_FILENAME: &str = "speaker-registry.enc";

fn default_app_support_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Application Support")
            .join("Attune")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".local").join("share").join("attune")
    }
}

pub fn default_registry_path() -> PathBuf {
    default_app_support_dir().join(REGISTRY_FILENAME)
}

pub fn registry_passphrase() -> Result<Vec<u8>> {
    let entry = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| AttuneError::Storage(format!("speaker-registry keychain entry: {e}")))?;
    match entry.get_password() {
        Ok(p) => Ok(p.into_bytes()),
        Err(keyring::Error::NoEntry) => {
            let mut bytes = Vec::with_capacity(32);
            bytes.extend_from_slice(Uuid::new_v4().as_bytes());
            bytes.extend_from_slice(Uuid::new_v4().as_bytes());
            let pass = hex::encode(&bytes);
            entry.set_password(&pass).map_err(|e| {
                AttuneError::Storage(format!("speaker-registry keychain write: {e}"))
            })?;
            Ok(pass.into_bytes())
        }
        Err(e) => Err(AttuneError::Storage(format!(
            "speaker-registry keychain read: {e}"
        ))),
    }
}

pub fn load_default() -> Result<SpeakerRegistry> {
    SpeakerRegistry::load(&default_registry_path(), &registry_passphrase()?)
}

pub fn save_default(registry: &SpeakerRegistry) -> Result<()> {
    registry.save(&default_registry_path(), &registry_passphrase()?)
}
