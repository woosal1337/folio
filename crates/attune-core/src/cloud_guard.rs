//! CloudGuard — central toggle that physically blocks every cloud
//! egress when Privacy Mode (a.k.a. Airgap) is on, or when a
//! graduated egress policy is active.
//!
//! v2 finding 048 / GET-42. The demo Granola can't replicate is "wifi
//! off, app still works end-to-end" — we deliver that by gating all
//! HTTP calls (LLM providers, embeddings, model downloads, webhooks)
//! behind a process-global atomic flag.
//!
//! ## Three-state model (GET-196)
//!
//! ```text
//! Airgap ON  →  block all non-localhost (hard kill-switch; unchanged)
//! Policy ON  →  only hosts listed in .attune/egress-policy.toml pass
//! Neither    →  allow everything (legacy open mode)
//! ```
//!
//! The policy file lives at `<vault_root>/.attune/egress-policy.toml`.
//! It is optional: if the file does not exist the guard stays in open
//! mode. Airgap always wins over the policy — flipping Privacy Mode on
//! blocks even policy-listed hosts.
//!
//! ### Example `.attune/egress-policy.toml`
//!
//! ```toml
//! [[hosts]]
//! host = "api.openai.com"
//!
//! [[hosts]]
//! host = "api.anthropic.com"
//!
//! [limits]
//! cost_ceiling_usd = 5.0
//! ```
//!
//! ### Architecture:
//!
//!   - `set_airgap(bool)` flips a `static AtomicBool` set at startup
//!     from `Settings.privacy_mode` and updated whenever the user
//!     toggles the switch in Settings → Privacy.
//!   - `load_egress_policy(vault_root)` reads the TOML manifest and
//!     hands the result to `set_egress_policy`, which stores it in a
//!     `static Mutex<Option<EgressPolicy>>`.
//!   - Every outbound HTTP call site calls `ensure_allowed(host)` before
//!     constructing the request. The three-state check runs in order:
//!     airgap → policy → open.
//!   - The error implements Display so providers can surface a clear
//!     message rather than a generic network failure.
//!
//! The titlebar AIRGAP badge subscribes to the flag via a tauri event
//! emitted on every toggle, so the UI mirrors the actual block state.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

static AIRGAP: AtomicBool = AtomicBool::new(false);
static POLICY: Mutex<Option<EgressPolicy>> = Mutex::new(None);

const EGRESS_PATH: &str = ".attune/egress-policy.toml";

/// Hosts that are NEVER blocked even when airgap is on. Local-only
/// services (the local Whisper model server, a sidecar embedding
/// provider, a local LLM at 127.0.0.1) must keep working.
const ALWAYS_ALLOWED: &[&str] = &["localhost", "127.0.0.1", "::1", "0.0.0.0"];

// ---------------------------------------------------------------------------
// Policy types (GET-196)
// ---------------------------------------------------------------------------

/// Graduated egress policy loaded from `.attune/egress-policy.toml`.
///
/// When present and non-empty, only hosts in `hosts` are permitted to
/// receive outbound requests (beyond `ALWAYS_ALLOWED`). An empty
/// `hosts` list is treated identically to a missing policy file —
/// everything is allowed — so the file can be scaffolded with just the
/// `[limits]` section without inadvertently blocking all traffic.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EgressPolicy {
    #[serde(default)]
    pub hosts: Vec<HostEntry>,
    #[serde(default)]
    pub limits: PolicyLimits,
}

/// A single allowed host entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntry {
    /// Bare hostname (no scheme, no port, no path).
    /// Examples: `"api.openai.com"`, `"my-embeddings.internal"`.
    pub host: String,
    /// Human-readable note shown in the policy audit log. Never evaluated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Per-policy cost ceiling (informational; surfaced to agents so they
/// can self-gate before making a call rather than failing mid-flight).
/// The guard itself does not enforce this — integration with the
/// `rate_limit` budget is a follow-up.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyLimits {
    /// Maximum cumulative cost (USD) the agents may spend before the
    /// user is prompted. `None` = no ceiling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_ceiling_usd: Option<f64>,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum CloudGuardError {
    #[error("privacy mode is on, outbound request to {host} blocked")]
    Airgapped { host: String },
    #[error("egress policy blocks outbound request to {host}")]
    PolicyBlocked { host: String },
}

// ---------------------------------------------------------------------------
// Airgap (hard kill-switch)
// ---------------------------------------------------------------------------

/// Flip the global airgap state. Called once at startup from Settings
/// and then on every Settings.privacy_mode change.
pub fn set_airgap(on: bool) {
    AIRGAP.store(on, Ordering::SeqCst);
}

/// Current state. The UI reads this to render the AIRGAP badge.
pub fn is_airgap() -> bool {
    AIRGAP.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// Egress policy (graduated middle state — GET-196)
// ---------------------------------------------------------------------------

/// Replace the process-global egress policy. Pass `None` to reset to
/// open mode (all hosts allowed when airgap is off).
pub fn set_egress_policy(policy: Option<EgressPolicy>) {
    *POLICY.lock().expect("POLICY mutex poisoned") = policy;
}

/// Read `.attune/egress-policy.toml` from `vault_root`. Returns `None`
/// when the file does not exist (normal — open mode). Logs and returns
/// `None` on parse failure so a corrupt file never breaks the app.
pub fn load_egress_policy(vault_root: &Path) -> Option<EgressPolicy> {
    let path = vault_root.join(EGRESS_PATH);
    if !path.exists() {
        return None;
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "egress-policy: read failed");
            return None;
        }
    };
    match toml::from_str::<EgressPolicy>(&raw) {
        Ok(p) => {
            tracing::info!(
                hosts = p.hosts.len(),
                cost_ceiling = ?p.limits.cost_ceiling_usd,
                "egress policy loaded"
            );
            Some(p)
        }
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "egress-policy: parse failed — policy ignored");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Core gate
// ---------------------------------------------------------------------------

fn is_always_allowed(host_lc: &str) -> bool {
    // Exact match (e.g. "localhost", "127.0.0.1").
    if ALWAYS_ALLOWED.contains(&host_lc) {
        return true;
    }
    // Bare-host match for "host:port" strings (e.g. "127.0.0.1:8080").
    if let Some(bare) = host_lc.split(':').next() {
        if ALWAYS_ALLOWED.contains(&bare) {
            return true;
        }
    }
    false
}

/// Returns `Ok(())` iff the network request is allowed in the current
/// mode. Callers should run this just before opening the socket.
///
/// Evaluation order:
/// 1. Airgap on → block all non-localhost.
/// 2. Policy loaded with ≥1 host → only listed hosts pass.
/// 3. Neither → allow everything.
pub fn ensure_allowed(host: &str) -> Result<(), CloudGuardError> {
    let host_lc = host.to_ascii_lowercase();

    // Branch 1 — hard airgap.
    if is_airgap() {
        if is_always_allowed(&host_lc) {
            return Ok(());
        }
        return Err(CloudGuardError::Airgapped {
            host: host.to_string(),
        });
    }

    // Branch 2 — graduated policy.
    {
        let guard = POLICY.lock().expect("POLICY mutex poisoned");
        if let Some(policy) = guard.as_ref() {
            // Empty hosts list = open (policy file present but not yet
            // populated — don't accidentally block everything).
            if !policy.hosts.is_empty() {
                if is_always_allowed(&host_lc) {
                    return Ok(());
                }
                let bare = host_lc.split(':').next().unwrap_or(&host_lc);
                let listed = policy.hosts.iter().any(|e| {
                    let e_lc = e.host.to_ascii_lowercase();
                    bare == e_lc || host_lc == e_lc
                });
                if !listed {
                    return Err(CloudGuardError::PolicyBlocked {
                        host: host.to_string(),
                    });
                }
            }
        }
    }

    // Branch 3 — open.
    Ok(())
}

/// Extract just the host portion of a URL (handles both bare hosts
/// and full URLs with scheme/path). Useful for callers who only have
/// the full URL string at hand.
pub fn host_of(url: &str) -> Option<&str> {
    // Strip scheme.
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    // Slash starts the path; '?' or '#' end the authority too.
    let end = after_scheme
        .find(['/', '?', '#'])
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

    // NOTE: `AIRGAP` and `POLICY` are process-globals, so tests that
    // mutate them race each other under cargo's default parallel runner.
    // This mutex serialises the mutating tests; each takes the guard for
    // its whole body and resets state before releasing.
    static GUARD: Mutex<()> = Mutex::new(());

    fn reset() {
        set_airgap(false);
        set_egress_policy(None);
    }

    // ------------------------------------------------------------------
    // Branch 3 — open mode (no airgap, no policy)
    // ------------------------------------------------------------------

    #[test]
    fn defaults_off_allows_everything() {
        let _g = GUARD.lock().unwrap();
        reset();
        assert!(ensure_allowed("api.openai.com").is_ok());
        assert!(ensure_allowed("huggingface.co").is_ok());
    }

    // ------------------------------------------------------------------
    // Branch 1 — airgap
    // ------------------------------------------------------------------

    #[test]
    fn airgap_blocks_external_hosts() {
        let _g = GUARD.lock().unwrap();
        reset();
        set_airgap(true);
        let err = ensure_allowed("api.openai.com").unwrap_err();
        assert!(matches!(err, CloudGuardError::Airgapped { .. }));
        reset();
    }

    #[test]
    fn airgap_allows_localhost_variants() {
        let _g = GUARD.lock().unwrap();
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
    fn airgap_overrides_policy() {
        let _g = GUARD.lock().unwrap();
        reset();
        set_egress_policy(Some(EgressPolicy {
            hosts: vec![HostEntry {
                host: "api.openai.com".to_string(),
                comment: None,
            }],
            limits: PolicyLimits::default(),
        }));
        set_airgap(true);
        let err = ensure_allowed("api.openai.com").unwrap_err();
        assert!(
            matches!(err, CloudGuardError::Airgapped { .. }),
            "airgap must override policy-allowed hosts"
        );
        reset();
    }

    // ------------------------------------------------------------------
    // Branch 2 — graduated policy
    // ------------------------------------------------------------------

    #[test]
    fn policy_blocks_unlisted_host() {
        let _g = GUARD.lock().unwrap();
        reset();
        set_egress_policy(Some(EgressPolicy {
            hosts: vec![HostEntry {
                host: "api.openai.com".to_string(),
                comment: None,
            }],
            limits: PolicyLimits::default(),
        }));
        let err = ensure_allowed("huggingface.co").unwrap_err();
        assert!(matches!(err, CloudGuardError::PolicyBlocked { .. }));
        reset();
    }

    #[test]
    fn policy_allows_listed_host() {
        let _g = GUARD.lock().unwrap();
        reset();
        set_egress_policy(Some(EgressPolicy {
            hosts: vec![HostEntry {
                host: "api.openai.com".to_string(),
                comment: None,
            }],
            limits: PolicyLimits::default(),
        }));
        assert!(ensure_allowed("api.openai.com").is_ok());
        reset();
    }

    #[test]
    fn policy_allows_listed_host_with_port() {
        let _g = GUARD.lock().unwrap();
        reset();
        set_egress_policy(Some(EgressPolicy {
            hosts: vec![HostEntry {
                host: "api.openai.com".to_string(),
                comment: None,
            }],
            limits: PolicyLimits::default(),
        }));
        assert!(ensure_allowed("api.openai.com:443").is_ok());
        reset();
    }

    #[test]
    fn policy_always_allows_localhost() {
        let _g = GUARD.lock().unwrap();
        reset();
        set_egress_policy(Some(EgressPolicy {
            hosts: vec![HostEntry {
                host: "api.openai.com".to_string(),
                comment: None,
            }],
            limits: PolicyLimits::default(),
        }));
        assert!(ensure_allowed("localhost").is_ok());
        assert!(ensure_allowed("127.0.0.1:11434").is_ok());
        reset();
    }

    #[test]
    fn empty_policy_hosts_is_open() {
        let _g = GUARD.lock().unwrap();
        reset();
        set_egress_policy(Some(EgressPolicy {
            hosts: vec![],
            limits: PolicyLimits::default(),
        }));
        assert!(ensure_allowed("any.host.io").is_ok());
        reset();
    }

    #[test]
    fn no_policy_is_open() {
        let _g = GUARD.lock().unwrap();
        reset();
        set_egress_policy(None);
        assert!(ensure_allowed("any.host.io").is_ok());
        reset();
    }

    // ------------------------------------------------------------------
    // host_of
    // ------------------------------------------------------------------

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

    // ------------------------------------------------------------------
    // Misc
    // ------------------------------------------------------------------

    #[test]
    fn toggle_is_observable() {
        let _g = GUARD.lock().unwrap();
        reset();
        assert!(!is_airgap());
        set_airgap(true);
        assert!(is_airgap());
        set_airgap(false);
        assert!(!is_airgap());
    }
}
