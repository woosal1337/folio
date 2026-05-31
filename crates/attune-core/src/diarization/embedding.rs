//! Per-speaker voice embeddings for the cross-recording speaker memory
//! (GET-189).
//!
//! The diarizer ([`super::runtime`]) tells us *which* cluster spoke when,
//! but not *who* — it returns labels, not embeddings. To remember a
//! speaker across recordings we need a stable [`EMBED_DIM`]-d vector per
//! cluster, computed with sherpa-onnx's WeSpeaker extractor (the same
//! embedding model the diarizer uses internally, loaded again here as a
//! standalone extractor).
//!
//! [`embed_speakers`] gathers up to a cap of each speaker's clearest audio
//! and produces one representative embedding per cluster. Those feed the
//! [`crate::speaker_memory`] registry: matched against it for auto-naming,
//! or stored into it when the user names a speaker.
//!
//! [`EMBED_DIM`]: crate::speaker_memory::EMBED_DIM

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;

use sherpa_onnx::{SpeakerEmbeddingExtractor, SpeakerEmbeddingExtractorConfig};

use crate::diarization::models::{DiarizationModel, DiarizationModelStore};
use crate::diarization::runtime::{DiarizationError, DiarizedSegment};

/// WeSpeaker expects 16 kHz mono — the same rate the pyannote segmentation
/// model runs at, so the diarizer's samples can be reused verbatim.
pub const EMBED_SAMPLE_RATE: u32 = 16_000;

/// Cap on the audio embedded per speaker. ~12 s of someone's clearest
/// speech is more than enough for a stable WeSpeaker embedding; the cap
/// keeps a speaker who talks for an hour from ballooning the buffer.
const MAX_SECONDS_PER_SPEAKER: f32 = 12.0;

/// Speakers with less than this much attributed audio are skipped — too
/// little signal to embed reliably, and a noisy embedding would poison the
/// registry's cosine math.
const MIN_SECONDS_PER_SPEAKER: f32 = 1.0;

/// Standalone WeSpeaker embedding extractor.
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
    /// Build an extractor from an explicit embedding-model path.
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

    /// Build an extractor from the model store. Errors with
    /// [`DiarizationError::ModelsNotDownloaded`] when the embedding model
    /// is missing.
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

    /// Embedding dimensionality reported by the model (256 for WeSpeaker
    /// ResNet34-LM).
    pub fn dim(&self) -> usize {
        self.extractor.dim().max(0) as usize
    }

    /// Compute an embedding for a 16 kHz mono chunk. Returns `None` when
    /// the extractor decides the chunk is too short to embed.
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

/// Compute one representative embedding per diarized speaker.
///
/// `samples_16k` must be the exact mono 16 kHz waveform the diarizer saw,
/// so a `DiarizedSegment`'s `[start_secs, end_secs]` maps to a sample
/// slice directly. For each speaker we concatenate up to
/// [`MAX_SECONDS_PER_SPEAKER`] of their audio, longest segments first
/// (the clearest, least fragmentary speech), and embed it. Speakers with
/// under [`MIN_SECONDS_PER_SPEAKER`] of audio — or whose chunk the
/// extractor rejects — are omitted from the result.
///
/// Returns a map of diarizer cluster id → embedding.
pub fn embed_speakers(
    embedder: &SpeakerEmbedder,
    samples_16k: &[f32],
    diarized: &[DiarizedSegment],
) -> BTreeMap<i32, Vec<f32>> {
    let total = samples_16k.len();
    let cap_samples = (MAX_SECONDS_PER_SPEAKER * EMBED_SAMPLE_RATE as f32) as usize;
    let min_samples = (MIN_SECONDS_PER_SPEAKER * EMBED_SAMPLE_RATE as f32) as usize;

    // Group segments by speaker.
    let mut by_speaker: BTreeMap<i32, Vec<&DiarizedSegment>> = BTreeMap::new();
    for d in diarized {
        by_speaker.entry(d.speaker).or_default().push(d);
    }

    let mut out = BTreeMap::new();
    for (speaker, mut segs) in by_speaker {
        // Longest segments first — the most representative speech.
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

/// Embed a whole mono 16 kHz waveform as a single speaker — used to derive
/// the user's "You" anchor from their mic track. Returns `None` when the
/// clip is too short. Caps the audio at [`MAX_SECONDS_PER_SPEAKER`].
pub fn embed_whole(embedder: &SpeakerEmbedder, samples_16k: &[f32]) -> Option<Vec<f32>> {
    let cap_samples = (MAX_SECONDS_PER_SPEAKER * EMBED_SAMPLE_RATE as f32) as usize;
    let min_samples = (MIN_SECONDS_PER_SPEAKER * EMBED_SAMPLE_RATE as f32) as usize;
    if samples_16k.len() < min_samples {
        return None;
    }
    let clip = &samples_16k[..samples_16k.len().min(cap_samples)];
    embedder.embed_chunk(clip)
}

/// Read a WAV file (any rate/layout), downmix to 16 kHz mono, and embed it
/// as a single speaker. Returns `None` when the clip is too short. Used to
/// anchor the user's "You" voice from their mic track.
pub fn embed_wav_file(
    embedder: &SpeakerEmbedder,
    path: &Path,
) -> Result<Option<Vec<f32>>, DiarizationError> {
    let samples = crate::diarization::runtime::read_wav_as_mono(path, EMBED_SAMPLE_RATE)?;
    Ok(embed_whole(embedder, &samples))
}
