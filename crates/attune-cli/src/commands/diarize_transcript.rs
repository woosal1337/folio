use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Context, Result};

use attune_core::diarization::{
    assign_speakers_by_overlap, label_system_channel, DiarizationOptions, DiarizationOutcome,
    DiarizationRuntime,
};
use attune_core::storage::session::TRANSCRIPT_FILENAME;
use attune_core::transcription::SessionTranscript;

use crate::cli::DiarizeTranscriptArgs;

pub fn run(args: DiarizeTranscriptArgs) -> Result<()> {
    let dir = &args.session_dir;
    if !dir.is_dir() {
        bail!("not a session directory: {}", dir.display());
    }
    let transcript_path = dir.join(TRANSCRIPT_FILENAME);
    let mut transcript = SessionTranscript::read_json(&transcript_path)
        .with_context(|| format!("reading transcript in {}", dir.display()))?;

    let opts = DiarizationOptions {
        num_speakers: args.num_speakers,
        threshold: args.threshold,
        ..Default::default()
    };

    let outcome = match (&args.segmentation, &args.embedding) {
        (Some(seg), Some(emb)) => label_with_models(dir, &mut transcript, seg, emb, &opts)?,
        _ => {
            label_system_channel(dir, &mut transcript, &opts).context("diarizing system channel")?
        }
    };

    transcript
        .write_json(&transcript_path)
        .with_context(|| format!("writing transcript {}", transcript_path.display()))?;

    println!(
        "labelled {} of {} system segments across {} speakers",
        outcome.num_labeled, outcome.num_segments, outcome.num_speakers
    );
    println!("updated {}", transcript_path.display());
    println!("re-open the note in attune to see Speaker 1/2/3…");
    Ok(())
}

fn label_with_models(
    dir: &Path,
    transcript: &mut SessionTranscript,
    seg: &Path,
    emb: &Path,
    opts: &DiarizationOptions,
) -> Result<DiarizationOutcome> {
    let runtime = DiarizationRuntime::open(seg, emb, opts).context("creating the diarizer")?;
    let system_wav = dir.join("system.wav");
    if !system_wav.is_file() {
        bail!("no system.wav in {}", dir.display());
    }
    let diarized = runtime
        .diarize_wav(&system_wav)
        .context("diarizing system.wav")?;

    let mut speakers: BTreeSet<i32> = BTreeSet::new();
    let mut outcome = DiarizationOutcome::default();
    for channel in transcript
        .channels
        .iter_mut()
        .filter(|c| c.channel == "system")
    {
        outcome.num_segments += channel.segments.len();
        let spans: Vec<(f64, f64)> = channel
            .segments
            .iter()
            .map(|s| (s.start_seconds, s.end_seconds))
            .collect();
        for (s, spk) in channel
            .segments
            .iter_mut()
            .zip(assign_speakers_by_overlap(&spans, &diarized))
        {
            s.speaker = spk;
            if let Some(x) = spk {
                speakers.insert(x);
                outcome.num_labeled += 1;
            }
        }
    }
    outcome.num_speakers = speakers.len();
    Ok(outcome)
}
