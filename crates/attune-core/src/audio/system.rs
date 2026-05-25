//! System audio capture via Apple's ScreenCaptureKit.
//!
//! Wraps `screencapturekit::SCStream` configured with `captures_audio = true`
//! and `excludes_current_process_audio = true`. The first display on the
//! system is used as the content source. No video frames are requested or
//! processed — we only care about the audio output. macOS still prompts for
//! Screen Recording permission the first time, even for audio-only capture.
//!
//! Sample buffers arrive via an `SCStreamOutputTrait` impl on a SCK-owned
//! thread. Each `CMSampleBuffer` exposes an `AudioBufferList` containing one
//! or more `AudioBuffer`s of interleaved `f32` PCM. We downmix to mono,
//! resample to the target rate via [`StreamingResampler`], and append to
//! the WAV writer.
//!
//! Future: CoreAudio HAL Tap (macOS 14.4+) avoids the Screen Recording
//! permission entirely. See `architecture/audio-capture.md` in the design
//! vault. We keep this implementation behind a trait-friendly shape so
//! swapping the backend is mechanical.

use std::sync::Arc;

use parking_lot::Mutex;
use tracing::{debug, error, info};

use crate::audio::resampler::StreamingResampler;
use crate::audio::wav_writer::AudioWavWriter;
use crate::error::{AttuneError, Result};
#[cfg(target_os = "macos")]
use crate::qos::{set_thread_qos, QosClass};

#[cfg(target_os = "macos")]
pub use macos_impl::SystemCapture;

#[cfg(not(target_os = "macos"))]
pub use stub_impl::SystemCapture;

/// Source sample rate requested from ScreenCaptureKit. The framework will
/// downmix / convert as needed. 48 kHz is the macOS hardware native rate
/// and avoids unnecessary OS-side resampling.
const SCK_SAMPLE_RATE: u32 = 48_000;
const SCK_CHANNEL_COUNT: u8 = 1;

#[cfg(target_os = "macos")]
mod macos_impl {
    use super::*;

    use core_media_rs::cm_sample_buffer::CMSampleBuffer;
    use screencapturekit::shareable_content::SCShareableContent;
    use screencapturekit::stream::configuration::SCStreamConfiguration;
    use screencapturekit::stream::content_filter::SCContentFilter;
    use screencapturekit::stream::output_trait::SCStreamOutputTrait;
    use screencapturekit::stream::output_type::SCStreamOutputType;
    use screencapturekit::stream::SCStream;

    /// Captures system audio on macOS via ScreenCaptureKit.
    pub struct SystemCapture {
        stream: Option<SCStream>,
        writer: Arc<AudioWavWriter>,
    }

    /// SCStream callback target. Holds the shared resampler + writer.
    /// Runs on SCK's audio thread.
    struct AudioOutput {
        writer: Arc<AudioWavWriter>,
        resampler: Arc<Mutex<StreamingResampler>>,
    }

    thread_local! {
        /// Set once per SCK callback thread; pthread_set_qos_class_self_np
        /// is per-thread so we tag on the first sample frame and skip
        /// the libc syscall every subsequent frame. v2 finding 064 / GET-99.
        static QOS_TAGGED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    impl SCStreamOutputTrait for AudioOutput {
        fn did_output_sample_buffer(
            &self,
            sample_buffer: CMSampleBuffer,
            of_type: SCStreamOutputType,
        ) {
            QOS_TAGGED.with(|cell| {
                if !cell.get() {
                    set_thread_qos(QosClass::UserInteractive);
                    cell.set(true);
                }
            });
            if of_type != SCStreamOutputType::Audio {
                return;
            }
            let abl = match sample_buffer.get_audio_buffer_list() {
                Ok(a) => a,
                Err(e) => {
                    error!(?e, "could not get audio buffer list from sample");
                    return;
                }
            };

            let num_buffers = abl.num_buffers();
            if num_buffers == 0 {
                return;
            }

            // ScreenCaptureKit on macOS delivers either:
            //   (a) one interleaved AudioBuffer with N channels, or
            //   (b) N deinterleaved AudioBuffers each with 1 channel.
            // We detect by inspecting the first buffer's channel count. With
            // SCK_CHANNEL_COUNT = 1 we expect (a) with channels=1, but we
            // handle both cases.
            let first = match abl.get(0) {
                Some(b) => b,
                None => return,
            };
            let first_channels = first.number_channels as usize;
            let first_bytes = first.data();
            if first_bytes.is_empty() || first_bytes.len() % 4 != 0 {
                return;
            }

            // f32 PCM interleaved, little-endian native order.
            let mono: Vec<f32> = if num_buffers == 1 {
                // Case (a). Interleaved already (channels=first_channels).
                interleaved_to_mono(first_bytes, first_channels.max(1))
            } else {
                // Case (b). Deinterleaved across N buffers, one channel each.
                deinterleaved_to_mono(&abl, num_buffers)
            };

            if mono.is_empty() {
                return;
            }

            let resampled = {
                let mut guard = self.resampler.lock();
                // Feed as if input_channels=1 since we already downmixed.
                match guard.process(&mono) {
                    Ok(out) => out,
                    Err(e) => {
                        error!(error = %e, "system audio resampler failed");
                        return;
                    }
                }
            };
            if let Err(e) = self.writer.append(&resampled) {
                error!(error = %e, "system audio wav append failed");
            }
        }
    }

    fn interleaved_to_mono(bytes: &[u8], channels: usize) -> Vec<f32> {
        if channels == 0 {
            return Vec::new();
        }
        let total_samples = bytes.len() / 4;
        let frames = total_samples / channels;
        let mut out = Vec::with_capacity(frames);
        for frame in 0..frames {
            let mut sum = 0.0_f32;
            for c in 0..channels {
                let idx = (frame * channels + c) * 4;
                let s = f32::from_le_bytes([
                    bytes[idx],
                    bytes[idx + 1],
                    bytes[idx + 2],
                    bytes[idx + 3],
                ]);
                sum += s;
            }
            out.push(sum / channels as f32);
        }
        out
    }

    fn deinterleaved_to_mono(
        abl: &core_audio_types_rs::audio_buffer_list::AudioBufferList,
        num_buffers: usize,
    ) -> Vec<f32> {
        // Each buffer is one channel. Average frame-by-frame across buffers.
        // Determine the minimum number of frames in case buffers differ
        // (they shouldn't, but be defensive).
        let mut min_frames = usize::MAX;
        for i in 0..num_buffers {
            if let Some(b) = abl.get(i) {
                let frames = b.data().len() / 4;
                if frames < min_frames {
                    min_frames = frames;
                }
            }
        }
        if min_frames == usize::MAX {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(min_frames);
        for frame in 0..min_frames {
            let mut sum = 0.0_f32;
            for i in 0..num_buffers {
                if let Some(b) = abl.get(i) {
                    let bytes = b.data();
                    let idx = frame * 4;
                    let s = f32::from_le_bytes([
                        bytes[idx],
                        bytes[idx + 1],
                        bytes[idx + 2],
                        bytes[idx + 3],
                    ]);
                    sum += s;
                }
            }
            out.push(sum / num_buffers as f32);
        }
        out
    }

    impl SystemCapture {
        pub fn start(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
            let content = SCShareableContent::get().map_err(|e| {
                AttuneError::SystemAudio(format!(
                    "could not enumerate shareable content (Screen Recording permission may be missing): {:?}",
                    e
                ))
            })?;
            let display = content
                .displays()
                .into_iter()
                .next()
                .ok_or_else(|| AttuneError::SystemAudio("no display available".into()))?;

            let config = SCStreamConfiguration::new()
                .set_captures_audio(true)
                .map_err(|e| AttuneError::SystemAudio(format!("captures_audio: {:?}", e)))?
                .set_excludes_current_process_audio(true)
                .map_err(|e| {
                    AttuneError::SystemAudio(format!("excludes_current_process_audio: {:?}", e))
                })?
                .set_sample_rate(SCK_SAMPLE_RATE)
                .map_err(|e| AttuneError::SystemAudio(format!("sample_rate: {:?}", e)))?
                .set_channel_count(SCK_CHANNEL_COUNT)
                .map_err(|e| AttuneError::SystemAudio(format!("channel_count: {:?}", e)))?;

            let filter = SCContentFilter::new().with_display_excluding_windows(&display, &[]);

            let resampler = Arc::new(Mutex::new(StreamingResampler::new(
                SCK_SAMPLE_RATE,
                1,
                target_sample_rate,
            )?));
            let output = AudioOutput {
                writer: writer.clone(),
                resampler,
            };

            let mut stream = SCStream::new(&filter, &config);
            stream.add_output_handler(output, SCStreamOutputType::Audio);

            stream
                .start_capture()
                .map_err(|e| AttuneError::SystemAudio(format!("start_capture: {:?}", e)))?;

            info!(
                sample_rate = SCK_SAMPLE_RATE,
                channels = SCK_CHANNEL_COUNT,
                "ScreenCaptureKit audio stream started"
            );

            Ok(Self {
                stream: Some(stream),
                writer,
            })
        }

        pub fn stop(mut self) -> Result<()> {
            if let Some(stream) = self.stream.take() {
                if let Err(e) = stream.stop_capture() {
                    error!(error = ?e, "ScreenCaptureKit stop_capture returned error");
                }
                // Allow the audio thread to drain any in-flight buffer before
                // finalizing the WAV writer.
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            self.writer.finalize()?;
            debug!(
                samples = self.writer.samples_written(),
                "system audio capture finalized"
            );
            Ok(())
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod stub_impl {
    use super::*;

    pub struct SystemCapture {
        writer: Arc<AudioWavWriter>,
    }

    impl SystemCapture {
        pub fn start(_writer: Arc<AudioWavWriter>, _target_sample_rate: u32) -> Result<Self> {
            Err(AttuneError::SystemAudioUnsupported)
        }

        pub fn stop(self) -> Result<()> {
            self.writer.finalize()
        }
    }
}
