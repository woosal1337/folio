const DEFAULT_WINDOW_SAMPLES: usize = 16_000 * 30;
const DEFAULT_RMS_FLOOR: f32 = 0.0056;
const DEFAULT_MIN_GAP_SECS: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveRange {
    pub start: usize,
    pub end: usize,
}

pub fn active_ranges(pcm: &[f32], sample_rate: u32) -> Vec<ActiveRange> {
    active_ranges_with(
        pcm,
        sample_rate,
        DEFAULT_WINDOW_SAMPLES,
        DEFAULT_RMS_FLOOR,
        DEFAULT_MIN_GAP_SECS,
    )
}

pub fn active_ranges_with(
    pcm: &[f32],
    sample_rate: u32,
    window: usize,
    rms_floor: f32,
    min_gap_secs: f32,
) -> Vec<ActiveRange> {
    if pcm.is_empty() || window == 0 || sample_rate == 0 {
        return Vec::new();
    }
    let mut raw: Vec<ActiveRange> = Vec::new();
    let mut idx = 0;
    while idx < pcm.len() {
        let end = (idx + window).min(pcm.len());
        let slice = &pcm[idx..end];
        if rms(slice) >= rms_floor {
            raw.push(ActiveRange { start: idx, end });
        }
        idx = end;
    }
    merge_close(&raw, sample_rate, min_gap_secs)
}

fn merge_close(raw: &[ActiveRange], sample_rate: u32, min_gap_secs: f32) -> Vec<ActiveRange> {
    if raw.is_empty() {
        return Vec::new();
    }
    let gap_samples = (min_gap_secs * sample_rate as f32) as usize;
    let mut out = vec![raw[0]];
    for range in raw.iter().skip(1) {
        let last = out
            .last_mut()
            .expect("invariant: out is non-empty here because we pushed at least one element above before entering this branch");
        if range.start.saturating_sub(last.end) <= gap_samples {
            last.end = range.end;
        } else {
            out.push(*range);
        }
    }
    out
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn loud_sine(samples: usize, freq_hz: u32, sample_rate: u32) -> Vec<f32> {
        (0..samples)
            .map(|i| (2.0 * PI * freq_hz as f32 * i as f32 / sample_rate as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn pure_silence_returns_no_ranges() {
        let pcm = vec![0.0_f32; 16_000 * 60];
        assert!(active_ranges(&pcm, 16_000).is_empty());
    }

    #[test]
    fn pure_speech_returns_one_range_covering_the_whole_buffer() {
        let pcm = loud_sine(16_000 * 30, 440, 16_000);
        let ranges = active_ranges(&pcm, 16_000);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[0].end, pcm.len());
    }

    #[test]
    fn silence_in_the_middle_splits_into_two_ranges() {
        let mut pcm = loud_sine(16_000 * 30, 440, 16_000);
        pcm.extend(std::iter::repeat_n(0.0_f32, 16_000 * 30));
        pcm.extend(loud_sine(16_000 * 30, 440, 16_000));
        let ranges = active_ranges(&pcm, 16_000);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[1].end, pcm.len());
    }

    #[test]
    fn short_silent_gap_is_bridged() {
        let mut pcm = loud_sine(16_000 * 30, 440, 16_000);
        pcm.extend(std::iter::repeat_n(0.0_f32, 16_000));
        pcm.extend(loud_sine(16_000 * 30, 440, 16_000));

        let ranges = active_ranges_with(
            &pcm,
            16_000,
            16_000,
            DEFAULT_RMS_FLOOR,
            DEFAULT_MIN_GAP_SECS,
        );

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[0].end, pcm.len());
    }
}
