use nnnoiseless::DenoiseState;

const FRAME: usize = DenoiseState::FRAME_SIZE;

const I16_SCALE: f32 = 32_768.0;

pub(super) fn enhance(samples_48k_mono: &[f32], atten_lim_db: f32) -> Result<Vec<f32>, String> {
    if samples_48k_mono.is_empty() {
        return Ok(Vec::new());
    }

    let floor = 10f32.powf(atten_lim_db / 20.0).clamp(0.0, 1.0);
    let wet = 1.0 - floor;

    let mut state = DenoiseState::new();
    let n = samples_48k_mono.len();
    let mut out = vec![0.0f32; n];

    let mut in_frame = [0.0f32; FRAME];
    let mut out_frame = [0.0f32; FRAME];

    let mut pos = 0;
    while pos < n {
        let take = (n - pos).min(FRAME);

        for (i, slot) in in_frame.iter_mut().enumerate() {
            *slot = if i < take {
                samples_48k_mono[pos + i] * I16_SCALE
            } else {
                0.0
            };
        }

        let _vad = state.process_frame(&mut out_frame, &in_frame);
        for i in 0..take {
            let enh = out_frame[i] / I16_SCALE;
            let dry = samples_48k_mono[pos + i];
            out[pos + i] = floor * dry + wet * enh;
        }
        pos += take;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_empty_output() {
        assert!(enhance(&[], -20.0).unwrap().is_empty());
    }

    #[test]
    fn output_length_matches_input() {
        let input: Vec<f32> = (0..(FRAME * 3 + 200))
            .map(|i| (i as f32 * 0.01).sin() * 0.2)
            .collect();
        let out = enhance(&input, -20.0).unwrap();
        assert_eq!(out.len(), input.len());
    }

    #[test]
    fn zero_db_atten_is_passthrough() {
        let input: Vec<f32> = (0..FRAME).map(|i| (i as f32 * 0.05).sin() * 0.3).collect();
        let out = enhance(&input, 0.0).unwrap();
        for (a, b) in input.iter().zip(out.iter()) {
            assert!((a - b).abs() < 1e-6, "{a} vs {b}");
        }
    }

    #[test]
    fn floor_is_respected_on_a_fully_suppressed_signal() {
        let input: Vec<f32> = (0..(FRAME * 4))
            .map(|i| (((i * 1103515245 + 12345) % 1000) as f32 / 1000.0 - 0.5) * 0.4)
            .collect();
        let out = enhance(&input, -20.0).unwrap();
        let dry_rms = rms(&input);
        let out_rms = rms(&out);

        assert!(
            out_rms >= 0.10 * dry_rms * 0.5,
            "out_rms {out_rms} fell below the -20 dB floor of dry_rms {dry_rms}"
        );
    }

    fn rms(s: &[f32]) -> f32 {
        if s.is_empty() {
            return 0.0;
        }
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }
}
