use keyring::Entry;
use tracing::debug;

use crate::error::{AttuneError, Result};
use crate::llm::ProviderId;

const KEYCHAIN_SERVICE: &str = "com.attune.app.provider-key";

pub struct KeyStore;

impl KeyStore {
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

    pub fn has(provider: ProviderId) -> bool {
        matches!(Self::get(provider), Ok(Some(_)))
    }

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
