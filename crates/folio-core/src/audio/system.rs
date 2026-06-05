use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use tracing::{debug, error, info, warn};

use crate::audio::resampler::StreamingResampler;
use crate::audio::wav_writer::AudioWavWriter;
use crate::error::{FolioError, Result};
#[cfg(target_os = "macos")]
use crate::qos::{set_thread_qos, QosClass};

#[cfg(target_os = "macos")]
pub use macos_impl::SystemCapture;

#[cfg(not(target_os = "macos"))]
pub use stub_impl::SystemCapture;

const SCK_SAMPLE_RATE: u32 = 48_000;
const SCK_CHANNEL_COUNT: u8 = 1;

const SILENCE_RMS_THRESHOLD: f32 = 0.002;

const SILENCE_PAUSE_AFTER_MS: u64 = 30_000;

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

    pub struct SystemCapture {
        inner: SystemCaptureInner,
        writer: Arc<AudioWavWriter>,
    }

    enum SystemCaptureInner {
        ProcessTap(crate::audio::process_tap::ProcessTapCapture),

        Sck(Option<SCStream>),
    }

    struct AudioOutput {
        writer: Arc<AudioWavWriter>,
        resampler: Arc<Mutex<StreamingResampler>>,

        last_active_ms: AtomicU64,

        paused: AtomicBool,
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    thread_local! {



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

            let first = match abl.get(0) {
                Some(b) => b,
                None => return,
            };
            let first_channels = first.number_channels as usize;
            let first_bytes = first.data();
            if first_bytes.is_empty() || first_bytes.len() % 4 != 0 {
                return;
            }

            let mono: Vec<f32> = if num_buffers == 1 {
                interleaved_to_mono(first_bytes, first_channels.max(1))
            } else {
                deinterleaved_to_mono(&abl, num_buffers)
            };

            if mono.is_empty() {
                return;
            }

            let buffer_rms = rms(&mono);
            let now = now_ms();
            let was_paused = self.paused.load(Ordering::Relaxed);
            if buffer_rms >= SILENCE_RMS_THRESHOLD {
                self.last_active_ms.store(now, Ordering::Relaxed);
                if was_paused {
                    self.paused.store(false, Ordering::Relaxed);
                    info!(
                        rms = buffer_rms,
                        threshold = SILENCE_RMS_THRESHOLD,
                        "system audio resumed — leaving silence pause"
                    );
                }
            } else {
                let last_active = self.last_active_ms.load(Ordering::Relaxed);
                let silent_for = now.saturating_sub(last_active);
                if silent_for >= SILENCE_PAUSE_AFTER_MS && !was_paused {
                    self.paused.store(true, Ordering::Relaxed);
                    info!(
                        silent_for_ms = silent_for,
                        threshold = SILENCE_RMS_THRESHOLD,
                        "system audio paused after sustained silence — skipping WAV writes until audio returns"
                    );
                }
            }

            if self.paused.load(Ordering::Relaxed) {
                return;
            }

            let resampled = {
                let mut guard = self.resampler.lock();

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
            if crate::audio::process_tap::is_supported() {
                match crate::audio::process_tap::ProcessTapCapture::start(
                    Arc::clone(&writer),
                    target_sample_rate,
                ) {
                    Ok(tap) => {
                        info!("system audio: using CoreAudio process tap (System Audio Recording Only)");
                        return Ok(Self {
                            inner: SystemCaptureInner::ProcessTap(tap),
                            writer,
                        });
                    }
                    Err(e) => {
                        warn!(error = %e, "process tap unavailable — falling back to ScreenCaptureKit");
                    }
                }
            }

            Self::start_sck(writer, target_sample_rate)
        }

        fn start_sck(writer: Arc<AudioWavWriter>, target_sample_rate: u32) -> Result<Self> {
            let content = SCShareableContent::get().map_err(|e| {
                FolioError::SystemAudio(format!(
                    "could not enumerate shareable content (Screen Recording permission may be missing): {:?}",
                    e
                ))
            })?;
            let display = content
                .displays()
                .into_iter()
                .next()
                .ok_or_else(|| FolioError::SystemAudio("no display available".into()))?;

            let config = SCStreamConfiguration::new()
                .set_captures_audio(true)
                .map_err(|e| FolioError::SystemAudio(format!("captures_audio: {:?}", e)))?
                .set_excludes_current_process_audio(true)
                .map_err(|e| {
                    FolioError::SystemAudio(format!("excludes_current_process_audio: {:?}", e))
                })?
                .set_sample_rate(SCK_SAMPLE_RATE)
                .map_err(|e| FolioError::SystemAudio(format!("sample_rate: {:?}", e)))?
                .set_channel_count(SCK_CHANNEL_COUNT)
                .map_err(|e| FolioError::SystemAudio(format!("channel_count: {:?}", e)))?;

            let filter = SCContentFilter::new().with_display_excluding_windows(&display, &[]);

            let resampler = Arc::new(Mutex::new(StreamingResampler::new(
                SCK_SAMPLE_RATE,
                1,
                target_sample_rate,
            )?));
            let output = AudioOutput {
                writer: writer.clone(),
                resampler,
                last_active_ms: AtomicU64::new(now_ms()),
                paused: AtomicBool::new(false),
            };

            let mut stream = SCStream::new(&filter, &config);
            stream.add_output_handler(output, SCStreamOutputType::Audio);
            stream
                .start_capture()
                .map_err(|e| FolioError::SystemAudio(format!("start_capture: {:?}", e)))?;

            info!(
                sample_rate = SCK_SAMPLE_RATE,
                channels = SCK_CHANNEL_COUNT,
                "ScreenCaptureKit audio stream started (Screen Recording permission)"
            );

            Ok(Self {
                inner: SystemCaptureInner::Sck(Some(stream)),
                writer,
            })
        }

        pub fn stop(mut self) -> Result<()> {
            match self.inner {
                SystemCaptureInner::ProcessTap(tap) => {
                    tap.stop()?;
                }
                SystemCaptureInner::Sck(ref mut opt) => {
                    if let Some(stream) = opt.take() {
                        if let Err(e) = stream.stop_capture() {
                            error!(error = ?e, "ScreenCaptureKit stop_capture returned error");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                    self.writer.finalize()?;
                }
            }
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
            Err(FolioError::SystemAudioUnsupported)
        }

        pub fn stop(self) -> Result<()> {
            self.writer.finalize()
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::macos_impl::*;

    fn rms_local(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    #[test]
    fn rms_of_empty_is_zero() {
        assert_eq!(rms_local(&[]), 0.0);
    }

    #[test]
    fn rms_of_silence_is_below_threshold() {
        let silent = vec![0.0_f32; 4096];
        assert!(rms_local(&silent) < 0.002);
    }

    #[test]
    fn rms_of_full_scale_sine_is_above_threshold() {
        let n = 48_000 / 1_000;
        let pcm: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / n as f32).sin())
            .collect();
        assert!(rms_local(&pcm) > 0.5);
    }

    #[allow(dead_code)]
    fn _api_present() {
        let _ = std::mem::size_of::<SystemCapture>();
    }
}
