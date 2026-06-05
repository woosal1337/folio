use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::{Semaphore, SemaphorePermit};

const HARD_CAP: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DailyBudgetFile {
    pub date: String,

    pub spent_usd: f64,
}

#[derive(Debug)]
pub struct RateLimiter {
    permits: Semaphore,
    budget_usd: f64,
    spent_usd_cents: AtomicU64,
    store_path: PathBuf,
    today: String,
}

impl RateLimiter {
    pub fn new(max_concurrency: usize, budget_usd: f64, store_path: PathBuf) -> Self {
        let n = max_concurrency.clamp(1, HARD_CAP);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let (loaded_date, loaded_spent) = match load_file(&store_path) {
            Some(state) => (state.date, state.spent_usd),
            None => (today.clone(), 0.0),
        };

        let spent = if loaded_date == today {
            loaded_spent
        } else {
            0.0
        };
        RateLimiter {
            permits: Semaphore::new(n),
            budget_usd,
            spent_usd_cents: AtomicU64::new((spent * 100.0).round() as u64),
            store_path,
            today,
        }
    }

    #[must_use]
    pub fn spent_usd(&self) -> f64 {
        self.spent_usd_cents.load(Ordering::Relaxed) as f64 / 100.0
    }

    #[must_use]
    pub fn budget_usd(&self) -> f64 {
        self.budget_usd
    }

    #[must_use]
    pub fn would_exceed_budget(&self, projected_run_usd: f64) -> bool {
        if self.budget_usd <= 0.0 {
            return false;
        }
        self.spent_usd() + projected_run_usd > self.budget_usd
    }

    pub async fn reserve(&self) -> Result<SemaphorePermit<'_>, BudgetExceeded> {
        if self.budget_usd > 0.0 && self.spent_usd() >= self.budget_usd {
            return Err(BudgetExceeded {
                spent_usd: self.spent_usd(),
                budget_usd: self.budget_usd,
            });
        }
        let permit = self
            .permits
            .acquire()
            .await
            .expect("invariant: this semaphore is owned by &self and is never explicitly closed across the app lifetime; acquire can only fail when close() has been called");
        Ok(permit)
    }

    pub fn record_cost(&self, run_usd: f64) {
        let cents = (run_usd * 100.0).round() as u64;
        self.spent_usd_cents.fetch_add(cents, Ordering::Relaxed);
        let state = DailyBudgetFile {
            date: self.today.clone(),
            spent_usd: self.spent_usd(),
        };
        if let Err(e) = save_file(&self.store_path, &state) {
            tracing::warn!(error = %e, "could not flush daily budget state");
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy)]
#[error("daily agent budget exceeded: ${spent_usd:.4} of ${budget_usd:.2}")]
pub struct BudgetExceeded {
    pub spent_usd: f64,

    pub budget_usd: f64,
}

fn load_file(path: &Path) -> Option<DailyBudgetFile> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_file(path: &Path, state: &DailyBudgetFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(state).unwrap_or_default())
}

pub fn default_budget_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Application Support")
            .join("Folio")
            .join("agent-budget")
            .join("today.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".config")
            .join("folio")
            .join("agent-budget")
            .join("today.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clamps_concurrency_to_hard_cap() {
        let dir = tempfile::tempdir().unwrap();
        let limiter = RateLimiter::new(99, 10.0, dir.path().join("budget.json"));
        assert_eq!(limiter.permits.available_permits(), HARD_CAP);
    }

    #[test]
    fn record_cost_persists_to_disk_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("budget.json");
        let limiter = RateLimiter::new(2, 1.0, path.clone());
        limiter.record_cost(0.123);
        limiter.record_cost(0.45);

        let reloaded = RateLimiter::new(2, 1.0, path);
        assert!((reloaded.spent_usd() - 0.57).abs() < 0.01);
    }

    #[test]
    fn would_exceed_budget_respects_zero_meaning_unlimited() {
        let dir = tempfile::tempdir().unwrap();
        let limiter = RateLimiter::new(2, 0.0, dir.path().join("budget.json"));
        assert!(!limiter.would_exceed_budget(999.0));
    }

    #[tokio::test]
    async fn reserve_fails_when_budget_already_spent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("budget.json");
        let limiter = RateLimiter::new(2, 0.10, path);
        limiter.record_cost(0.15);
        let result = limiter.reserve().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn reserve_caps_concurrency() {
        let dir = tempfile::tempdir().unwrap();
        let limiter = RateLimiter::new(2, 10.0, dir.path().join("budget.json"));
        let p1 = limiter.reserve().await.unwrap();
        let p2 = limiter.reserve().await.unwrap();
        assert_eq!(limiter.permits.available_permits(), 0);
        drop(p1);
        drop(p2);
    }
}
