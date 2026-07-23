use keyring::Entry;

use crate::error::{FolioError, Result};

const KEYCHAIN_SERVICE: &str = "com.folio.app.server-token";
const ACCESS: &str = "access";
const REFRESH: &str = "refresh";

pub struct ServerTokens;

impl ServerTokens {
    pub fn set(access: &str, refresh: &str) -> Result<()> {
        if access.trim().is_empty() {
            return Err(FolioError::Backend(
                "refusing to store an empty access token".into(),
            ));
        }
        entry(ACCESS)?.set_password(access).map_err(keychain_err)?;
        entry(REFRESH)?
            .set_password(refresh)
            .map_err(keychain_err)?;
        Ok(())
    }

    pub fn access() -> Result<Option<String>> {
        read(ACCESS)
    }

    pub fn refresh() -> Result<Option<String>> {
        read(REFRESH)
    }

    pub fn clear() -> Result<()> {
        remove(ACCESS)?;
        remove(REFRESH)?;
        Ok(())
    }

    pub fn has() -> bool {
        matches!(Self::access(), Ok(Some(_)))
    }
}

fn entry(account: &str) -> Result<Entry> {
    Entry::new(KEYCHAIN_SERVICE, account).map_err(keychain_err)
}

fn read(account: &str) -> Result<Option<String>> {
    match entry(account)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keychain_err(e)),
    }
}

fn remove(account: &str) -> Result<()> {
    match entry(account)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(keychain_err(e)),
    }
}

fn keychain_err(e: keyring::Error) -> FolioError {
    FolioError::Keychain(e.to_string())
}
