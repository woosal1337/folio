//! System audio capture.
//!
//! v0 week 1: stub. Returns [`AttuneError::SystemAudioUnsupported`] on start.
//! The capture orchestrator falls back to mic-only if system capture is
//! unavailable, so the CLI remains usable.
//!
//! v0 week 2 (planned): ScreenCaptureKit-based capture via the
//! `screencapturekit` crate. SCStream configured with `captures_audio = true`,
//! `excludes_current_process_audio = true`, no video output. Samples arrive
//! via an `SCStreamOutput` handler implementing the audio buffer trait.
//!
//! v1 (planned): CoreAudio HAL Tap (macOS 14.4+) for system audio capture
//! without the Screen Recording permission requirement. See
//! `architecture/audio-capture.md` in the design vault.

use std::sync::Arc;

use tracing::warn;

use crate::audio::wav_writer::AudioWavWriter;
use crate::error::{AttuneError, Result};

pub struct SystemCapture {
    writer: Arc<AudioWavWriter>,
}

impl SystemCapture {
    /// Start system audio capture. Currently unimplemented on all platforms.
    /// See module docs for the planned implementation.
    pub fn start(writer: Arc<AudioWavWriter>, _target_sample_rate: u32) -> Result<Self> {
        warn!(
            "system audio capture not yet implemented in v0 week 1. \
             ScreenCaptureKit integration lands in week 2. \
             Mic-only capture continues."
        );
        // Returning the error lets the caller decide whether to abort or
        // continue with mic-only capture.
        let _ = writer;
        Err(AttuneError::SystemAudioUnsupported)
    }

    pub fn stop(self) -> Result<()> {
        self.writer.finalize()
    }
}

// ---------------------------------------------------------------------------
// Implementation notes for the week-2 ScreenCaptureKit integration.
// ---------------------------------------------------------------------------
//
// Pseudocode of the intended SCStream setup. Kept here so the next session has
// a starting point that matches the architecture docs.
//
// #[cfg(target_os = "macos")]
// fn build_sc_stream(writer: Arc<AudioWavWriter>, sample_rate: u32) -> Result<SCStream> {
//     use screencapturekit::sc_shareable_content::SCShareableContent;
//     use screencapturekit::sc_content_filter::{SCContentFilter, InitParams};
//     use screencapturekit::sc_stream_configuration::SCStreamConfiguration;
//     use screencapturekit::sc_stream::SCStream;
//
//     let content = SCShareableContent::current();
//     let primary_display = content.displays.first().cloned()
//         .ok_or_else(|| AttuneError::SystemAudio("no display".into()))?;
//     let filter = SCContentFilter::new(InitParams::Display(primary_display));
//
//     let mut config = SCStreamConfiguration::default();
//     config.set_captures_audio(true);
//     config.set_excludes_current_process_audio(true);
//     config.set_sample_rate(sample_rate as i64);
//
//     let mut stream = SCStream::new(filter, config, /* delegate */ ...);
//     stream.add_output(/* SCStreamOutputType::Audio, audio handler */);
//     stream.start_capture()
//         .map_err(|e| AttuneError::SystemAudio(format!("start: {e}")))?;
//     Ok(stream)
// }
//
// The audio handler receives CMSampleBuffers. Extract the AudioBufferList,
// downmix to mono if stereo, resample to 16 kHz via StreamingResampler, write
// to the WAV file.
