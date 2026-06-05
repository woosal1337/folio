use std::time::{Duration, Instant};

use parking_lot::Mutex;

const SILENCE_WARN_SECS: u64 = 6;
const SILENCE_DBFS: f64 = -45.0;

pub struct LevelMeter {
    accum: Mutex<(f64, u64)>,
    started: Instant,
}

impl LevelMeter {
    pub fn new() -> Self {
        Self {
            accum: Mutex::new((0.0, 0)),
            started: Instant::now(),
        }
    }

    pub fn record(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let mut sum_sq = 0.0_f64;
        for &s in samples {
            let v = s as f64;
            sum_sq += v * v;
        }
        let mut guard = self.accum.lock();
        guard.0 += sum_sq;
        guard.1 += samples.len() as u64;
    }

    pub fn is_silent(&self) -> bool {
        if self.started.elapsed() < Duration::from_secs(SILENCE_WARN_SECS) {
            return false;
        }
        let (sum_sq, count) = *self.accum.lock();
        if count == 0 {
            return true;
        }
        let rms = (sum_sq / count as f64).sqrt();
        if !rms.is_finite() || rms <= 0.0 {
            return true;
        }
        20.0 * rms.log10() < SILENCE_DBFS
    }
}

impl Default for LevelMeter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_before_warmup() {
        let m = LevelMeter::new();
        m.record(&[0.0; 1024]);
        assert!(!m.is_silent());
    }

    #[test]
    fn loud_signal_is_not_silent() {
        let m = LevelMeter {
            accum: Mutex::new((0.0, 0)),
            started: Instant::now() - Duration::from_secs(SILENCE_WARN_SECS + 1),
        };
        let loud: Vec<f32> = (0..48_000).map(|i| 0.3 * (i as f32 / 7.0).sin()).collect();
        m.record(&loud);
        assert!(!m.is_silent());
    }

    #[test]
    fn near_silent_signal_is_flagged() {
        let m = LevelMeter {
            accum: Mutex::new((0.0, 0)),
            started: Instant::now() - Duration::from_secs(SILENCE_WARN_SECS + 1),
        };
        let faint: Vec<f32> = (0..48_000)
            .map(|i| 0.001 * (i as f32 / 7.0).sin())
            .collect();
        m.record(&faint);
        assert!(m.is_silent());
    }

    #[test]
    fn no_samples_after_warmup_is_silent() {
        let m = LevelMeter {
            accum: Mutex::new((0.0, 0)),
            started: Instant::now() - Duration::from_secs(SILENCE_WARN_SECS + 1),
        };
        assert!(m.is_silent());
    }
}
