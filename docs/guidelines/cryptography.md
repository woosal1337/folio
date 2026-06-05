# Cryptography conventions

Centralises every cryptographic choice the codebase makes. Cited from
`docs/CODE_STYLE.md` §9.5 and `docs/refactor/PHASE-3-PUNCH-LIST.md`.

## Algorithms in use

| Use case                                         | Algorithm                                   | Crate                             | Module                                               |
| ------------------------------------------------ | ------------------------------------------- | --------------------------------- | ---------------------------------------------------- |
| Per-recording encryption at rest                 | AES-256-GCM with a random 12-byte nonce     | `aes-gcm = "0.10"`                | `folio_core::encryption`                             |
| Key derivation for the above                     | Argon2id (m=64 MiB, t=3, p=1) → 32-byte key | `argon2 = "0.5"`                  | `folio_core::encryption::derive_key`                 |
| Webhook signing                                  | HMAC-SHA256                                 | `hmac = "0.12"` + `sha2 = "0.10"` | `folio_core::webhooks::sign`                         |
| Embedding cache keys                             | SHA-256 of `model_id \0 content`            | `sha2 = "0.10"`                   | `folio_core::memory::embedding_cache::cache_key`     |
| Updater bundle signatures (#020 / Tauri Updater) | Ed25519 over the bundle bytes               | `tauri-plugin-updater = "2"`      | `src-tauri/tauri.conf.json` `plugins.updater.pubkey` |
| TLS                                              | rustls (no openssl in the workspace)        | `reqwest = "*"` with `rustls-tls` | enforced in `deny.toml`                              |
| Macros + bundle hashes (audit trail)             | SHA-256                                     | `sha2 = "0.10"`                   | `folio_core::storage::digest`                        |

## Where keys live

| Key                                          | Storage                                                                   | Module                                                                           |
| -------------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| OpenAI / Anthropic / model-provider API keys | macOS Keychain via the `keyring` crate                                    | `folio_core::llm::KeyStore`                                                      |
| Per-recording encryption passphrase          | macOS Keychain (one entry per recording slug)                             | `folio_core::llm::KeyStore` (`ProviderId::EncryptionPassphrase` slot, follow-up) |
| Webhook signing secret                       | macOS Keychain (one entry per webhook subscription)                       | `folio_core::llm::KeyStore` (`webhook:<id>` slot)                                |
| Tauri-updater private key                    | GitHub Secrets (`TAURI_SIGNING_PRIVATE_KEY`)                              | `.github/workflows/release.yml`                                                  |
| Tauri-updater public key                     | Source-controlled in `src-tauri/tauri.conf.json` `plugins.updater.pubkey` | rotates per `docs/guidelines/release-engineering.md` §key-rotation               |

**Never** persist a key in `Settings`, `localStorage`, or a config file
committed to git. The legacy `Settings.openai_api_key` field exists
only as a transient read-fallback during the Phase-3 B9 migration
(see `docs/refactor/PHASE-3-PUNCH-LIST.md`); it will be removed once a
release of overlap has passed.

## Why AES-256-GCM + Argon2id

- AES-256-GCM is the NIST-blessed AEAD that hardware-accelerated
  AES-NI / ARMv8 instructions make essentially free on Apple Silicon.
  GCM provides authenticated encryption out of the box — the 16-byte
  tag at the end of every ciphertext rejects bit-flip tampering
  without us adding a separate MAC pass.
- Argon2id with 64 MiB memory + 3 iterations is the conservative
  preset from the IETF Crypto Forum Research Group. Memory-hard so a
  GPU rig cannot brute-force a stolen ciphertext + passphrase pair
  cheaply.
- We don't use scrypt (predecessor, less defensible parameter
  guidance) or bcrypt (memory cost too low for modern attackers).

## Why HMAC-SHA256 over CMAC / KMAC for webhooks

Industry standard for HTTP signing (Stripe, GitHub, Slack, Twilio all
use HMAC-SHA256). Library support is universal, so users wiring
Folio into their own infrastructure can verify the signature in any
language. KMAC / SHA-3 would be marginally stronger but the
ecosystem cost outweighs the gain.

## Why rustls (not OpenSSL)

`deny.toml` explicitly bans `openssl` and `openssl-sys`. OpenSSL has a
long CVE history and a complex build story (vendored vs. system).
rustls is memory-safe Rust, builds reproducibly in CI on macOS +
Windows + Linux, and integrates with `webpki-roots` for a trusted CA
bundle without depending on the host OS store.

## Forbidden constructs

- **MD5 and SHA-1** — even as cache keys. SHA-256 is fast enough.
- **AES-CBC without an explicit MAC** — `aes-gcm` is the only AEAD we
  use; CBC + HMAC is error-prone.
- **Custom KDFs** — Argon2id, scrypt, or PBKDF2 only. No rolling your
  own.
- **Hardcoded keys / IV reuse** — every encryption call generates a
  fresh nonce; every Argon2 call uses a fresh salt.
- **Logging keys, ciphertext, or password material** — the `IpcError`
  formatter in `src/shared/lib/ipc.ts` is built to redact these
  before they cross the boundary; never bypass it.

## Audit cadence

- Every dependency rev that touches `aes-gcm`, `argon2`, `hmac`,
  `sha2`, `keyring`, `rustls`, or `tauri-plugin-updater` triggers a
  manual review against the latest RustSec advisory + this doc.
- CI runs `cargo audit` (`.github/workflows/ci.yml::rust-audit`) on
  every PR. Findings under the `[severity = "high"]` threshold block
  the merge.
- `cargo deny check` runs in a macOS + ubuntu matrix slice so
  platform-gated crates (`screencapturekit`, `cocoa`, etc.) get a
  fresh license + advisory pass.

## Related

- `docs/CODE_STYLE.md` §3 (errors) — secret redaction at the IPC
  boundary.
- `docs/CODE_STYLE.md` §8 (security) — keychain storage, deny-listed
  algorithms, Privacy Mode airgap.
- `docs/guidelines/release-engineering.md` — updater key rotation.
- `NOTICE` — third-party crypto-library attributions.
