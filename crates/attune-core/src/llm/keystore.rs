//! macOS Keychain (and Linux/Windows native equivalents) wrapper for
//! LLM provider API keys.
//!
//! One keychain entry per provider, under service
//! `com.attune.app.provider-key`, account = the provider's wire id
//! ("openai", "anthropic", "deepseek"). Reading a missing key returns
//! `Ok(None)`, not an error — the "not configured" state must be
//! distinguishable from a real keychain failure.
//!
//! Setting overwrites unconditionally. Deleting is idempotent (no-op
//! if absent). The plaintext key never appears in logs.

use keyring::Entry;
use tracing::debug;

use crate::error::{AttuneError, Result};
use crate::llm::ProviderId;

/// Service identifier used for every entry this app stores in the
/// keychain. Stable across versions; changing it orphans existing keys.
const KEYCHAIN_SERVICE: &str = "com.attune.app.provider-key";

/// Read/write API keys for LLM providers.
pub struct KeyStore;

impl KeyStore {
    /// Read the API key for `provider`. `Ok(None)` means "not
    /// configured" and is the normal first-launch state. `Err` means
    /// the keychain itself failed.
    pub fn get(provider: ProviderId) -> Result<Option<String>> {
        let entry = entry_for(provider)?;
        match entry.get_password() {
            Ok(key) => {
                debug!(provider = provider.as_str(), "loaded api key from keychain");
                Ok(Some(key))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AttuneError::Keychain(e.to_string())),
        }
    }

    /// Store `api_key` under `provider`, overwriting any existing
    /// entry. Empty strings are rejected — pass [`Self::delete`]
    /// to clear a slot intentionally.
    pub fn set(provider: ProviderId, api_key: &str) -> Result<()> {
        if api_key.trim().is_empty() {
            return Err(AttuneError::Llm(
                "refusing to store an empty api key".to_string(),
            ));
        }
        let entry = entry_for(provider)?;
        entry
            .set_password(api_key)
            .map_err(|e| AttuneError::Keychain(e.to_string()))?;
        debug!(provider = provider.as_str(), "stored api key in keychain");
        Ok(())
    }

    /// Remove the key for `provider`. Returns `Ok(())` whether or not
    /// an entry existed.
    pub fn delete(provider: ProviderId) -> Result<()> {
        let entry = entry_for(provider)?;
        match entry.delete_credential() {
            Ok(()) => {
                debug!(
                    provider = provider.as_str(),
                    "removed api key from keychain"
                );
                Ok(())
            }
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AttuneError::Keychain(e.to_string())),
        }
    }

    /// True if a key is present for `provider`. Convenience wrapper
    /// over `get(..).is_some()`.
    pub fn has(provider: ProviderId) -> bool {
        matches!(Self::get(provider), Ok(Some(_)))
    }

    /// Last 4 characters of the stored key, for UI display. Returns
    /// `None` if no key is stored. Never returns the full key.
    pub fn redacted_suffix(provider: ProviderId) -> Option<String> {
        let key = Self::get(provider).ok().flatten()?;
        let chars: Vec<char> = key.chars().collect();
        let n = chars.len();
        let suffix: String = if n >= 4 {
            chars[n - 4..].iter().collect()
        } else {
            chars.iter().collect()
        };
        Some(format!("…{}", suffix))
    }
}

fn entry_for(provider: ProviderId) -> Result<Entry> {
    Entry::new(KEYCHAIN_SERVICE, provider.as_str())
        .map_err(|e| AttuneError::Keychain(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All keychain integration tests are `#[ignore]` because they
    /// touch the real macOS keychain and require an interactive prompt
    /// on first run. They serve as a smoke-test you can run manually:
    /// `cargo test -p attune-core keystore -- --ignored`.
    #[test]
    #[ignore]
    fn round_trip_set_get_delete() {
        let p = ProviderId::OpenAi;
        let _ = KeyStore::delete(p);
        assert!(matches!(KeyStore::get(p), Ok(None)));
        KeyStore::set(p, "sk-test-attune-keystore-1234567890").unwrap();
        let got = KeyStore::get(p).unwrap();
        assert_eq!(got.as_deref(), Some("sk-test-attune-keystore-1234567890"));
        assert_eq!(KeyStore::redacted_suffix(p).unwrap(), "…7890");
        assert!(KeyStore::has(p));
        KeyStore::delete(p).unwrap();
        assert!(!KeyStore::has(p));
    }

    #[test]
    fn rejects_empty_key() {
        let err = KeyStore::set(ProviderId::OpenAi, "   ").unwrap_err();
        assert!(matches!(err, AttuneError::Llm(_)));
    }
}
