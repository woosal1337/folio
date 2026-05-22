//! WAV file writer for captured audio.
//!
//! Mono 16-bit PCM at the target sample rate (16 kHz by default). Files are
//! written incrementally during capture and finalized atomically on close.

use std::path::Path;
use std::sync::Mutex;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::error::{AttuneError, Result};

/// Thread-safe wrapper around `hound::WavWriter`. Capture callbacks lock,
/// write, and release per buffer. The lock is contended only between the
/// callback thread and the finalize call.
pub struct AudioWavWriter {
    inner: Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>,
    sample_rate: u32,
    samples_written: parking_lot::Mutex<u64>,
}

impl AudioWavWriter {
    /// Create a new mono 16-bit PCM WAV writer at `path`. The file is created
    /// or truncated.
    pub fn create<P: AsRef<Path>>(path: P, sample_rate: u32) -> Result<Self> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let writer = WavWriter::create(path, spec)?;
        Ok(Self {
            inner: Mutex::new(Some(writer)),
            sample_rate,
            samples_written: parking_lot::Mutex::new(0),
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn samples_written(&self) -> u64 {
        *self.samples_written.lock()
    }

    /// Append mono float samples. Values are clamped to [-1.0, 1.0] then
    /// quantized to int16. No-op if the writer has been finalized.
    pub fn append(&self, samples: &[f32]) -> Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| AttuneError::WavWriter(format!("poisoned mutex: {e}")))?;
        let Some(writer) = guard.as_mut() else {
            return Ok(());
        };
        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let int_sample = (clamped * i16::MAX as f32) as i16;
            writer.write_sample(int_sample)?;
        }
        *self.samples_written.lock() += samples.len() as u64;
        Ok(())
    }

    /// Finalize the WAV file. Subsequent writes are silently dropped.
    pub fn finalize(&self) -> Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| AttuneError::WavWriter(format!("poisoned mutex: {e}")))?;
        if let Some(writer) = guard.take() {
            writer.finalize()?;
        }
        Ok(())
    }
}

impl Drop for AudioWavWriter {
    fn drop(&mut self) {
        let _ = self.finalize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_silent_wav() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("silent.wav");
        let w = AudioWavWriter::create(&path, 16_000).unwrap();
        w.append(&vec![0.0_f32; 16_000]).unwrap();
        w.finalize().unwrap();
        assert_eq!(w.samples_written(), 16_000);

        let reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 16_000);
        assert_eq!(spec.bits_per_sample, 16);
    }

    #[test]
    fn clamps_out_of_range_samples() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clamped.wav");
        let w = AudioWavWriter::create(&path, 16_000).unwrap();
        w.append(&[2.0, -2.0, 0.5, -0.5]).unwrap();
        w.finalize().unwrap();

        let mut reader = hound::WavReader::open(&path).unwrap();
        let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(samples.len(), 4);
        assert_eq!(samples[0], i16::MAX);
        assert_eq!(samples[1], -i16::MAX);
        // 0.5 * 32767 = 16383.5 → 16383 (truncation)
        assert!((samples[2] as i32 - 16_383).abs() <= 1);
        assert!((samples[3] as i32 + 16_383).abs() <= 1);
    }

    #[test]
    fn append_after_finalize_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("finalized.wav");
        let w = AudioWavWriter::create(&path, 16_000).unwrap();
        w.append(&[0.1; 10]).unwrap();
        w.finalize().unwrap();
        // Should not error, should not write.
        w.append(&[0.5; 10]).unwrap();
        assert_eq!(w.samples_written(), 10);
    }
}
