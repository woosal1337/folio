# Rust Standards

Attune targets Rust 1.88, edition 2021, pinned via `rust-toolchain.toml`. The
goal is clear, audit-friendly code in a workspace that compiles cleanly on
both Apple Silicon and Intel macOS.

## Workspace shape

- Workspace dependencies live in the root `Cargo.toml` under
  `[workspace.dependencies]`. Crates reference them with
  `name = { workspace = true }`. Never duplicate a version pin per-crate.
- Workspace lints live in `[workspace.lints]`. Per-crate `[lints]` should be
  `workspace = true` unless there is a documented reason.
- Crate names are kebab-case (`attune-core`), library names are snake_case
  (`attune_core`). Binary targets use kebab-case (`attune-cli`).

## Formatting

- `rustfmt.toml` is canonical. The file pins `edition = "2021"`,
  `max_width = 100`, `tab_spaces = 4`, `use_field_init_shorthand = true`,
  `use_try_shorthand = true`. Do not override locally.
- Run `cargo fmt --all` before every commit. CI fails on `cargo fmt --check`.
- Imports are grouped by `rustfmt` (std, external, internal). Do not hand-sort.

## Linting

- `cargo clippy --workspace --all-targets -- -D warnings` must pass.
- `RUSTFLAGS="-D warnings"` is set in CI; treat all warnings as errors locally
  too.
- Allowed exceptions live in `[workspace.lints.clippy]` with a one-line
  comment explaining why. Currently only `uninlined_format_args` is allowed.

## Error model

- Each public crate defines a single `Error` enum with `thiserror` derives.
  `attune-core` exposes `AttuneError`. New error categories are added as
  variants, not as new top-level types.
- Errors carry context. Prefer `#[error("audio device error: {0}")] AudioDevice(String)`
  over opaque `#[error("audio device error")] AudioDevice`. Use `String` for
  wrapped third-party errors; use typed inner errors when the caller might
  match on them.
- `#[from]` is reserved for losslessly wrapping a single foreign error type
  (e.g. `std::io::Error`, `hound::Error`). Use it sparingly; manual
  conversion via `.map_err(|e| AttuneError::Foo(format!("{e}")))` is fine and
  often more informative.
- `anyhow` is allowed in binaries (`attune-cli`, `attune-app`) at the
  top-level edge where errors are formatted for the user. Libraries do not
  return `anyhow::Result`.
- `Result<T>` is a per-crate type alias for `Result<T, AttuneError>`. Do not
  re-define the alias in each module.

## Unwrap discipline

- No `unwrap()` outside `#[cfg(test)]` modules and `build.rs`. Use
  `expect("<reason>")` only for invariants that cannot fail at runtime.
- `unreachable!()` is allowed when match exhaustiveness or type-state proves
  the branch is dead. Annotate with the invariant.
- `panic!()` is reserved for cases that indicate programmer error (config
  malformed at startup, etc.). User-facing failures return `Result`.

## Logging

- `tracing` only. Never `println!` for diagnostic output. The CLI may use
  `println!` for user-facing output (table rows, success messages); it does
  not use `println!` for errors.
- Use `info!` for state transitions ("capture started"), `warn!` for
  recoverable problems ("system audio unavailable, continuing mic-only"),
  `error!` for failures, `debug!` for internals, `trace!` for hot-path
  detail behind a feature gate.
- Use structured fields, not interpolation: `info!(device = %name, "selected")`
  not `info!("selected {name}")`. The `%` sigil uses Display, `?` uses Debug.
- One span per long-lived operation. Audio thread code does not emit logs
  inside the inner loop.

## Async

- `tokio` is the runtime. Async runs in Tauri command handlers and CLI
  subcommands. Library code is sync where the operation is sync (audio
  capture, WAV writes) and async where it must be (HTTP).
- Do not spawn `tokio::task::spawn` for fire-and-forget unless you also
  capture the handle. Detached tasks lose errors.
- Bridge audio thread → async land via `crossbeam-channel`, not Tokio mpsc.
  Crossbeam channels do not depend on a runtime and do not block runtime
  threads.

## Concurrency

- `parking_lot::Mutex` for sync state shared across threads. It is faster
  than `std::sync::Mutex` and never poisons.
- `std::sync::Mutex` only when an external API requires it (e.g.
  `WavWriter` ownership across threads). Wrap with a thin layer that maps
  poison errors to `AttuneError`.
- Audio thread code MUST NOT allocate on the hot path. Pre-allocate buffers
  in setup; reuse them in callbacks. See `standards/audio.md`.
- `crossbeam-queue::ArrayQueue` for fixed-size ring buffers across the
  audio → consumer boundary. Lock-free, no allocation per push/pop.
- `AtomicBool`, `AtomicU64` with `Ordering::SeqCst` for stop flags and
  counters. Looser orderings are allowed when the data does not order other
  memory accesses; document the reasoning.

## Module layout

- One concern per file. `audio/mic.rs` is mic capture; `audio/system.rs` is
  system audio. `mod.rs` re-exports the public surface but does not contain
  implementation.
- `pub use` re-exports from `mod.rs` are the public API surface of the
  module. Internal types are not re-exported.
- macOS-only code is `#[cfg(target_os = "macos")]` at the module or item
  level. A stub `#[cfg(not(target_os = "macos"))]` impl exists so the crate
  compiles cross-platform.
- FFI types intended to cross the UniFFI boundary live in
  `crates/attune-core/src/ffi/` when that lands. For now, model types live
  alongside their owning module.

## Comments

- Doc comments (`///`, `//!`) on every public item: function, struct, enum,
  trait, module. Doc comments explain *what* the item is and *what* it
  guarantees. They run in `cargo test --doc`.
- Inline comments only for non-obvious *why*. Code that needs an inline
  comment to explain *what* it does should be rewritten with clearer names.
- Always keep `// SAFETY:` blocks above `unsafe`. They are part of the
  correctness proof.
- Do not leave commented-out code in the tree. Use `git log` to recover it.
- Do not add `// TODO:` to the tree. File an issue or a tracker note instead.
  Exception: `// TODO(#123)` referencing an open issue is allowed.

## Naming

- Types: `UpperCamelCase`. Acronyms are not all-caps:
  `HttpClient`, not `HTTPClient`. `WavWriter`, not `WAVWriter`.
- Functions, methods, fields, modules: `lower_snake_case`.
- Constants: `SCREAMING_SNAKE_CASE`.
- Boolean fields: positive form. `enabled` not `disabled`; `recording` not
  `not_idle`.
- Builder methods: verb form. `with_sample_rate(rate)`, `set_channels(n)`.
- Conversion methods follow std: `as_str` (cheap), `to_string` (cloning),
  `into_path` (consuming).

## Tests

- Unit tests live in `#[cfg(test)] mod tests { ... }` at the bottom of the
  file under test. Use `super::*` to bring everything into scope.
- Integration tests live in `crates/<crate>/tests/`. They exercise the
  public API only.
- Audio code tests use synthetic signals (sine, silence, white noise),
  not recorded files in the repo. Generating test signals is a few lines
  with `(0..n).map(|i| (i as f32 / k).sin())`.
- Tests that require an audio device are marked `#[ignore]` and run
  manually with `cargo test -- --ignored`.
- Prefer property-style assertions on numeric output (`assert!((a - b).abs() < eps`)
  over exact equality. Floating-point round-trips will drift by ULPs.

## Performance

- Profile before optimizing. `cargo flamegraph` and `samply` for CPU,
  `dhat-rs` for allocations. Do not pre-optimize against guesses.
- Allocation discipline scales with how often code runs. Audio callbacks
  (44.1 kHz × callback size) must be allocation-free. Tauri command
  handlers (user clicks) can allocate liberally.
- `Vec::with_capacity(n)` when `n` is known. `Vec::extend_from_slice` over
  loops. `Vec::drain(..)` returns an iterator; collecting it allocates a
  second Vec — drain into a reusable buffer instead.
- Prefer `&str` and `&[T]` in signatures over owned types. Take `impl AsRef<Path>`
  for path parameters.

## Dependencies

- Add a dependency only if the alternative is more than ~100 lines of code
  or a class of bugs (e.g. WAV format, SIMD resampler, HTTP client). Pull
  one feature flag at a time; minimal feature sets keep builds fast.
- Pin to compatible-major (`"1.0"` for `1.x`). Do not pre-release-pin
  (`"=1.2.3"`) unless tracking a known regression.
- Audit transitively for `unsafe` and license terms before adding. Run
  `cargo deny check` periodically (set up in `tooling.md`).
- `default-features = false` and pull in only what you use. Each unused
  feature is build time and binary size.

## Unsafe

- Each `unsafe` block has a `// SAFETY:` comment justifying every invariant
  the compiler can't check.
- `unsafe impl Send/Sync` requires a paragraph-length justification (see
  `CaptureSession` in `audio/capture.rs` for the template).
- FFI types from `cocoa`/`objc` are unsafe by definition; wrap them in a
  safe abstraction at the module boundary and confine the unsafe surface
  to that wrapper.

## Lints we enforce

```toml
[workspace.lints.clippy]
uninlined_format_args = "allow"   # noisy, fights with our log format style

# Recommended additions to enforce as we grow:
# missing_docs_in_private_items = "warn"
# unwrap_used = "warn"
# expect_used = "warn"     # already convention; turn on once tests are gated
# print_stderr = "warn"
# print_stdout = "warn"
# todo = "warn"
```

## Release profile

The workspace `[profile.release]` uses `opt-level = 3`, `lto = "thin"`,
`codegen-units = 1`, `debug = "line-tables-only"`, `strip = "symbols"`. This
is the Tauri-distribution profile. Do not change it without measuring
binary size and startup time.
