//! Per-recording encryption at rest. v2 finding 051 / GET-68.
//!
//! AES-256-GCM with a 32-byte data key derived from a user
//! passphrase via Argon2id. The passphrase lives in the macOS
//! Keychain (the existing `keystore` module owns the storage); this
//! module turns it into bytes the WAV writer can XOR against.
//!
//! On-disk envelope (little-endian):
//!
//! ```text
//! magic "ATTN1"  (5 bytes)
//! salt          (16 bytes)
//! nonce         (12 bytes)
//! ciphertext    (N bytes, ends with AES-GCM 16-byte tag)
//! ```
//!
//! Memories extracted from an encrypted recording carry an
//! `encrypted_source = true` flag and never inline transcript
//! quotes — the memory layer enforces that policy at the call site.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::error::{AttuneError, Result};

pub const MAGIC: &[u8; 5] = b"ATTN1";
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const KEY_LEN: usize = 32;
pub const HEADER_LEN: usize = MAGIC.len() + SALT_LEN + NONCE_LEN;

/// Derive a 32-byte AES-256 key from `passphrase` and `salt` using
/// Argon2id with sensible defaults (memory 64 MiB, time cost 3, one
/// lane). Salt MUST be uniformly random per-recording so the same
/// passphrase yields a different key per recording.
pub fn derive_key(passphrase: &[u8], salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let params = Params::new(64 * 1024, 3, 1, Some(KEY_LEN))
        .map_err(|e| AttuneError::Storage(format!("argon2 params: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; KEY_LEN];
    argon
        .hash_password_into(passphrase, salt, &mut out)
        .map_err(|e| AttuneError::Storage(format!("argon2 hash: {e}")))?;
    Ok(out)
}

/// Encrypt `plaintext` and prepend the magic + salt + nonce header.
/// Generates a fresh salt + nonce on every call so two ciphertexts of
/// the same plaintext under the same passphrase still differ.
pub fn seal(passphrase: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let salt = random_bytes(SALT_LEN);
    let key_bytes = derive_key(passphrase, &salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes = Aes256Gcm::generate_nonce(&mut OsRng);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| AttuneError::Storage(format!("aes-gcm seal: {e}")))?;
    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt a previously-sealed envelope. Verifies the magic header,
/// re-derives the key from the embedded salt, and returns the
/// plaintext. Fails with a clear error when the magic does not match
/// or the AES-GCM authentication tag rejects the input.
pub fn open(passphrase: &[u8], envelope: &[u8]) -> Result<Vec<u8>> {
    if envelope.len() < HEADER_LEN {
        return Err(AttuneError::Storage(format!(
            "envelope shorter than header: got {} bytes",
            envelope.len()
        )));
    }
    if &envelope[..MAGIC.len()] != MAGIC {
        return Err(AttuneError::Storage(format!(
            "bad magic: expected {:?}",
            std::str::from_utf8(MAGIC).unwrap_or("?")
        )));
    }
    let salt = &envelope[MAGIC.len()..MAGIC.len() + SALT_LEN];
    let nonce_bytes = &envelope[MAGIC.len() + SALT_LEN..HEADER_LEN];
    let ciphertext = &envelope[HEADER_LEN..];
    let key_bytes = derive_key(passphrase, salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AttuneError::Storage(format!("aes-gcm open: {e}")))
}

fn random_bytes(len: usize) -> Vec<u8> {
    use aes_gcm::aead::rand_core::RngCore;
    let mut out = vec![0u8; len];
    OsRng.fill_bytes(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_small_plaintext() {
        let passphrase = b"correct horse battery staple";
        let payload = b"the meeting started at 10am";
        let sealed = seal(passphrase, payload).unwrap();
        let opened = open(passphrase, &sealed).unwrap();
        assert_eq!(opened, payload);
    }

    #[test]
    fn wrong_passphrase_fails_to_open() {
        let sealed = seal(b"right", b"hello").unwrap();
        let result = open(b"wrong", &sealed);
        assert!(result.is_err());
    }

    #[test]
    fn open_rejects_bad_magic() {
        let mut sealed = seal(b"pw", b"hi").unwrap();
        sealed[0] = b'X';
        let result = open(b"pw", &sealed);
        assert!(result.is_err());
    }

    #[test]
    fn open_rejects_truncated_envelope() {
        let sealed = seal(b"pw", b"hi").unwrap();
        let result = open(b"pw", &sealed[..HEADER_LEN - 1]);
        assert!(result.is_err());
    }

    #[test]
    fn two_seals_of_same_input_differ_due_to_random_salt_and_nonce() {
        let a = seal(b"pw", b"hello").unwrap();
        let b = seal(b"pw", b"hello").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn derive_key_is_deterministic_for_same_inputs() {
        let salt = [7u8; SALT_LEN];
        let k1 = derive_key(b"pw", &salt).unwrap();
        let k2 = derive_key(b"pw", &salt).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_key_differs_for_different_salts() {
        let k1 = derive_key(b"pw", &[1u8; SALT_LEN]).unwrap();
        let k2 = derive_key(b"pw", &[2u8; SALT_LEN]).unwrap();
        assert_ne!(k1, k2);
    }
}
