# Audio pipeline — Attune guidelines

Source-cited guidance for the Rust audio capture and processing stack: CoreAudio VoiceProcessingIO + cpal + ScreenCaptureKit on the producer side, `rubato` for sample-rate conversion, `hound` for WAV writing, feeding a Whisper transcriber that wants 16 kHz mono Float32.

§1 (realtime thread rules) is non-negotiable for any code that touches the audio callback. The rest are strong defaults — deviate only with a documented reason.

## TL;DR — the rules

1. **The realtime audio callback cannot allocate, lock, syscall, or panic into C.** Pre-allocate everything; communicate via lock-free SPSC ring + atomics. ([Bencina](http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing))
2. **`ringbuf::HeapRb<f32>` or `rtrb::RingBuffer<f32>` for callback → consumer.** Never `crossbeam::ArrayQueue` on the audio thread (CAS spin under contention).
3. **Resample on the consumer thread, not the callback.** `rubato::FftFixedIn` is the sweet spot for Whisper paths.
4. **Let the AudioUnit pick its sample rate; query and adapt.** Coercing format usually fails with `kAudioUnitErr_FormatNotSupported` or silent stream death.
5. **Pick the louder of mic vs system per frame, don't mix.** Already what Attune does in `b6f3fb0` for transcription clarity.
6. **`hound::WavWriter::finalize()` must be called explicitly.** Drop-finalize silently swallows errors.
7. **Test through a `Sink` trait with fixture WAVs and RMS-bounded assertions.** Hardware-dependent tests are flaky-or-fake.

## §1 Realtime audio thread rules

The audio callback runs on a high-priority OS thread driven by the hardware clock. If you don't deliver the next buffer in time, the user hears a glitch. There is no retry. From Ross Bencina's canonical [Real-time audio programming 101](http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing):

> "If you don't know how long it will take, don't do it."

A frame budget at 48 kHz / 128 frames is ~2.6 ms. The allocator can take milliseconds under pressure. Therefore:

**Forbidden in the callback:**

- **No heap alloc/dealloc.** Anything with unbounded worst-case is out.
- **No locks.** `Mutex`, `RwLock`, `parking_lot` — any blocking primitive can priority-invert and stall the callback. See [timur.audio](https://timur.audio/using-locks-in-real-time-audio-processing-safely) for the narrow exceptions.
- **No syscalls.** No `println!`, no `tracing::info!` going through a file appender, no `File::write`, no `SystemTime::now()` on some platforms, no socket I/O. See Bencina's follow-up [Interfacing Real-Time Audio and File I/O](http://www.rossbencina.com/code/interfacing-real-time-audio-and-file-io).
- **No `Arc::clone` whose drop runs on the audio thread.** The last drop frees memory. Use the [basedrop](https://micahrj.github.io/posts/basedrop/) pattern: deferred drops sent to a collector thread.
- **No unbounded loops, recursion with data-dependent depth, `Vec::push` that may grow.** Pre-size everything.
- **No `panic!`.** Unwinding into C is UB. Wrap callback body in `std::panic::catch_unwind`.

**Required in the callback:**

- Read from a pre-allocated buffer. Write into a pre-allocated buffer.
- Communicate with the rest of the program via a wait-free SPSC ring or a single-slot atomic.
- Use algorithms with O(1) worst-case, not amortized.
- Atomics on built-in numeric types (`AtomicU32`, `AtomicUsize`) are fine; they're lock-free on every platform we target.

**Enforcement:**

```rust
#[cfg(debug_assertions)]
use assert_no_alloc::assert_no_alloc;

fn on_capture(buf: &[f32]) {
    #[cfg(debug_assertions)]
    assert_no_alloc(|| process(buf));
    #[cfg(not(debug_assertions))]
    process(buf);
}
```

`assert_no_alloc` panics on any allocation reaching the global allocator and is the standard tripwire in the Rust audio ecosystem ([docs.rs](https://docs.rs/assert_no_alloc)).

## §2 Lock-free producer-consumer between callback and writer thread

The audio callback is the producer. A normal Tokio task or `std::thread` is the consumer (resampling, WAV writing, Whisper feed). They communicate through a wait-free SPSC ring of `f32` samples.

| Crate                              | Topology                               | Wait-freedom                    | When to pick                                                                                                                                     |
| ---------------------------------- | -------------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **`rtrb`**                         | SPSC                                   | Wait-free both sides            | Strictest realtime guarantees. Used internally by Kira. `Producer::write_chunk_uninit` exposes a slice you can fill without intermediate copies. |
| **`ringbuf`**                      | SPSC (and async/MPMC variants in v0.4) | Wait-free SPSC                  | Larger API surface than rtrb (`push_slice`/`pop_slice` amortise cache-line sync). The de-facto pairing with `cpal` examples.                     |
| **`crossbeam::queue::ArrayQueue`** | MPMC                                   | Lock-free but **not wait-free** | CAS loops, can spin under contention. **Never on the audio thread.** Use only for non-audio producer-consumer.                                   |

**Recommendation:** `ringbuf::HeapRb<f32>` with `push_slice` from the callback and `pop_slice` from the writer is the pragmatic default. Kira's docs explicitly recommend `rtrb` if you want the strictest guarantees ([docs.rs/kira](https://docs.rs/kira/latest/kira/command/index.html)).

**Sizing.** Capacity = `max_expected_writer_latency_seconds * sample_rate * channels * 4`. For a writer that might block on disk for 200 ms at 48 kHz stereo: `0.2 * 48000 * 2 * 4 ≈ 76_800` samples; round up to next power of two = `131_072`. Pre-allocate once at startup.

**Overflow policy.** If the ring is full, the callback drops the new chunk and bumps an `AtomicU64 dropped_samples` counter. Logging from the callback is forbidden; the consumer reads the counter and emits the warning.

**Platform gotcha** ([cpal#970](https://github.com/RustAudio/cpal/issues/970)): on Windows WASAPI, certain `ringbuf::Producer::push` patterns inside the callback can cause the callback to stop firing entirely. Workaround is to batch with `push_slice` and never touch any other synchronization primitive in the same callback.

## §3 Sample-rate conversion with `rubato`

Whisper requires 16 kHz mono Float32. Our sources hand us 44.1 / 48 / 16 / 24 kHz depending on device:

- VPIO often picks 24 kHz on macOS (negotiated, not requested)
- ScreenCaptureKit defaults to 48 kHz
- Built-in M-series mic over cpal: 44.1 kHz

| Resampler         | Algorithm                              | Quality                    | Speed   | Pick for                                                                            |
| ----------------- | -------------------------------------- | -------------------------- | ------- | ----------------------------------------------------------------------------------- |
| **`SincFixedIn`** | Windowed-sinc with anti-aliasing       | Highest                    | Slowest | Archival recording, music, anything a human will listen to closely                  |
| **`FftFixedIn`**  | FFT-based block resampler, fixed ratio | High                       | Fast    | **Speech-to-text where ratio is known and constant** (default for the Whisper path) |
| **`FastFixedIn`** | Polynomial / linear-ish                | Lower (artefacts > ~6 kHz) | Fastest | Latency-critical preview, VAD pre-filter, anything you throw away                   |

**Recommendation:** `FftFixedIn::<f32>::new(input_rate, 16_000, chunk_size, sub_chunks, channels)`. Whisper's voiceband is < 8 kHz so the FFT resampler's near-Nyquist behaviour is irrelevant. `xd009642`'s [Streaming Audio APIs: Audio Decoding](https://xd009642.github.io/2024/11/04/streaming-audio-APIs-audio-decoding.html) walks through this exact choice and lands on the same answer.

**Real-world prior art.** whisper.cpp's reference example uses linear-ish for batch files ([whisper.cpp#1844](https://github.com/ggml-org/whisper.cpp/issues/1844)). `mlx-whisper` punts to soundfile/librosa, which uses high-quality sinc. For a streaming Rust transcriber, `FftFixedIn` is the pragmatic sweet spot.

**Latency.** All `FftFixedIn` variants are block-based — you owe at least `chunk_size` input samples before the first output samples appear. At 48 kHz with `chunk_size = 1024`, that's ~21 ms of algorithmic latency. For Whisper this is negligible; the model itself wants 1+ second windows.

**Do not resample on the audio callback.** `rubato::process()` allocates internally and uses FFT plans. Always resample on the consumer thread after the ring buffer.

## §4 Audio format negotiation with CoreAudio / VPIO

VoiceProcessingIO is opinionated about format. It picks its own sample rate at init, and on macOS the input and output rates **must match** ([AudioUnitMac](https://github.com/pinkydodo/AudioUnitMac)). Coercing it usually ends in `kAudioUnitErr_FormatNotSupported` or — worse — a silent stream where the callback never fires. CPAL has the same shape ([cpal#213](https://github.com/RustAudio/cpal/issues/213)).

**Idiomatic Rust pattern: ask the unit, then resample.**

1. Initialise the AudioUnit with no format opinion.
2. Query `kAudioUnitProperty_StreamFormat` _after_ initialisation to learn what VPIO actually chose. Treat the returned `AudioStreamBasicDescription` as truth.
3. Build the `rubato::FftFixedIn` for `negotiated_rate → 16_000` once, on the setup thread.
4. Convert interleaved → planar if needed, also on the consumer thread.

```rust
let mut unit = AudioUnit::new(IOType::VoiceProcessingIO)?;
unit.initialize()?;
let asbd: AudioStreamBasicDescription = unit.get_property(
    kAudioUnitProperty_StreamFormat, Scope::Output, Element::Input,
)?;
let captured_rate = asbd.mSampleRate as u32;          // e.g. 24_000 on AirPods, 44_100 on built-in
let captured_channels = asbd.mChannelsPerFrame as usize;
let resampler = FftFixedIn::<f32>::new(
    captured_rate as usize, 16_000, 1024, 2, 1,
)?;
```

This is exactly the pattern Attune's `VoiceProcessingMicCapture` uses — see `crates/attune-core/src/audio/voice_processing_capture.rs`. CPAL takes the same philosophy: `SupportedStreamConfig` returns what the device can actually do, never what you wished it could do.

**Channel handling.** VPIO is typically mono-in. ScreenCaptureKit is stereo by default. Downmix stereo → mono on the consumer thread with `(L + R) * 0.5` — Whisper does not benefit from sophisticated downmixing.

**Louder-track selection** (Attune-specific, from `b6f3fb0`): when both mic and system tracks are present, pick the louder one per frame instead of mixing. Preserves SNR for whoever is actually talking and is what Attune already does. Document this in any pipeline diagram so it doesn't get "improved" into a sum mixer.

## §5 WAV writing under streaming load

Use `hound` ([docs.rs](https://docs.rs/hound)). It is the boring, correct choice. Do not hand-roll a RIFF header unless you need a non-standard chunk.

**The lifecycle trap.** From [docs.rs/hound/WavWriter](https://docs.rs/hound/latest/hound/struct.WavWriter.html):

> "If finalize is not called, the file will be finalized upon drop. However, finalization may fail, and without calling finalize, such a failure cannot be observed."

`WavWriter::finalize` patches the RIFF and `data` chunk sizes once the writer knows the sample count. If you skip it, the destructor tries — and **silently swallows errors**.

**Rule:** always call `writer.finalize()?` explicitly and propagate the error. Treat the writer like a `File` you have to `close`.

**Arc trap.** Putting a `WavWriter` behind `Arc<Mutex<…>>` makes it nearly impossible to call `finalize` (which consumes `self`). The clones keep the writer alive past where you wanted to close it, and the file ends up with the wrong size header. Players that respect the chunk size will refuse to open it; others will truncate playback.

**Two safe shapes:**

1. **Owned by the writer thread.** The writer thread owns the `WavWriter` outright. Other threads send commands ("stop", "rotate file") via `mpsc::channel`. The writer thread calls `finalize()` on the owned writer when it processes "stop".
2. **`Option<WavWriter>` inside a `Mutex` only on the control thread.** The audio path never touches the mutex. When stopping, `lock().take().unwrap().finalize()`.

Attune's `AudioWavWriter` currently exposes a thread-safe `append(&[f32])` that locks an internal `Mutex<hound::WavWriter>`. This is shape (2) without the consume-on-finalize discipline — we rely on `Drop` to finalize, which silently discards errors. **Recommend a follow-up to add an explicit `finalize(self) -> Result<()>`** and call it from `CaptureSession::stop`.

**Sample alignment.** `hound::WavWriter::flush` returns `Error::UnfinishedSample` if the count is not a multiple of `channels`. Always write whole frames; mid-frame flushes produce technically-invalid files.

**Format choice.** For Whisper pipelines, write 16-bit PCM mono at the post-resample rate (16 kHz), not 32-bit float. Smaller files, identical model accuracy, every tool on earth can open it. Use `WavSpec { channels: 1, sample_rate: 16_000, bits_per_sample: 16, sample_format: SampleFormat::Int }`. Quantise on the consumer thread with `(x.clamp(-1.0, 1.0) * 32767.0) as i16`.

## §6 Testing audio code without hardware

You cannot CI-test a real microphone. Decouple early.

**Layered test strategy:**

1. **Pure DSP units take slices, return slices.** Resampling, downmix, RMS, gain. Trivially testable: feed a known sine, assert the output spectrum or RMS within tolerance.
2. **Callback shim takes a trait.** Define `trait AudioSink { fn write(&mut self, samples: &[f32]); }`. The real callback owns a `RingbufProducerSink`; tests own a `Vec<f32>`-backed sink. The callback function is generic over the sink and never references a real device.
3. **Fixture WAVs.** Check in short (~5 s) WAV files under `crates/attune-core/tests/fixtures/` covering:
   - Silence
   - 1 kHz sine
   - Pink noise
   - Real speech with known transcript
   - Clipped signal
   - Near-silent signal that triggered the hallucination bug in `d9a9b6e`

   Read with `hound::WavReader`, feed through the pipeline, assert.

4. **Time-domain assertions, not bit-exactness.** Resamplers and FFTs are not bit-reproducible across platforms. Assert on:
   - **RMS** within ±0.5 dB of expected
   - **Silence detection** — RMS below −60 dBFS for an all-zero input
   - **Frame count** — `output_len ≈ input_len * out_rate / in_rate` within ±`chunk_size`
   - **Length of detected speech regions** — VAD output frames, not transcript text

5. **Property tests** with `proptest`: arbitrary lengths, arbitrary input rates from `{8000, 16000, 22050, 44100, 48000}`. Assert the pipeline never panics, never produces NaN, never produces samples outside `[-1.0, 1.0]`.

**Reference patterns:**

- [`audio-processor-testing-helpers`](https://docs.rs/audio-processor-testing-helpers) formalises sine generators, RMS assertions, fixture loaders.
- [`rodio`](https://docs.rs/rodio) exposes a WAV `Source` sink specifically as a hardware-free path: _"Saving Source's output to a WAV file is intended primarily for testing and diagnostics."_
- CPAL itself has no automated audio tests for the callback path — its CI exercises device enumeration only. This is a known gap and the reason every downstream project rolls its own sink trait.

**What not to test:** Don't test that "the microphone returns audio." That's a smoke test for a human with headphones. CI cannot verify it, and flaky audio-device tests are worse than no tests. Mark such tests `#[ignore]` and document the manual procedure.

## Attune-specific recommendations (priority order)

1. **Add explicit `AudioWavWriter::finalize(self) -> Result<()>`** and call it from `CaptureSession::stop`. Today the `Drop` impl silently swallows finalize errors.
2. **Wrap the VPIO render callback in `std::panic::catch_unwind`** to prevent UB on an unwind into C. The current callback is tiny and shouldn't panic, but defense in depth is cheap.
3. **Adopt `assert_no_alloc` in debug builds** on the audio callback path. Catches the "I added a `format!` to a log line" regression immediately.
4. **Swap `Arc<Mutex<AudioWavWriter>>` for an mpsc-channel-owned writer** in a future refactor. Today's shape works but doesn't follow §5's "two safe shapes."
5. **Define `trait AudioSink` and `RingbufProducerSink`** so the cpal mic capture, the VPIO mic capture, and the system capture all funnel through the same testable interface.
6. **Add fixture WAVs** for the silence, 1 kHz sine, and pink noise tests under `crates/attune-core/tests/fixtures/`.

## Sources

- [Ross Bencina — Real-time audio programming 101: time waits for nothing](http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing)
- [Ross Bencina — Interfacing Real-Time Audio and File I/O](http://www.rossbencina.com/code/interfacing-real-time-audio-and-file-io)
- [timur.audio — Using locks in real-time audio processing, safely](https://timur.audio/using-locks-in-real-time-audio-processing-safely)
- [Micah Johnston — Basedrop: A garbage collector for real-time audio in Rust](https://micahrj.github.io/posts/basedrop/)
- [docs.rs — `assert_no_alloc`](https://docs.rs/assert_no_alloc)
- [docs.rs — `rtrb::RingBuffer`](https://docs.rs/rtrb/latest/rtrb/struct.RingBuffer.html)
- [Rust Audio Discourse — Announcement: rtrb](https://rust-audio.discourse.group/t/announcement-real-time-ring-buffer-rtrb/346)
- [docs.rs — `ringbuf`](https://docs.rs/ringbuf/latest/ringbuf/)
- [docs.rs — `kira::command`](https://docs.rs/kira/latest/kira/command/index.html)
- [github.com/RustAudio/cpal#970 — Windows: callback fails with ringbuf](https://github.com/RustAudio/cpal/issues/970)
- [github.com/RustAudio/cpal#213 — enumerate.rs shows unsupported rates](https://github.com/RustAudio/cpal/issues/213)
- [docs.rs — `rubato`](https://docs.rs/rubato/latest/rubato/index.html)
- [docs.rs — `rubato::FftFixedIn`](https://docs.rs/rubato/latest/rubato/struct.FftFixedIn.html)
- [docs.rs — `rubato::SincFixedIn`](https://docs.rs/rubato/latest/rubato/struct.SincFixedIn.html)
- [docs.rs — `rubato::FastFixedIn`](https://docs.rs/rubato/latest/rubato/struct.FastFixedIn.html)
- [xd009642 — Streaming Audio APIs: Audio Decoding](https://xd009642.github.io/2024/11/04/streaming-audio-APIs-audio-decoding.html)
- [github.com/ggml-org/whisper.cpp#909 — 16 kHz required](https://github.com/ggml-org/whisper.cpp/issues/909)
- [github.com/ggml-org/whisper.cpp#1844 — sample-rate discussion](https://github.com/ggml-org/whisper.cpp/issues/1844)
- [github.com/pinkydodo/AudioUnitMac — VPIO demo](https://github.com/pinkydodo/AudioUnitMac)
- [github.com/svtlabs/screencapturekit-rs](https://github.com/svtlabs/screencapturekit-rs)
- [docs.rs — `hound::WavWriter`](https://docs.rs/hound/latest/hound/struct.WavWriter.html)
- [docs.rs — `audio-processor-testing-helpers`](https://docs.rs/audio-processor-testing-helpers)
- [docs.rs — `rodio`](https://docs.rs/rodio)
- [Rust Audio working group](https://rust.audio/)
