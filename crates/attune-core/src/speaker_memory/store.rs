//! Default on-disk location and OS-keychain passphrase for the encrypted
//! speaker registry, plus convenience load/save helpers.
//!
//! The registry itself ([`SpeakerRegistry`]) is passphrase-agnostic — it
//! takes the key as a parameter so it stays pure and testable. This module
//! supplies the real key: a 256-bit secret generated once and kept in the
//! macOS Keychain (never written to disk), the same trust boundary the
//! provider API keys use.

use std::path::PathBuf;

use keyring::Entry;
use uuid::Uuid;

use super::SpeakerRegistry;
use crate::error::{AttuneError, Result};

/// Keychain service + account for the registry passphrase.
const KEYCHAIN_SERVICE: &str = "com.attune.app.speaker-registry";
const KEYCHAIN_ACCOUNT: &str = "registry-passphrase";

/// Encrypted registry filename under the app-support dir.
const REGISTRY_FILENAME: &str = "speaker-registry.enc";

fn default_app_support_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "macos")]
    {
        home.join("Library").join("Application Support").join("Attune")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".local").join("share").join("attune")
    }
}

/// Default path of the encrypted registry file.
pub fn default_registry_path() -> PathBuf {
    default_app_support_dir().join(REGISTRY_FILENAME)
}

/// Fetch the registry passphrase from the OS keychain, generating and
/// storing a fresh 256-bit secret (hex-encoded) on first use. The secret
/// never touches disk; losing it (e.g. keychain reset) makes an existing
/// registry unreadable — by design, the encrypted biometric data is then
/// inert rather than recoverable.
pub fn registry_passphrase() -> Result<Vec<u8>> {
    let entry = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| AttuneError::Storage(format!("speaker-registry keychain entry: {e}")))?;
    match entry.get_password() {
        Ok(p) => Ok(p.into_bytes()),
        Err(keyring::Error::NoEntry) => {
            // Two v4 UUIDs = 256 bits of getrandom-backed entropy.
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

/// Load the registry from its default location with the keychain
/// passphrase. A missing file is a first run (empty registry).
pub fn load_default() -> Result<SpeakerRegistry> {
    SpeakerRegistry::load(&default_registry_path(), &registry_passphrase()?)
}

/// Encrypt and persist the registry to its default location.
pub fn save_default(registry: &SpeakerRegistry) -> Result<()> {
    registry.save(&default_registry_path(), &registry_passphrase()?)
}
