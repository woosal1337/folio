# Rust error handling — Attune guidelines

Source-cited synthesis for the Attune codebase. Targets `attune-core`, `attune-cli`, and the `src-tauri` shell. Use this doc when adding new error variants, designing fallible APIs, or wiring an error across the Tauri IPC boundary.

## TL;DR — the rules

1. **`thiserror` inside crates that other code matches on. `anyhow` at the application boundary (`main`, Tauri commands).** Most production Rust uses both. ([Palmieri](https://www.lpalmieri.com/posts/error-handling-rust/), [OneUptime](https://oneuptime.com/blog/post/2026-01-25-error-types-thiserror-anyhow-rust/view))
2. **One error enum per subsystem, not one mega-enum per crate.** `AudioError`, `TranscriptionError`, `StorageError`, `LlmError` — not a single `AttuneError` that mixes I/O, Whisper, and Keychain failures.
3. **`#[non_exhaustive]` on public error enums** so adding a variant isn't a SemVer break. Skip it on crate-private enums. ([RFC 2008](https://rust-lang.github.io/rfcs/2008-non-exhaustive.html))
4. **Always preserve the source chain** via `#[source]` or `#[from]`. Never collapse to a string and lose context.
5. **`.with_context(|| ...)` lazy — `.context("…")` eager.** Use `with_context` whenever the message requires allocation. ([anyhow::Context](https://docs.rs/anyhow/latest/anyhow/trait.Context.html))
6. **No `.unwrap()` outside tests.** `expect("invariant: <why this cannot be None>")` is acceptable when the panic genuinely indicates a bug. ([burntsushi](https://burntsushi.net/unwrap/))
7. **Tauri commands return a tagged enum** that serializes as `{ kind, message }` so the frontend can branch on `kind`, not parse English. ([Tauri Tutorials](https://tauritutorials.com/blog/handling-errors-in-tauri))

## thiserror vs anyhow — when each fits

> "Do you expect the caller to behave differently based on the failure mode? Use an error enumeration. Do you expect the caller to just give up? Use an opaque error." — [Luca Palmieri](https://www.lpalmieri.com/posts/error-handling-rust/)

- **`thiserror` in `attune-core`**: every public function's `Err` variant is a thing a caller might match on. `LocalWhisperTranscriber::transcribe` returning `Err(TranscriptionError::ModelMissing { path })` lets the frontend show a "download the model" CTA.
- **`anyhow` in `attune-cli` and `src-tauri`**: these crates do glue work. They take a Result from `attune-core`, add context with `.with_context(|| ...)`, and either log-and-exit (`main`) or convert to the IPC error shape (Tauri commands).

```rust
// attune-cli/src/commands/transcribe.rs
let pcm = decode_wav_to_16k_mono(&args.audio)
    .with_context(|| format!("decoding {}", args.audio.display()))?;
```

## Designing error enums that survive 2 years

- **Group by subsystem, not by call site.** One `AudioError` covering capture, device, format, resampler beats one `MicError` + one `SystemError` + one `ResamplerError`.
- **Variants are _outcomes the caller can act on_.** Not 1:1 with every internal failure point. If the only action is "log and bail," merge into `Internal(#[source] anyhow::Error)`.
- **`Display` short and human; chain via `Debug`.** Anyone formatting an error with `{:?}` gets the full chain; `{}` shows the top message.
- **Wrap third-party errors inside your enum, don't leak them.** Don't expose `whisper_rs::WhisperError` or `cpal::BuildStreamError` in `pub fn` signatures of your public API.

```rust
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum TranscriptionError {
    #[error("model not found at {path}")]
    ModelMissing { path: PathBuf },

    #[error("whisper inference failed")]
    Inference(#[source] whisper_rs::WhisperError),

    #[error("audio decode failed")]
    Decode(#[source] hound::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

## The wrap-vs-flatten antipattern

From [nrc/error-docs](https://nrc.github.io/error-docs/error-design/error-type-design.html):

> "It is fairly common to have variants which simply contain a source error, with a simple `From` impl to convert one error to another. **This idiom is very overused, to the point where you should consider it an anti-pattern unless you can prove otherwise.**"

- **Wrap** (preserve source) when the inner error type is part of the caller's mental model: `std::io::Error` from a `read_file` operation is fine to surface.
- **Flatten with context** when the inner type is an implementation detail. `whisper_open(path).with_context(|| format!("loading model {path:?}"))?` beats nesting `WhisperError` inside three layers of enums.

Three rules of thumb:

1. If the caller will _match_ on `Err(_)`, give them named variants.
2. If the caller will only _display_ `Err(_)`, give them context, not nesting.
3. Wrapping should be additive — every wrap layer should _add_ meaning the caller wants. Wrappers that just rename = bad.

## Context with `anyhow` — lazy vs eager

- **`.context("static str")`** — eager, cheap, no allocation on the success path. Use when the message is a literal.
- **`.with_context(|| format!(...))`** — lazy, only formats on the error path. Use whenever you need string formatting.

```rust
// good — no format! on success
let cfg = std::fs::read_to_string(&path)
    .with_context(|| format!("reading config at {}", path.display()))?;

// also good — no allocation
fs::create_dir_all(&dir).context("creating recording directory")?;

// bad — always allocates
let cfg = std::fs::read_to_string(&path)
    .context(format!("reading config at {}", path.display()))?;
```

## `?` and `From` conversions

- `thiserror`'s `#[from]` generates the `From` impl and `?` does implicit conversion.
- "Always go for `From`. Implementing `From` automatically provides one with an implementation of `Into`." ([std::convert::From](https://doc.rust-lang.org/std/convert/trait.From.html))
- Don't write hand-rolled `From` impls for every nested error type — only for the ones you actually need at `?`-call sites.

## Tauri command error shape

> "Since the return type must implement `serde::Serialize`, most errors don't work directly... The `thiserror` + tagged enum pattern is the correct default. Set it up on day one." — [dev.to](https://dev.to/hiyoyok/rust-error-handling-in-tauri-commands-the-pattern-that-actually-works-35le)

Pattern Attune should adopt (todo):

```rust
// src-tauri/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("model not installed")]
    ModelMissing,

    #[error("recording session is busy")]
    SessionBusy,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl serde::Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("CommandError", 2)?;
        let kind = match self {
            CommandError::ModelMissing => "ModelMissing",
            CommandError::SessionBusy => "SessionBusy",
            CommandError::Other(_) => "Other",
        };
        st.serialize_field("kind", kind)?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}
```

The frontend then branches:

```ts
try {
  await invoke("transcribe_recording", { id });
} catch (e: any) {
  if (e?.kind === "ModelMissing") showDownloadCta();
  else toast.error(e?.message ?? String(e));
}
```

> "An asynchronous command won't crash on panic, but the JavaScript Promise will never resolve." — [Tauri Tutorials](https://tauritutorials.com/blog/handling-errors-in-tauri)

Wrap risky async commands in `catch_unwind` or just be disciplined: no `.unwrap()` in command bodies.

## Result/Option idioms

**Antipatterns:**

- `.unwrap()` in non-test code.
- `.expect("")` or `.expect("called .expect on None")` — the message must explain _why_ the panic is impossible.
- `match x { Ok(v) => v, Err(_) => return Err(...) }` where `?` works.
- Re-wrapping an error in your own enum with zero added context.
- Holding a `std::sync::Mutex` guard across `.await` (deadlocks Tokio).

**Good idioms:**

- `expect("invariant: settings.json is created at first run, cannot be missing here")`.
- `Option::map` for `T -> U`; `if let Some(x) = ...` when you need control flow or `?` inside.
- `.ok_or_else(|| MyError::Missing { name })` over `.ok_or(MyError::Missing { name: name.clone() })` — lazy alloc.
- `.unwrap_or_default()`, `.unwrap_or_else(|_| fallback())` for graceful degradation.
- Combinator chains (`map.and_then.map_err`) for short transforms; switch to `match` once branches have side effects.

## `tracing` is part of error handling

- **Format operators** — these are easy to get wrong:
  - `?value` → `Debug`
  - `%value` → `Display`
  - bare `value` → must implement `tracing::Value`

```rust
tracing::warn!(error = %err, file = ?path, "transcription failed");
```

- **`#[instrument]`** for function entry tracing instead of `debug!("foo started")` at the top. Use `skip(self, buffer)` for non-Debug or large args. ([docs](https://docs.rs/tracing/latest/tracing/attr.instrument.html))
- **Log levels for failure paths**:
  - `error!` — user-visible failure, action required.
  - `warn!` — recovered failure, degraded mode (audio dropout, retry).
  - `info!` — lifecycle (started capture, model loaded).
  - `debug!` — per-operation detail.
  - `trace!` — per-frame firehose, off in release.

- **`tracing-error::SpanTrace`** carries span context into the error, so the Tauri command boundary can log the full chain once and serialize only `{ kind, message }` to JS.

## Practical migration notes for Attune

Current state: single `AttuneError` enum at `crates/attune-core/src/error.rs`, ~15 variants.

Recommended changes (priority order):

1. **Split `AttuneError` into per-subsystem enums** (`AudioError`, `TranscriptionError`, `StorageError`, `LlmError`). Keep a top-level `AttuneError` that wraps them via `#[from]` if you want a single re-export. Tag all public enums `#[non_exhaustive]`.
2. **Add a Tauri-side `CommandError`** with a `Serialize` impl that emits `{ kind, message }`. Update commands to return `Result<T, CommandError>` instead of `Result<T, String>`.
3. **Replace `format!("…: {e}")` in error variants** with `#[source]` chains. Don't lose source by serializing it into a string field.
4. **Audit `.unwrap()` / `.expect()` for messages** — every `.expect()` should explain the invariant in one sentence.

## Sources

- [How to Design Error Types with thiserror and anyhow — OneUptime](https://oneuptime.com/blog/post/2026-01-25-error-types-thiserror-anyhow-rust/view)
- [Error Handling In Rust: A Deep Dive — Luca Palmieri](https://www.lpalmieri.com/posts/error-handling-rust/)
- [Rust Error Handling with anyhow and thiserror — Caroline Morton](https://www.carolinemorton.co.uk/blog/rust-error-handling-anyhow-thiserror/)
- [anyhow on docs.rs](https://docs.rs/anyhow) and [`Context` trait](https://docs.rs/anyhow/latest/anyhow/trait.Context.html)
- [The non_exhaustive attribute — Rust Reference](https://doc.rust-lang.org/reference/attributes/type_system.html)
- [RFC 2008: non_exhaustive](https://rust-lang.github.io/rfcs/2008-non-exhaustive.html)
- [Effective Rust — Item 4: Prefer idiomatic Error types](https://effective-rust.com/errors.html)
- [Error type design — nrc/error-docs](https://nrc.github.io/error-docs/error-design/error-type-design.html)
- [Handling Errors in Tauri — Tauri Tutorials](https://tauritutorials.com/blog/handling-errors-in-tauri)
- [Rust Error Handling in Tauri Commands: The Pattern That Actually Works](https://dev.to/hiyoyok/rust-error-handling-in-tauri-commands-the-pattern-that-actually-works-35le)
- [Using unwrap() in Rust is Okay — Andrew Gallant](https://burntsushi.net/unwrap/)
- [tracing on docs.rs](https://docs.rs/tracing) and [#[instrument]](https://docs.rs/tracing/latest/tracing/attr.instrument.html)
