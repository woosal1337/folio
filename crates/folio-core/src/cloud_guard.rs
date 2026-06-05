use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

static AIRGAP: AtomicBool = AtomicBool::new(false);
static POLICY: Mutex<Option<EgressPolicy>> = Mutex::new(None);

const EGRESS_PATH: &str = ".folio/egress-policy.toml";

const ALWAYS_ALLOWED: &[&str] = &["localhost", "127.0.0.1", "::1", "0.0.0.0"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EgressPolicy {
    #[serde(default)]
    pub hosts: Vec<HostEntry>,
    #[serde(default)]
    pub limits: PolicyLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEntry {
    pub host: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolicyLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_ceiling_usd: Option<f64>,
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CloudGuardError {
    #[error("privacy mode is on, outbound request to {host} blocked")]
    Airgapped { host: String },
    #[error("egress policy blocks outbound request to {host}")]
    PolicyBlocked { host: String },
}

pub fn set_airgap(on: bool) {
    AIRGAP.store(on, Ordering::SeqCst);
}

#[must_use]
pub fn is_airgap() -> bool {
    AIRGAP.load(Ordering::SeqCst)
}

pub fn set_egress_policy(policy: Option<EgressPolicy>) {
    *POLICY.lock() = policy;
}

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

fn is_always_allowed(host_lc: &str) -> bool {
    if ALWAYS_ALLOWED.contains(&host_lc) {
        return true;
    }

    if let Some(bare) = host_lc.split(':').next() {
        if ALWAYS_ALLOWED.contains(&bare) {
            return true;
        }
    }
    false
}

pub fn ensure_allowed(host: &str) -> Result<(), CloudGuardError> {
    let host_lc = host.to_ascii_lowercase();

    if is_airgap() {
        if is_always_allowed(&host_lc) {
            return Ok(());
        }
        return Err(CloudGuardError::Airgapped {
            host: host.to_string(),
        });
    }

    {
        let guard = POLICY.lock();
        if let Some(policy) = guard.as_ref() {
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

    Ok(())
}

pub fn host_of(url: &str) -> Option<&str> {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);

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

    static GUARD: Mutex<()> = Mutex::new(());

    fn reset() {
        set_airgap(false);
        set_egress_policy(None);
    }

    #[test]
    fn defaults_off_allows_everything() {
        let _g = GUARD.lock().unwrap();
        reset();
        assert!(ensure_allowed("api.openai.com").is_ok());
        assert!(ensure_allowed("huggingface.co").is_ok());
    }

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
        let _g = GUARD.lock().unwrap();
        reset();
        assert!(!is_airgap());
        set_airgap(true);
        assert!(is_airgap());
        set_airgap(false);
        assert!(!is_airgap());
    }
}
