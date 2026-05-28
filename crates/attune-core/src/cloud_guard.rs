//! CloudGuard — central toggle that physically blocks every cloud
//! egress when Privacy Mode (a.k.a. Airgap) is on.
//!
//! v2 finding 048 / GET-42. The demo Granola can't replicate is "wifi
//! off, app still works end-to-end" — we deliver that by gating all
//! HTTP calls (LLM providers, embeddings, model downloads, webhooks)
//! behind a process-global atomic flag.
//!
//! Architecture:
//!
//!   - `set_airgap(bool)` flips a `static AtomicBool` set at startup
//!     from `Settings.privacy_mode` and updated whenever the user
//!     toggles the switch in Settings → Privacy.
//!   - Every outbound HTTP call site (LLM, embeddings, model
//!     downloader, webhook delivery, sentry/posthog if any) calls
//!     `ensure_allowed(host)` before constructing the request. If the
//!     flag is on and the host isn't on the allowlist (currently:
//!     `localhost`, `127.0.0.1`, `::1`), the call short-circuits with
//!     `CloudGuardError::Airgapped { host }`.
//!   - The error implements Display so providers can surface a clear
//!     "blocked by Privacy Mode" message rather than a generic network
//!     failure.
//!
//! The titlebar AIRGAP badge subscribes to the flag via a tauri event
//! emitted on every toggle, so the UI mirrors the actual block state.

use std::sync::atomic::{AtomicBool, Ordering};

static AIRGAP: AtomicBool = AtomicBool::new(false);

/// Hosts that are NEVER blocked even when airgap is on. Local-only
/// services (the local Whisper model server, a sidecar embedding
/// provider, a local LLM at 127.0.0.1) must keep working.
const ALWAYS_ALLOWED: &[&str] = &["localhost", "127.0.0.1", "::1", "0.0.0.0"];

#[derive(Debug, thiserror::Error)]
pub enum CloudGuardError {
    #[error("Privacy Mode is on — outbound request to {host} blocked")]
    Airgapped { host: String },
}

/// Flip the global airgap state. Called once at startup from Settings
/// and then on every Settings.privacy_mode change.
pub fn set_airgap(on: bool) {
    AIRGAP.store(on, Ordering::SeqCst);
}

/// Current state. The UI reads this to render the AIRGAP badge.
pub fn is_airgap() -> bool {
    AIRGAP.load(Ordering::SeqCst)
}

/// Returns Ok(()) iff the network request is allowed in the current
/// mode. Callers should run this just before opening the socket.
pub fn ensure_allowed(host: &str) -> Result<(), CloudGuardError> {
    if !is_airgap() {
        return Ok(());
    }
    let host_lc = host.to_ascii_lowercase();
    if ALWAYS_ALLOWED.iter().any(|h| host_lc == *h) {
        return Ok(());
    }
    // Catch IP literal allowances (e.g. "127.0.0.1:8080" — split the
    // port off so the port doesn't fool the allowlist check).
    if let Some(bare) = host_lc.split(':').next() {
        if ALWAYS_ALLOWED.contains(&bare) {
            return Ok(());
        }
    }
    Err(CloudGuardError::Airgapped {
        host: host.to_string(),
    })
}

/// Extract just the host portion of a URL (handles both bare hosts
/// and full URLs with scheme/path). Useful for callers who only have
/// the full URL string at hand.
pub fn host_of(url: &str) -> Option<&str> {
    // Strip scheme.
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    // Slash starts the path; '?' or '#' end the authority too.
    let end = after_scheme
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(after_scheme.len());
    let authority = &after_scheme[..end];
    if authority.is_empty() {
        None
    } else {
        Some(authority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // NOTE: `AIRGAP` is a process-global atomic, so tests that flip it
    // race each other (and any other test calling `ensure_allowed`)
    // under cargo's default parallel runner. This mutex serialises the
    // airgap-mutating tests; each takes the guard for its whole body
    // and resets the flag before releasing. Tests that only read
    // (`host_of_*`) don't need it.
    static AIRGAP_LOCK: Mutex<()> = Mutex::new(());

    fn reset() {
        set_airgap(false);
    }

    #[test]
    fn defaults_off_allows_everything() {
        let _g = AIRGAP_LOCK.lock().unwrap();
        reset();
        assert!(ensure_allowed("api.openai.com").is_ok());
        assert!(ensure_allowed("huggingface.co").is_ok());
    }

    #[test]
    fn airgap_blocks_external_hosts() {
        let _g = AIRGAP_LOCK.lock().unwrap();
        reset();
        set_airgap(true);
        let err = ensure_allowed("api.openai.com").unwrap_err();
        assert!(matches!(err, CloudGuardError::Airgapped { .. }));
        reset();
    }

    #[test]
    fn airgap_allows_localhost_variants() {
        let _g = AIRGAP_LOCK.lock().unwrap();
        reset();
        set_airgap(true);
        for h in [
            "localhost",
            "127.0.0.1",
            "::1",
            "0.0.0.0",
            "127.0.0.1:8080",
            "LOCALHOST",
        ] {
            assert!(ensure_allowed(h).is_ok(), "should allow {h}");
        }
        reset();
    }

    #[test]
    fn host_of_strips_scheme_and_path() {
        assert_eq!(
            host_of("https://api.openai.com/v1/chat"),
            Some("api.openai.com")
        );
        assert_eq!(
            host_of("http://localhost:8080/health"),
            Some("localhost:8080")
        );
        assert_eq!(host_of("api.openai.com"), Some("api.openai.com"));
        assert_eq!(host_of(""), None);
    }

    #[test]
    fn toggle_is_observable() {
        let _g = AIRGAP_LOCK.lock().unwrap();
        reset();
        assert!(!is_airgap());
        set_airgap(true);
        assert!(is_airgap());
        set_airgap(false);
        assert!(!is_airgap());
    }
}
