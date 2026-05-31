//! RNNoise backend, via the pure-Rust [`nnnoiseless`] crate.
//!
//! RNNoise (Jean-Marc Valin) is a small recurrent denoiser. We ship it
//! as the speech-enhancement backend because it is pure Rust, on
//! crates.io (no git dependency, no cargo-deny source exception), embeds
//! its own ~85 KB model (no first-run download), runs natively at
//! 48 kHz, and is permissively licensed (BSD-3-Clause).
//!
//! DeepFilterNet3 was the originally-researched backend (higher quality
//! on paper). It is deferred: its only Rust runtime is a git crate whose
//! tract dependency is version-deadlocked — the source compiles only
//! against `tract <= 0.21.4`, yet the embedded model fails tract
//! 0.21.4's codegen pass at runtime (`running pass codegen`, NaN-packed
//! conv kernels). Resurrecting it needs an upstream tract bump or a
//! vendored ONNX model driven through `ort`. The
//! [`super::enhance_wav_file`] seam makes swapping the backend
//! mechanical. See GET-188.
//!
//! ## Attenuation limit
//!
//! RNNoise has no built-in suppression-depth control, so we realise
//! `atten_lim_db` as a wet/dry floor:
//!
//! ```text
//! out = floor * dry + (1 - floor) * enhanced,   floor = 10^(atten_lim_db / 20)
//! ```
//!
//! At the default -20 dB the dry signal keeps a 10% floor, so the
//! enhancer can never suppress a region by more than ~20 dB. This is the
//! conservative cap that protects the spectral detail Whisper and the
//! speaker-embedding model rely on (the "When De-noising Hurts" finding,
//! arXiv 2512.17562). `0 dB` is passthrough; very negative dB is
//! (almost) fully wet.

use nnnoiseless::DenoiseState;

/// Samples per RNNoise frame (10 ms at 48 kHz).
const FRAME: usize = DenoiseState::FRAME_SIZE;
/// RNNoise expects samples in 16-bit-integer scale, not normalised
/// `[-1, 1]`. We scale in and back out around `process_frame`.
const I16_SCALE: f32 = 32_768.0;

/// Enhance a 48 kHz mono signal in place-ish: returns a new buffer of the
/// same length, time-aligned with the input. `atten_lim_db` caps the
/// suppression depth (a negative dB value).
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
        // Scale dry → the i16 domain RNNoise expects; zero-pad the final
        // partial frame.
        for (i, slot) in in_frame.iter_mut().enumerate() {
            *slot = if i < take {
                samples_48k_mono[pos + i] * I16_SCALE
            } else {
                0.0
            };
        }
        // `process_frame` returns the frame's voice-activity probability,
        // which we don't need here; it denoises into `out_frame`.
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
        // 3.5 frames worth, to exercise the partial-frame tail.
        let input: Vec<f32> = (0..(FRAME * 3 + 200))
            .map(|i| (i as f32 * 0.01).sin() * 0.2)
            .collect();
        let out = enhance(&input, -20.0).unwrap();
        assert_eq!(out.len(), input.len());
    }

    #[test]
    fn zero_db_atten_is_passthrough() {
        // floor = 10^0 = 1.0 → out == dry exactly, regardless of the
        // denoiser, because wet = 0.
        let input: Vec<f32> = (0..FRAME).map(|i| (i as f32 * 0.05).sin() * 0.3).collect();
        let out = enhance(&input, 0.0).unwrap();
        for (a, b) in input.iter().zip(out.iter()) {
            assert!((a - b).abs() < 1e-6, "{a} vs {b}");
        }
    }

    #[test]
    fn floor_is_respected_on_a_fully_suppressed_signal() {
        // Pure broadband-ish noise: RNNoise should suppress hard, but the
        // -20 dB floor must keep >= ~10% of the dry energy.
        let input: Vec<f32> = (0..(FRAME * 4))
            .map(|i| (((i * 1103515245 + 12345) % 1000) as f32 / 1000.0 - 0.5) * 0.4)
            .collect();
        let out = enhance(&input, -20.0).unwrap();
        let dry_rms = rms(&input);
        let out_rms = rms(&out);
        // Output retains at least the floor fraction of the input energy.
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
