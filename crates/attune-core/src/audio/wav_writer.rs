use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};
use parking_lot::Mutex;

use crate::error::{AttuneError, Result};

pub fn concat_wavs(parts: &[std::path::PathBuf], out: &Path) -> Result<()> {
    let existing: Vec<&std::path::PathBuf> = parts.iter().filter(|p| p.exists()).collect();
    if existing.is_empty() {
        return Err(AttuneError::Storage("concat_wavs: no input parts".into()));
    }

    let first = hound::WavReader::open(existing[0])
        .map_err(|e| AttuneError::Storage(format!("open {}: {e}", existing[0].display())))?;
    let spec = first.spec();
    drop(first);

    let tmp = out.with_extension("merging.wav");
    {
        let mut writer = WavWriter::create(&tmp, spec)
            .map_err(|e| AttuneError::Storage(format!("create {}: {e}", tmp.display())))?;
        for part in &existing {
            let mut reader = hound::WavReader::open(part)
                .map_err(|e| AttuneError::Storage(format!("open {}: {e}", part.display())))?;
            if reader.spec().sample_rate != spec.sample_rate {
                return Err(AttuneError::Storage(format!(
                    "concat_wavs: {} is {} Hz, expected {} Hz",
                    part.display(),
                    reader.spec().sample_rate,
                    spec.sample_rate
                )));
            }
            for sample in reader.samples::<i16>() {
                let s = sample.map_err(|e| AttuneError::Storage(format!("read sample: {e}")))?;
                writer
                    .write_sample(s)
                    .map_err(|e| AttuneError::Storage(format!("write sample: {e}")))?;
            }
        }
        writer
            .finalize()
            .map_err(|e| AttuneError::Storage(format!("finalize {}: {e}", tmp.display())))?;
    }
    std::fs::rename(&tmp, out).map_err(|e| {
        AttuneError::Storage(format!(
            "rename {} -> {}: {e}",
            tmp.display(),
            out.display()
        ))
    })?;
    Ok(())
}

pub struct AudioWavWriter {
    inner: Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>,
    sample_rate: u32,
    samples_written: Mutex<u64>,
}

impl AudioWavWriter {
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

    pub fn append(&self, samples: &[f32]) -> Result<()> {
        let mut guard = self.inner.lock();
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

    pub fn finalize(&self) -> Result<()> {
        let mut guard = self.inner.lock();
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

        assert!((samples[2] as i32 - 16_383).abs() <= 1);
        assert!((samples[3] as i32 + 16_383).abs() <= 1);
    }

    #[test]
    fn concat_wavs_merges_parts_in_order_and_preserves_rate() {
        let dir = tempfile::tempdir().unwrap();
        let p0 = dir.path().join("p0.wav");
        let p1 = dir.path().join("p1.wav");
        let out = dir.path().join("merged.wav");

        let w0 = AudioWavWriter::create(&p0, 16_000).unwrap();
        w0.append(&[0.25_f32; 100]).unwrap();
        w0.finalize().unwrap();
        let w1 = AudioWavWriter::create(&p1, 16_000).unwrap();
        w1.append(&[0.5_f32; 200]).unwrap();
        w1.finalize().unwrap();

        concat_wavs(&[p0, p1], &out).unwrap();

        let reader = hound::WavReader::open(&out).unwrap();
        assert_eq!(reader.spec().sample_rate, 16_000);
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.len(), 300, "merged length is the sum of parts");
    }

    #[test]
    fn concat_wavs_skips_missing_parts() {
        let dir = tempfile::tempdir().unwrap();
        let present = dir.path().join("present.wav");
        let missing = dir.path().join("missing.wav");
        let out = dir.path().join("out.wav");
        let w = AudioWavWriter::create(&present, 16_000).unwrap();
        w.append(&[0.1_f32; 50]).unwrap();
        w.finalize().unwrap();

        concat_wavs(&[missing, present], &out).unwrap();
        let reader = hound::WavReader::open(&out).unwrap();
        assert_eq!(reader.len(), 50);
    }

    #[test]
    fn append_after_finalize_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("finalized.wav");
        let w = AudioWavWriter::create(&path, 16_000).unwrap();
        w.append(&[0.1; 10]).unwrap();
        w.finalize().unwrap();

        w.append(&[0.5; 10]).unwrap();
        assert_eq!(w.samples_written(), 10);
    }
}
