use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;

use sherpa_onnx::{SpeakerEmbeddingExtractor, SpeakerEmbeddingExtractorConfig};

use crate::diarization::models::{DiarizationModel, DiarizationModelStore};
use crate::diarization::runtime::{DiarizationError, DiarizedSegment};

pub const EMBED_SAMPLE_RATE: u32 = 16_000;

const MAX_SECONDS_PER_SPEAKER: f32 = 12.0;

const MIN_SECONDS_PER_SPEAKER: f32 = 1.0;

pub struct SpeakerEmbedder {
    extractor: SpeakerEmbeddingExtractor,
}

impl std::fmt::Debug for SpeakerEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeakerEmbedder")
            .field("dim", &self.dim())
            .finish()
    }
}

impl SpeakerEmbedder {
    pub fn open(embedding_model: &Path, num_threads: i32) -> Result<Self, DiarizationError> {
        let config = SpeakerEmbeddingExtractorConfig {
            model: Some(embedding_model.to_string_lossy().into_owned()),
            num_threads,
            debug: false,
            provider: None,
        };
        let extractor = SpeakerEmbeddingExtractor::create(&config).ok_or_else(|| {
            DiarizationError::Runtime(format!(
                "failed to create speaker-embedding extractor ({})",
                embedding_model.display()
            ))
        })?;
        Ok(Self { extractor })
    }

    pub fn from_store(
        store: &DiarizationModelStore,
        num_threads: i32,
    ) -> Result<Self, DiarizationError> {
        if !store.is_ready() {
            return Err(DiarizationError::ModelsNotDownloaded);
        }
        Self::open(
            &store.path_for(DiarizationModel::EmbeddingResnet34Lm),
            num_threads,
        )
    }

    pub fn dim(&self) -> usize {
        self.extractor.dim().max(0) as usize
    }

    pub fn embed_chunk(&self, samples_16k: &[f32]) -> Option<Vec<f32>> {
        let stream = self.extractor.create_stream()?;
        stream.accept_waveform(EMBED_SAMPLE_RATE as i32, samples_16k);
        stream.input_finished();
        if !self.extractor.is_ready(&stream) {
            return None;
        }
        self.extractor.compute(&stream)
    }
}

pub fn embed_speakers(
    embedder: &SpeakerEmbedder,
    samples_16k: &[f32],
    diarized: &[DiarizedSegment],
) -> BTreeMap<i32, Vec<f32>> {
    let total = samples_16k.len();
    let cap_samples = (MAX_SECONDS_PER_SPEAKER * EMBED_SAMPLE_RATE as f32) as usize;
    let min_samples = (MIN_SECONDS_PER_SPEAKER * EMBED_SAMPLE_RATE as f32) as usize;

    let mut by_speaker: BTreeMap<i32, Vec<&DiarizedSegment>> = BTreeMap::new();
    for d in diarized {
        by_speaker.entry(d.speaker).or_default().push(d);
    }

    let mut out = BTreeMap::new();
    for (speaker, mut segs) in by_speaker {
        segs.sort_by(|a, b| {
            let da = a.end_secs - a.start_secs;
            let db = b.end_secs - b.start_secs;
            db.partial_cmp(&da).unwrap_or(Ordering::Equal)
        });

        let mut buf: Vec<f32> = Vec::new();
        for s in segs {
            let start = (s.start_secs.max(0.0) * EMBED_SAMPLE_RATE as f32) as usize;
            let end = ((s.end_secs.max(0.0) * EMBED_SAMPLE_RATE as f32) as usize).min(total);
            if start >= end {
                continue;
            }
            buf.extend_from_slice(&samples_16k[start..end]);
            if buf.len() >= cap_samples {
                buf.truncate(cap_samples);
                break;
            }
        }

        if buf.len() < min_samples {
            continue;
        }
        if let Some(emb) = embedder.embed_chunk(&buf) {
            out.insert(speaker, emb);
        }
    }
    out
}

pub fn embed_whole(embedder: &SpeakerEmbedder, samples_16k: &[f32]) -> Option<Vec<f32>> {
    let cap_samples = (MAX_SECONDS_PER_SPEAKER * EMBED_SAMPLE_RATE as f32) as usize;
    let min_samples = (MIN_SECONDS_PER_SPEAKER * EMBED_SAMPLE_RATE as f32) as usize;
    if samples_16k.len() < min_samples {
        return None;
    }
    let clip = &samples_16k[..samples_16k.len().min(cap_samples)];
    embedder.embed_chunk(clip)
}

pub fn embed_wav_file(
    embedder: &SpeakerEmbedder,
    path: &Path,
) -> Result<Option<Vec<f32>>, DiarizationError> {
    let samples = crate::diarization::runtime::read_wav_as_mono(path, EMBED_SAMPLE_RATE)?;
    Ok(embed_whole(embedder, &samples))
}
