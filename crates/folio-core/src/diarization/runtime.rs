use std::path::Path;

use hound::{SampleFormat, WavReader};
use sherpa_onnx::{
    FastClusteringConfig, OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig,
    OfflineSpeakerSegmentationModelConfig, OfflineSpeakerSegmentationPyannoteModelConfig,
    SpeakerEmbeddingExtractorConfig,
};
use thiserror::Error;

use crate::audio::resampler::StreamingResampler;
use crate::diarization::models::DiarizationModelStore;

#[derive(Debug, Error)]
pub enum DiarizationError {
    #[error(
        "required diarization models are not on disk; call DiarizationModelStore::download first"
    )]
    ModelsNotDownloaded,
    #[error("sherpa-onnx diarization failed: {0}")]
    Runtime(String),
    #[error("wav: {0}")]
    Wav(String),
    #[error("resample: {0}")]
    Resample(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiarizedSegment {
    pub start_secs: f32,
    pub end_secs: f32,
    pub speaker: i32,
}

#[derive(Debug, Clone)]
pub struct DiarizationOptions {
    pub num_speakers: i32,

    pub threshold: f32,

    pub num_threads: i32,

    pub min_duration_on: f32,
    pub min_duration_off: f32,
}

impl Default for DiarizationOptions {
    fn default() -> Self {
        Self {
            num_speakers: 0,
            threshold: 0.4,
            num_threads: 2,
            min_duration_on: 0.7,
            min_duration_off: 0.5,
        }
    }
}

pub struct DiarizationRuntime {
    sd: OfflineSpeakerDiarization,
}

impl std::fmt::Debug for DiarizationRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiarizationRuntime")
            .field("sample_rate", &self.sample_rate())
            .finish()
    }
}

impl DiarizationRuntime {
    pub fn open(
        segmentation_model: &Path,
        embedding_model: &Path,
        opts: &DiarizationOptions,
    ) -> Result<Self, DiarizationError> {
        let config = OfflineSpeakerDiarizationConfig {
            segmentation: OfflineSpeakerSegmentationModelConfig {
                pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                    model: Some(segmentation_model.to_string_lossy().into_owned()),
                },
                num_threads: opts.num_threads,
                debug: false,
                provider: None,
            },
            embedding: SpeakerEmbeddingExtractorConfig {
                model: Some(embedding_model.to_string_lossy().into_owned()),
                num_threads: opts.num_threads,
                debug: false,
                provider: None,
            },
            clustering: FastClusteringConfig {
                num_clusters: opts.num_speakers,
                threshold: opts.threshold,
            },
            min_duration_on: opts.min_duration_on,
            min_duration_off: opts.min_duration_off,
        };
        let sd = OfflineSpeakerDiarization::create(&config).ok_or_else(|| {
            DiarizationError::Runtime(format!(
                "failed to create diarizer (segmentation={}, embedding={})",
                segmentation_model.display(),
                embedding_model.display()
            ))
        })?;
        Ok(Self { sd })
    }

    pub fn from_store(
        store: &DiarizationModelStore,
        opts: &DiarizationOptions,
    ) -> Result<Self, DiarizationError> {
        use crate::diarization::models::DiarizationModel;
        if !store.is_ready() {
            return Err(DiarizationError::ModelsNotDownloaded);
        }
        Self::open(
            &store.path_for(DiarizationModel::Segmentation),
            &store.path_for(DiarizationModel::EmbeddingResnet34Lm),
            opts,
        )
    }

    pub fn sample_rate(&self) -> u32 {
        self.sd.sample_rate().max(0) as u32
    }

    pub fn diarize_samples(
        &self,
        samples: &[f32],
    ) -> Result<Vec<DiarizedSegment>, DiarizationError> {
        let result = self
            .sd
            .process(samples)
            .ok_or_else(|| DiarizationError::Runtime("process returned no result".into()))?;
        Ok(result
            .sort_by_start_time()
            .into_iter()
            .map(|s| DiarizedSegment {
                start_secs: s.start,
                end_secs: s.end,
                speaker: s.speaker,
            })
            .collect())
    }

    pub fn diarize_wav(&self, path: &Path) -> Result<Vec<DiarizedSegment>, DiarizationError> {
        let samples = read_wav_as_mono(path, self.sample_rate())?;
        self.diarize_samples(&samples)
    }
}

pub fn assign_speakers_by_overlap(
    spans: &[(f64, f64)],
    diarized: &[DiarizedSegment],
) -> Vec<Option<i32>> {
    spans
        .iter()
        .map(|&(start, end)| {
            let mut best: Option<(i32, f64)> = None;
            for d in diarized {
                let overlap =
                    (end.min(d.end_secs as f64) - start.max(d.start_secs as f64)).max(0.0);
                if overlap > 0.0 && best.is_none_or(|(_, bo)| overlap > bo) {
                    best = Some((d.speaker, overlap));
                }
            }
            if let Some((spk, _)) = best {
                return Some(spk);
            }

            let mid = (start + end) / 2.0;
            diarized
                .iter()
                .min_by(|a, b| {
                    let da = (((a.start_secs + a.end_secs) as f64) / 2.0 - mid).abs();
                    let db = (((b.start_secs + b.end_secs) as f64) / 2.0 - mid).abs();
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|d| d.speaker)
        })
        .collect()
}

pub(crate) fn read_wav_as_mono(
    path: &Path,
    target_rate: u32,
) -> Result<Vec<f32>, DiarizationError> {
    let reader = WavReader::open(path)
        .map_err(|e| DiarizationError::Wav(format!("open {}: {e}", path.display())))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;
    let interleaved = read_interleaved_f32(reader)?;
    let mono = if channels <= 1 {
        interleaved
    } else {
        let frames = interleaved.len() / channels;
        let mut out = Vec::with_capacity(frames);
        for frame in 0..frames {
            let sum: f32 = (0..channels)
                .map(|c| interleaved[frame * channels + c])
                .sum();
            out.push(sum / channels as f32);
        }
        out
    };
    if spec.sample_rate == target_rate || target_rate == 0 {
        return Ok(mono);
    }
    let mut rs = StreamingResampler::new(spec.sample_rate, 1, target_rate)
        .map_err(|e| DiarizationError::Resample(e.to_string()))?;
    let mut out = rs
        .process(&mono)
        .map_err(|e| DiarizationError::Resample(e.to_string()))?;
    out.extend(
        rs.flush()
            .map_err(|e| DiarizationError::Resample(e.to_string()))?,
    );
    Ok(out)
}

fn read_interleaved_f32<R: std::io::Read>(
    reader: WavReader<R>,
) -> Result<Vec<f32>, DiarizationError> {
    let spec = reader.spec();
    let mut out: Vec<f32> = Vec::with_capacity(reader.len() as usize);
    match spec.sample_format {
        SampleFormat::Float => {
            for s in reader.into_samples::<f32>() {
                out.push(s.map_err(|e| DiarizationError::Wav(format!("read: {e}")))?);
            }
        }
        SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            if !(8..=32).contains(&bits) {
                return Err(DiarizationError::Wav(format!(
                    "unsupported bit depth: {bits}"
                )));
            }
            let max = (1i64 << (bits - 1)) as f32;
            for s in reader.into_samples::<i32>() {
                let raw = s.map_err(|e| DiarizationError::Wav(format!("read: {e}")))?;
                out.push(raw as f32 / max);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diarization::models::DiarizationModelStore;

    #[test]
    fn from_store_errors_when_models_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = DiarizationModelStore::new(dir.path());
        let err = DiarizationRuntime::from_store(&store, &DiarizationOptions::default())
            .expect_err("missing models should fail");
        assert!(matches!(err, DiarizationError::ModelsNotDownloaded));
    }

    #[test]
    fn default_options_auto_estimate_speakers() {
        let o = DiarizationOptions::default();
        assert_eq!(o.num_speakers, 0);
        assert!(o.threshold > 0.0);
    }
}
