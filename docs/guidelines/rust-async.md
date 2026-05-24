# Rust async & concurrency — Attune guidelines

Source-cited guidance for writing async code in Attune's Tauri 2 shell and the underlying `attune-core` library. Covers Tokio patterns, `Send`/`Sync` boundaries for FFI handles, the audio realtime callback rules, and cancellation safety.

## TL;DR — the rules

1. **`parking_lot::Mutex` by default. `tokio::sync::Mutex` only when the guard must be held across `.await`.** ([Tokio tutorial](https://tokio.rs/tokio/tutorial/shared-state))
2. **Never hold a sync lock across `.await`.** Deadlocks Tokio's worker thread.
3. **`spawn_blocking` for CPU-heavy / blocking work.** Whisper inference, large file I/O, ffmpeg.
4. **`JoinSet` + `CancellationToken` for task groups.** `JoinSet::join_next` is cancel-safe in `select!`. ([JoinSet docs](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html))
5. **Audio realtime callback: no alloc, no lock, no syscall, no `Arc` drop, no panic across FFI.** Lock-free SPSC ring + atomics + `catch_unwind` at the boundary.
6. **`cpal::Stream` / VPIO `AudioUnit` live on a dedicated owner thread; async world only sees channels.**
7. **`tracing::instrument` at module entry points** with `?err` (Debug) or `%err` (Display) format operators.

## `parking_lot` vs `tokio::sync::Mutex`

From [Tokio's shared state docs](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html):

> "If the value behind the mutex is just data, it's usually appropriate to use a blocking mutex such as the one in the standard library or parking_lot."

Decision flowchart:

- Holding the guard across `.await`? → `tokio::sync::Mutex`.
- Otherwise? → `parking_lot::Mutex` (faster, no poisoning, no async overhead).

Holding a `std::sync::Mutex` across `.await` is a classic deadlock — Tokio can park the task on a worker thread that's holding the lock another task needs. The compiler does _not_ catch this; `parking_lot::MutexGuard` is `!Send` to prevent it, but `std::sync::MutexGuard` is `Send` and silently lets you shoot yourself.

Pattern from the Tokio tutorial:

> "Wrap it in a struct, and lock the mutex only inside non-async methods on that struct."

```rust
struct SettingsStore { inner: Mutex<SettingsData> }

impl SettingsStore {
    pub fn get(&self) -> Settings { self.inner.lock().clone() }
    pub fn save(&self, s: Settings) -> Result<(), Error> {
        let mut g = self.inner.lock();
        *g = s.clone();
        write_to_disk(&g)?;            // no .await here, safe to hold lock
        Ok(())
    }
}
```

## Structured concurrency with `JoinSet`

From [Adam Szpilewicz](https://medium.com/@adamszpilewicz/structured-concurrency-in-rust-with-tokio-beyond-tokio-spawn-78eefd1febb4):

> "Treating tasks like children of a parent — they don't outlive their scope, and they clean up on failure."

Prefer `JoinSet` over bare `tokio::spawn` whenever the lifetime of the spawned task is tied to the lifetime of the spawner:

```rust
use tokio::task::JoinSet;
let mut tasks = JoinSet::new();
tasks.spawn(async move { transcribe_chunk(c1).await });
tasks.spawn(async move { transcribe_chunk(c2).await });
while let Some(res) = tasks.join_next().await {
    let segment = res??;
    out.push(segment);
}
// JoinSet drop aborts any still-running tasks — no leaks.
```

`JoinSet::join_next` is **cancel-safe** in `select!` (per docs). That's not true of every `await` point — see below.

Pair with [`tokio_util::sync::CancellationToken`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) for graceful shutdown. One root token at app start, `.child_token()` per subsystem.

## Cancellation safety in `select!`

From [Comprehensive Rust](https://google.github.io/comprehensive-rust/concurrency/async-pitfalls/cancellation.html) and [Oxide RFD 400](https://rfd.shared.oxide.computer/rfd/0400):

> "Perhaps the most common source of async cancellation bugs is `tokio::select!`."

Definition: a future is cancel-safe if dropping it before completion is a no-op — the next attempt can succeed without losing state. The compiler can't check this; you have to know.

| Cancel-safe in `select!`                          | Not cancel-safe                                     |
| ------------------------------------------------- | --------------------------------------------------- |
| `JoinSet::join_next`                              | `AsyncReadExt::read_exact` (partial read lost)      |
| `mpsc::Receiver::recv`                            | `AsyncReadExt::read_to_end`                         |
| `tokio::time::sleep` (not held across resumption) | Most stateful protocol reads (e.g., HTTP/2 framing) |
| Custom futures that don't progress until polled   | Anything that yields after grabbing a resource      |

Rule of thumb: if you're unsure, spawn the operation into a task and `select!` on the `JoinHandle` instead — `JoinHandle::await` _is_ cancel-safe (it just leaves the task running).

## Channels vs spawn

- **Channels** (`tokio::sync::mpsc`, `broadcast`, `watch`) for **continuous data flow**: audio frames → encoder → transcriber. One task per pipeline stage.
- **`spawn`** for **fire-and-forget** units of work with a clear lifetime.

From the [Tokio tutorial](https://tokio.rs/tokio/tutorial/shared-state):

> "When you do want shared access to an IO resource, it is often better to spawn a task to manage the IO resource, and to use message passing to communicate with that task."

This is the standard pattern for Attune's `WavWriter` and `LocalWhisperTranscriber` — a dedicated worker task owns the resource; commands and event handlers send work to it via mpsc.

## `spawn_blocking` for CPU / blocking work

From [users.rust-lang.org](https://users.rust-lang.org/t/tokio-from-async-to-sync-and-back-to-async-block-on-vs-spawn-blocking/83438):

> "If you find yourself in an asynchronous execution context and needing to call some (synchronous) function which performs blocking operations, then consider wrapping that call inside `spawn_blocking()`."

Attune candidates:

- Whisper inference (`whisper_rs::FullParams::full(...)`)
- Large file I/O (model loading, big WAV reads)
- ffmpeg subprocess invocations
- `std::fs` operations that exceed a few KB

Anti-rule: never `.await` from inside a `spawn_blocking` closure. Use std blocking APIs inside; if you need to hop back to async, use a channel for results.

```rust
let path = args.audio.clone();
let segments = tokio::task::spawn_blocking(move || {
    let ctx = WhisperContext::new(&path.to_string_lossy(), Default::default())?;
    /* … sync inference … */
    Ok::<_, anyhow::Error>(segments)
}).await??;
```

## `Send` / `Sync` for FFI handles

`cpal::Stream` is `!Send` on macOS because the underlying `AudioUnit` callback must remain bound to the thread that built it ([cpal docs](https://docs.rs/cpal)). The same applies to our `VoiceProcessingMicCapture` — the AudioUnit and its render callback live where they were constructed.

**Do not** wrap them with `unsafe impl Send` and hope. That postpones a soundness bug.

**Do**: keep the stream on a dedicated owner thread, expose only channels and `AtomicBool` to the async world. Tauri's main thread is fine as the owner; spawn the consumer task on Tokio.

## Audio thread rules (verbatim)

The realtime audio callback runs on a high-priority OS thread driven by the hardware clock. The full discussion is in [`audio-pipeline.md`](./audio-pipeline.md); this section is the async-relevant subset.

**Forbidden in the callback:**

- Heap allocations (the allocator can take milliseconds under pressure).
- Locks of any kind. Even `parking_lot`'s fast path can call into the kernel under contention.
- Syscalls (`println!`, file I/O, `SystemTime::now()` on some platforms).
- `Arc::clone` whose drop will eventually run on the audio thread — the last drop frees memory.
- Unbounded loops; recursion; `Vec::push` that may grow.
- `panic!` — unwinding into C is UB. Wrap callback body in `std::panic::catch_unwind`.

**Required in the callback:**

- Pre-allocated buffers, read/write only.
- Lock-free SPSC ring (`rtrb` or `ringbuf::HeapRb`) to hand data to the consumer thread.
- Atomics (`AtomicU32`, `AtomicUsize`, `AtomicBool`) for control + counters.

**Out-of-band error signaling:**

- Bump an `AtomicU64` dropout counter. Consumer task reads it periodically and logs the delta with `tracing::warn!`.
- Never log from the audio thread.

## `tracing::instrument` and structured logs

Format operators (these matter, easy to get wrong):

- `?value` → `Debug`
- `%value` → `Display`
- bare `value` → must implement `tracing::Value`

```rust
tracing::warn!(error = %err, file = ?path, "transcription failed");
```

Conventions from [docs.rs/tracing](https://docs.rs/tracing/latest/tracing/attr.instrument.html):

- `#[instrument(level = "debug")]` in favor of `debug!("foo started")` at the top of fns.
- `skip(self, buffer)` to exclude large / non-Debug args from the span.
- `ret` records the `Ok` return value (won't double-log errors).
- `err` records errors at the configured level when the fn returns `Err`. Pair with `thiserror` error types.

Log levels:

| Level    | When                                                     |
| -------- | -------------------------------------------------------- |
| `error!` | User-visible failure, action required                    |
| `warn!`  | Recovered failure / degraded mode (audio dropout, retry) |
| `info!`  | Lifecycle (started capture, model loaded)                |
| `debug!` | Per-operation detail                                     |
| `trace!` | Per-sample / per-frame firehose — off in release         |

[`tracing-error::SpanTrace`](https://docs.rs/tracing-error) attaches span context to errors so the Tauri command boundary can log the full chain once before serializing `{ kind, message }` to JS.

## Tauri command async patterns

From the official Tauri 2 docs:

- `async fn` commands can't take `&str` or `State<'_, Data>` by reference. Take owned `String` and `State<'_, AppState>`.
- An async command that panics will hang the JS Promise forever. Catch panics or be disciplined about not panicking.
- Long-running work should not block the command. The command should _spawn_ the work onto an internal worker, return an ID / channel, and let the consumer subscribe via `tauri::ipc::Channel<T>` or `app.emit_to(...)` updates.

Pattern:

```rust
#[tauri::command]
pub async fn start_transcription(
    state: State<'_, AppState>,
    audio_path: PathBuf,
) -> Result<TranscriptionId, CommandError> {
    let id = state.transcription().enqueue(audio_path).await?;
    Ok(id)
}
```

`state.transcription()` returns a `&TranscriptionEngine` that owns the worker task; `enqueue` is a quick channel send. The command returns in milliseconds. Progress + completion arrive via emitted events.

## Attune-specific async pitfalls observed

- The CLI `attune-cli` currently uses `std::thread::sleep(Duration::from_secs(args.seconds))` from sync `main` — fine for a CLI test harness, but don't copy this pattern into the Tauri shell.
- `CaptureSession` uses `unsafe impl Send` (in `crates/attune-core/src/audio/capture.rs`) with a `// SAFETY:` justification noting that the cpal `Stream` is Send-via-Mutex-discipline. This is acceptable; the alternative would be a major restructure to keep the stream pinned to one owner thread (per the [Send/Sync](#send--sync-for-ffi-handles) guidance above). Revisit in a future refactor.
- Tauri command bodies in `src-tauri/src/commands/` are appropriately thin — most are <50 lines and delegate to `attune-core`. Keep it that way; resist adding business logic to commands.

## Sources

- [tokio::sync::Mutex docs](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html)
- [Tokio: Shared state tutorial](https://tokio.rs/tokio/tutorial/shared-state)
- [Tokio JoinSet docs](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html)
- [tokio_util::sync::CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)
- [Comprehensive Rust — Cancellation](https://google.github.io/comprehensive-rust/concurrency/async-pitfalls/cancellation.html)
- [Oxide RFD 400 — Cancel safety](https://rfd.shared.oxide.computer/rfd/0400)
- [Adam Szpilewicz — Structured Concurrency in Rust with Tokio](https://medium.com/@adamszpilewicz/structured-concurrency-in-rust-with-tokio-beyond-tokio-spawn-78eefd1febb4)
- [Sunshowers — Cancelling async Rust (RustConf 2025)](https://github.com/sunshowers/cancelling-async-rust)
- [cpal on docs.rs](https://docs.rs/cpal)
- [Rust forum — Data access in audio callback](https://users.rust-lang.org/t/data-access-in-audio-callback/82701)
- [tracing crate docs](https://docs.rs/tracing) and [#[instrument]](https://docs.rs/tracing/latest/tracing/attr.instrument.html)
- [tracing-error crate](https://docs.rs/tracing-error)
- [Calling Rust from the Frontend — Tauri v2](https://v2.tauri.app/develop/calling-rust/)
