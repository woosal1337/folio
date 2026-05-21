//! Resampling to Whisper's expected 16 kHz mono input.
//!
//! Uses `rubato` for high-quality polyphase resampling. The capture callback
//! pushes float samples into the resampler; the resampler emits 16 kHz mono
//! float samples for the WAV writer and (later) the Whisper queue.
//!
//! Multichannel input is downmixed to mono by averaging before resampling.

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::error::{AttuneError, Result};

/// Streaming resampler. Accepts arbitrary-length input chunks and produces
/// output as soon as enough input is buffered.
pub struct StreamingResampler {
    input_sample_rate: u32,
    output_sample_rate: u32,
    input_channels: u16,
    inner: SincFixedIn<f32>,
    /// Input chunk size required by `rubato`. We buffer until we have this many
    /// frames, then process.
    chunk_size: usize,
    /// Pending mono samples awaiting the next resampler chunk.
    pending: Vec<f32>,
}

impl StreamingResampler {
    pub fn new(
        input_sample_rate: u32,
        input_channels: u16,
        output_sample_rate: u32,
    ) -> Result<Self> {
        if input_channels == 0 {
            return Err(AttuneError::Resampler("input_channels must be > 0".into()));
        }

        let chunk_size = 1024;
        let ratio = output_sample_rate as f64 / input_sample_rate as f64;

        // High-quality polyphase: longer sinc kernel + cubic interpolation
        // between table entries. About 2-3x slower than the previous "good
        // enough for Whisper" config and meaningfully cleaner for playback,
        // especially around 4-8 kHz where voice intelligibility lives.
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Cubic,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let inner = SincFixedIn::<f32>::new(ratio, 2.0, params, chunk_size, 1)
            .map_err(|e| AttuneError::Resampler(format!("init failed: {e}")))?;

        Ok(Self {
            input_sample_rate,
            output_sample_rate,
            input_channels,
            inner,
            chunk_size,
            pending: Vec::with_capacity(chunk_size * 4),
        })
    }

    pub fn input_sample_rate(&self) -> u32 {
        self.input_sample_rate
    }
    pub fn output_sample_rate(&self) -> u32 {
        self.output_sample_rate
    }

    /// Feed interleaved multichannel samples. Returns any newly-produced output
    /// mono 16 kHz samples.
    pub fn process(&mut self, interleaved_input: &[f32]) -> Result<Vec<f32>> {
        if interleaved_input.is_empty() {
            return Ok(Vec::new());
        }

        let channels = self.input_channels as usize;
        let frames = interleaved_input.len() / channels;
        self.pending.reserve(frames);

        // Downmix to mono by averaging channels for each frame.
        for frame_idx in 0..frames {
            let start = frame_idx * channels;
            let mut sum = 0.0_f32;
            for c in 0..channels {
                sum += interleaved_input[start + c];
            }
            self.pending.push(sum / channels as f32);
        }

        // Fast-path: no resampling needed.
        if self.input_sample_rate == self.output_sample_rate {
            let out = std::mem::take(&mut self.pending);
            return Ok(out);
        }

        let mut output = Vec::new();
        while self.pending.len() >= self.chunk_size {
            let input_chunk: Vec<f32> = self.pending.drain(..self.chunk_size).collect();
            let input_frames = vec![input_chunk];
            let mut output_frames = self
                .inner
                .process(&input_frames, None)
                .map_err(|e| AttuneError::Resampler(format!("process failed: {e}")))?;
            output.append(&mut output_frames[0]);
        }
        Ok(output)
    }

    /// Drain any remaining buffered samples. Called once at the end of a
    /// capture session.
    pub fn flush(&mut self) -> Result<Vec<f32>> {
        if self.pending.is_empty() {
            return Ok(Vec::new());
        }

        // Pad with zeros to fill one final chunk, process, then return what was
        // produced. Tail latency is bounded by the resampler delay.
        if self.input_sample_rate == self.output_sample_rate {
            let out = std::mem::take(&mut self.pending);
            return Ok(out);
        }

        self.pending.resize(self.chunk_size, 0.0);
        let input_chunk: Vec<f32> = std::mem::take(&mut self.pending);
        let input_frames = vec![input_chunk];
        let mut output_frames = self
            .inner
            .process(&input_frames, None)
            .map_err(|e| AttuneError::Resampler(format!("flush failed: {e}")))?;
        Ok(std::mem::take(&mut output_frames[0]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_through_when_rates_match() {
        let mut r = StreamingResampler::new(16_000, 1, 16_000).unwrap();
        let input: Vec<f32> = (0..2048).map(|i| (i as f32 / 100.0).sin()).collect();
        let out = r.process(&input).unwrap();
        assert_eq!(out.len(), input.len());
        for (a, b) in input.iter().zip(out.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn downmixes_stereo_to_mono() {
        let mut r = StreamingResampler::new(16_000, 2, 16_000).unwrap();
        // Stereo interleaved: L=1.0, R=-1.0 → mono 0.0.
        let input: Vec<f32> = (0..1024).flat_map(|_| [1.0_f32, -1.0]).collect();
        let out = r.process(&input).unwrap();
        assert_eq!(out.len(), 1024);
        for sample in out {
            assert!(sample.abs() < 1e-6);
        }
    }

    #[test]
    fn downsamples_48k_to_16k_roughly_one_third() {
        let mut r = StreamingResampler::new(48_000, 1, 16_000).unwrap();
        let mut all_output = Vec::new();
        // Feed 48000 samples in 1024-sample chunks.
        let total_input = 48_000;
        let mut produced_before_flush = 0;
        for chunk_start in (0..total_input).step_by(1024) {
            let chunk_end = (chunk_start + 1024).min(total_input);
            let chunk: Vec<f32> = (chunk_start..chunk_end)
                .map(|i| (i as f32 / 50.0).sin())
                .collect();
            let mut out = r.process(&chunk).unwrap();
            produced_before_flush += out.len();
            all_output.append(&mut out);
        }
        let mut tail = r.flush().unwrap();
        all_output.append(&mut tail);

        // 48000 input frames → roughly 16000 output frames. Polyphase resamplers
        // introduce a small delay so we accept ±5% deviation.
        let lower = (16_000.0_f32 * 0.95) as usize;
        let upper = (16_000.0_f32 * 1.05) as usize;
        assert!(
            all_output.len() >= lower && all_output.len() <= upper,
            "expected ~16000 output samples, got {} (pre-flush {})",
            all_output.len(),
            produced_before_flush,
        );
    }
}
