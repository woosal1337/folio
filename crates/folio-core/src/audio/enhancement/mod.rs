mod rnnoise;

use std::path::Path;
use std::time::Instant;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

use crate::audio::resampler::StreamingResampler;

pub const MODEL_SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone, Copy)]
pub struct EnhancementConfig {
    pub atten_lim_db: f32,
}

impl Default for EnhancementConfig {
    fn default() -> Self {
        Self {
            atten_lim_db: -20.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnhanceStats {
    pub input_samples: u64,

    pub output_samples: u64,

    pub input_rms: f32,

    pub output_rms: f32,

    pub processing_secs: f64,

    pub audio_secs: f64,
}

impl EnhanceStats {
    pub fn rtf(&self) -> f64 {
        if self.audio_secs > 0.0 {
            self.processing_secs / self.audio_secs
        } else {
            0.0
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnhancementError {
    #[error("wav io: {0}")]
    Wav(String),

    #[error("resample: {0}")]
    Resample(String),

    #[error("enhancement backend: {0}")]
    Backend(String),
}

pub fn enhance_wav_file(
    input: &Path,
    output: &Path,
    config: &EnhancementConfig,
) -> Result<EnhanceStats, EnhancementError> {
    let (mono_48k, source_format, audio_secs) = read_wav_as_48k_mono(input)?;

    let input_rms = rms(&mono_48k);
    let t0 = Instant::now();
    let enhanced = run_backend(&mono_48k, config)?;
    let processing_secs = t0.elapsed().as_secs_f64();
    let output_rms = rms(&enhanced);

    write_mono_48k_wav(output, &enhanced, source_format)?;

    Ok(EnhanceStats {
        input_samples: mono_48k.len() as u64,
        output_samples: enhanced.len() as u64,
        input_rms,
        output_rms,
        processing_secs,
        audio_secs,
    })
}

fn run_backend(
    samples_48k_mono: &[f32],
    config: &EnhancementConfig,
) -> Result<Vec<f32>, EnhancementError> {
    rnnoise::enhance(samples_48k_mono, config.atten_lim_db).map_err(EnhancementError::Backend)
}

#[derive(Debug, Clone, Copy)]
struct SourceFormat {
    sample_format: SampleFormat,
    bits_per_sample: u16,
}

fn read_wav_as_48k_mono(path: &Path) -> Result<(Vec<f32>, SourceFormat, f64), EnhancementError> {
    let reader = WavReader::open(path)
        .map_err(|e| EnhancementError::Wav(format!("open {}: {e}", path.display())))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;
    let interleaved = read_interleaved_f32(reader)?;
    let mono = downmix_to_mono(&interleaved, channels);
    let audio_secs = if spec.sample_rate > 0 {
        mono.len() as f64 / spec.sample_rate as f64
    } else {
        0.0
    };

    let mono_48k = if spec.sample_rate == MODEL_SAMPLE_RATE {
        mono
    } else {
        let mut rs = StreamingResampler::new(spec.sample_rate, 1, MODEL_SAMPLE_RATE)
            .map_err(|e| EnhancementError::Resample(e.to_string()))?;
        let mut out = rs
            .process(&mono)
            .map_err(|e| EnhancementError::Resample(e.to_string()))?;
        out.extend(
            rs.flush()
                .map_err(|e| EnhancementError::Resample(e.to_string()))?,
        );
        out
    };

    Ok((
        mono_48k,
        SourceFormat {
            sample_format: spec.sample_format,
            bits_per_sample: spec.bits_per_sample,
        },
        audio_secs,
    ))
}

fn write_mono_48k_wav(
    path: &Path,
    samples: &[f32],
    format: SourceFormat,
) -> Result<(), EnhancementError> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: MODEL_SAMPLE_RATE,
        bits_per_sample: format.bits_per_sample,
        sample_format: format.sample_format,
    };
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| EnhancementError::Wav(format!("create {}: {e}", path.display())))?;
    match format.sample_format {
        SampleFormat::Float => {
            for &s in samples {
                writer
                    .write_sample(s)
                    .map_err(|e| EnhancementError::Wav(format!("write: {e}")))?;
            }
        }
        SampleFormat::Int => {
            let bits = format.bits_per_sample.max(1);
            let max = (1i64 << (bits - 1)) as f32;

            let lo = -(1i64 << (bits - 1)) as i32;
            let hi = ((1i64 << (bits - 1)) - 1) as i32;
            for &s in samples {
                let v = ((s.clamp(-1.0, 1.0) * max).round() as i32).clamp(lo, hi);
                writer
                    .write_sample(v)
                    .map_err(|e| EnhancementError::Wav(format!("write: {e}")))?;
            }
        }
    }
    writer
        .finalize()
        .map_err(|e| EnhancementError::Wav(format!("finalize {}: {e}", path.display())))?;
    Ok(())
}

fn read_interleaved_f32<R: std::io::Read>(
    reader: WavReader<R>,
) -> Result<Vec<f32>, EnhancementError> {
    let spec = reader.spec();
    let mut out: Vec<f32> = Vec::with_capacity(reader.len() as usize);
    match spec.sample_format {
        SampleFormat::Float => {
            for s in reader.into_samples::<f32>() {
                out.push(s.map_err(|e| EnhancementError::Wav(format!("read: {e}")))?);
            }
        }
        SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            if !(8..=32).contains(&bits) {
                return Err(EnhancementError::Wav(format!(
                    "unsupported PCM bit depth: {bits}"
                )));
            }
            let max = (1i64 << (bits - 1)) as f32;
            for s in reader.into_samples::<i32>() {
                let raw = s.map_err(|e| EnhancementError::Wav(format!("read: {e}")))?;
                out.push(raw as f32 / max);
            }
        }
    }
    Ok(out)
}

fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let frames = interleaved.len() / channels;
    let mut out = Vec::with_capacity(frames);
    for frame in 0..frames {
        let mut sum = 0.0f32;
        for c in 0..channels {
            sum += interleaved[frame * channels + c];
        }
        out.push(sum / channels as f32);
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
    use tempfile::TempDir;

    fn write_wav(path: &Path, rate: u32, channels: u16, samples: &[f32]) {
        let spec = WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut w = WavWriter::create(path, spec).unwrap();
        for &s in samples {
            w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn full_scale_input_round_trips_through_int_writeback() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("system.wav");

        let mut s = vec![1.0f32; 4800];
        for (i, v) in s.iter_mut().enumerate() {
            if i % 2 == 1 {
                *v = -1.0;
            }
        }
        write_wav(&input, 48_000, 1, &s);
        let out = dir.path().join("system.enhanced.wav");

        let cfg = EnhancementConfig { atten_lim_db: 0.0 };
        let stats = enhance_wav_file(&input, &out, &cfg).expect("must not TooWide");
        assert!(out.exists());
        assert_eq!(stats.output_samples, 4800);
    }

    #[test]
    fn downmix_averages_channels() {
        let inter = vec![1.0, 0.0, 1.0, 0.0];
        assert_eq!(downmix_to_mono(&inter, 2), vec![0.5, 0.5]);
    }

    #[test]
    fn downmix_passthrough_mono() {
        let mono = vec![0.1, 0.2, 0.3];
        assert_eq!(downmix_to_mono(&mono, 1), mono);
    }

    #[test]
    fn rms_of_silence_is_zero() {
        assert_eq!(rms(&vec![0.0; 1024]), 0.0);
        assert_eq!(rms(&[]), 0.0);
    }

    #[test]
    fn config_default_is_conservative() {
        assert_eq!(EnhancementConfig::default().atten_lim_db, -20.0);
    }

    #[test]
    fn enhances_a_real_signal_and_writes_48k_mono() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("system.wav");
        let n = 44_100 * 2;
        let mut stereo = Vec::with_capacity(n * 2);
        for i in 0..n {
            let t = i as f32 / 44_100.0;
            let tone = (2.0 * std::f32::consts::PI * 300.0 * t).sin() * 0.3;

            let noise = (((i * 1103515245 + 12345) % 1000) as f32 / 1000.0 - 0.5) * 0.1;
            let s = tone + noise;
            stereo.push(s);
            stereo.push(s);
        }
        write_wav(&input, 44_100, 2, &stereo);
        let out = dir.path().join("system.enhanced.wav");
        let stats = enhance_wav_file(&input, &out, &EnhancementConfig::default()).unwrap();
        assert!(out.exists());
        let r = WavReader::open(&out).unwrap();
        assert_eq!(r.spec().sample_rate, MODEL_SAMPLE_RATE);
        assert_eq!(r.spec().channels, 1);
        assert!((stats.audio_secs - 2.0).abs() < 0.05);

        assert!(
            (stats.output_samples as i64 - stats.input_samples as i64).abs() < 1024,
            "output {} vs input {}",
            stats.output_samples,
            stats.input_samples
        );
    }
}
