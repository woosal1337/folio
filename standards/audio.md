# Audio Standards

Audio is the heart of Attune. This document captures the rules that keep
capture glitch-free, predictable, and portable to the iOS/macOS Whisper
pipeline that lands later.

## Real-time discipline

The platform audio callback runs on a real-time scheduled thread with a
fixed budget — typically 5–10 ms of wall clock per buffer of 256–1024
frames at 44.1–48 kHz. Missing that deadline drops samples. The rules:

1. **No allocations on the hot path.** Allocate at setup, reuse at run time.
   No `Vec::new()`, no `String::from`, no `Box::new`, no `format!` inside an
   audio callback or anything it calls transitively.
2. **No syscalls on the hot path.** No file I/O, no network, no
   `println!`. The WAV writer is locked-and-released per buffer; the lock
   is uncontested in steady state. Log lines emit only on state changes,
   not per buffer.
3. **No blocking locks.** Use `parking_lot::Mutex` (faster, never poisons)
   or a lock-free queue. The lock must never be held across more than a
   trivial amount of work.
4. **No panics.** Errors are logged via `tracing::error!` and the callback
   continues with the next buffer. A panicked callback unwinds into the
   platform audio thread — undefined behaviour territory.

Violations are caught by review, not by the compiler. The cost of getting
this wrong is audio dropouts users actually hear.

## Sample formats

- The capture pipeline normalises to `f32` interleaved as early as possible.
  `cpal` exposes `SampleFormat::F32`, `I16`, `U16`; we convert non-`f32`
  inputs to `f32` in the callback.
- The on-disk format is 16-bit signed PCM mono in WAV. The quantization
  step (`(sample * i16::MAX as f32) as i16`) clamps to `[-1.0, 1.0]` and
  rounds toward zero. Loudness or peak-aware quantization is out of scope
  for v0; we accept the ~0.003 dB error band.
- Whisper expects 16 kHz mono `f32`. The on-disk WAV files are written at
  the source's native rate (44.1, 48, etc.) unless `target_sample_rate`
  forces 16 kHz. The transcription pipeline resamples at consumption time
  in v1.

## Resampling

- `rubato::SincFixedIn<f32>` for streaming polyphase resampling. The
  parameters chosen in `resampler.rs` (`sinc_len = 256`, cubic interpolation,
  256× oversampling, Blackman-Harris 2-term window) trade ~3× CPU for
  meaningfully better quality in the 4–8 kHz speech band.
- Multichannel input is downmixed to mono *before* resampling: cheaper than
  resampling per channel, identical math.
- The chunk size is 1024 frames. Smaller chunks lower latency but raise
  per-chunk overhead; we chose 1024 because the audio callback delivers
  about that many frames per fire and matching means no extra buffering.
- The streaming resampler holds an internal `pending` buffer for partial
  chunks. The fast path (`input_sample_rate == output_sample_rate`) skips
  resampling entirely and just downmixes.
- On `flush`, partial chunks are zero-padded to the chunk size, processed,
  and emitted. The tail-latency cost is bounded by the chunk size.

## macOS capture sources

### Microphone (`cpal`)

`cpal` wraps CoreAudio's AudioUnit. Default input device, default config.
Configuration is read once at setup; per-callback work is the conversion
+ resampler + writer chain. The cpal callback owns the audio thread; we
never hold an Arc across `await`.

### System audio (`ScreenCaptureKit`)

- `SCStream` with `captures_audio = true` and
  `excludes_current_process_audio = true`. We don't request video frames.
- Source rate forced to 48 kHz, channel count 1. SCK does any mixing /
  resampling needed before we see the buffer.
- `CMSampleBuffer` audio comes either as one interleaved buffer or as N
  deinterleaved single-channel buffers. The detection is by number of
  `AudioBuffer`s in the `AudioBufferList`; both code paths exist and are
  unit-tested with synthetic input.
- Screen Recording permission is required on macOS 13+. The capture
  fails cleanly with `AttuneError::SystemAudio` if permission is missing;
  the orchestrator falls back to mic-only.
- The `Drop` order matters: drop the stream first, give the SCK audio
  thread a brief window (`sleep(200ms)`) to drain in-flight buffers, then
  finalize the WAV. Skipping the sleep can truncate the tail.

### CoreAudio HAL Tap (future)

- macOS 14.4+ exposes a HAL Tap API that captures system audio without the
  Screen Recording permission. We keep `SystemCapture` behind a
  trait-like shape so swapping the implementation is mechanical. Tracked
  in `architecture/audio-capture.md` in the design vault.

## WAV writing

- `hound` for PCM WAV. Mono, 16-bit, sample rate per source. The writer is
  wrapped in `Mutex<Option<WavWriter>>` so `finalize()` can be called
  exactly once even if the stream tries to flush later.
- The capture callback appends per-buffer. The mutex is uncontested in the
  common case (only the audio thread holds it; finalize runs after the
  stream is dropped).
- `samples_written` is tracked with an `AtomicU64`. The audio callback
  fetch-adds; the UI thread reads with `Ordering::Relaxed`. We do not need
  ordering with respect to other memory; just a count.
- `AudioWavWriter::drop` calls `finalize()` defensively. A double finalize
  is a no-op because `inner.take()` returns `None` the second time.

## Device enumeration

- `list_input_devices` runs on demand from the UI. It is not part of the
  hot path. Returns `Vec<DeviceInfo>` with name, default flag, default
  sample rate, default channel count. Sort: default first, then
  alphabetical.
- Names come from `cpal::Device::name()`, which can fail; on failure the
  device is labelled `<unknown>` but still appears in the list.

## Volume / clipping

- We do not apply gain. The user picks the device; we capture what it
  reports. Soft-clipping or limiter logic is out of scope.
- Quantization clamps to `[-1.0, 1.0]`. We do not detect or warn on
  clipping in v0; future versions should track peak in `samples_written`'s
  neighbour and surface clipping in the UI.

## Test strategy

- Synthetic signals: sine, silence, white noise via `(0..n).map(|i| ((i as f32) / k).sin())`.
- Resampler tests verify: pass-through when rates match, downmixing
  stereo→mono, downsample ratio 48k→16k within ±5%.
- WAV writer tests verify: silence round-trips through `hound::WavReader`,
  values outside `[-1, 1]` clamp at `±i16::MAX`, append after finalize is
  a silent no-op.
- Real-device tests are `#[ignore]` and only run manually because CI
  runners may not expose an audio device.

## Channel labelling

- `Channel::Microphone` writes to `mic.wav` and is labelled `me` in
  downstream transcripts.
- `Channel::System` writes to `system.wav` and is labelled `others`.
- Multi-speaker diarization on the system channel is a v1 feature; v0
  treats the entire system channel as a single "others" speaker.

## Cross-platform stance

- v0 ships macOS only. Non-macOS platforms get stub implementations that
  return `AttuneError::SystemAudioUnsupported`. The crate still compiles
  on Linux for CI ergonomics; the binary does not.
- When Windows/Linux capture lands, the public API does not change. The
  `cfg`-gated implementation choices are confined to `audio/system.rs`.
