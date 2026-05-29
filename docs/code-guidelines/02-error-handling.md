# 02 — Error Handling, Robustness, Correctness

From the Rust Book ch.9, thiserror/anyhow/eyre docs, `std::error::Error`,
`std::sync::Mutex`, the Rustonomicon (FFI), BurntSushi, and Sabrina Jewson
("Modular Errors"). `CODE_STYLE.md` §3 is canon (single `AttuneError` enum in
core; `Result<_, String>` at the Tauri boundary).

## 1. Result vs panic

- **Return `Result<T, E>` for any failure a caller could anticipate and act on.**
- **Panic only when all hold:** the state is unexpected, downstream relies on not
  being in it, and it can't be encoded in the type system. A panic means *bug*.
- Out-of-bounds / broken invariants / contract violations → panic. Malformed
  input / missing files / rate limits / bad user data → `Result`.
- `Result` is the default; panic is the documented exception.

## 2. `unwrap` / `expect` discipline

- **No bare `.unwrap()` in production library code** — it's a context-free panic
  that removes the caller's recovery. Propagate with `?`.
- When you must panic on `None`/`Err`, use `.expect("...")` over `.unwrap()`.
- **Write `expect` in the "should" style** describing the invariant:
  `"hardcoded IP should be valid"`, not `"parse failed"`.
- `unwrap`/`expect` are fine in examples, prototypes, and tests.
- Treat any `.unwrap()`/`.expect()` in non-test library code as a defect to
  justify (with a comment) or remove.

## 3. The `?` operator and `From` conversion

- Prefer `?` over explicit `match`/`map_err` chains for propagation.
- Implement `From<SourceError>` for each emitted error so `?` converts
  automatically (no per-call `.map_err`).
- `?` works only in fns returning `Result`/`Option`/`FromResidual`.
- Don't mix `Result`/`Option` through `?`; convert first (`opt.ok_or(e)?`).

## 4. Library error enums (thiserror)

- **`thiserror` for libraries; `anyhow`/`eyre` for binaries.** Libraries must
  expose typed, matchable errors and not leak `anyhow` into their public API.
- Every variant has `#[error("...")]` — lowercase, no trailing punctuation.
- Use `#[from]` only where the conversion is unambiguous; otherwise wrap manually
  so the variant names *which step* failed (`ReadConfig`, not `Io`).
- Use `#[source]`/`#[from]` to chain the cause so `source()` is populated; use
  `#[error(transparent)]` for pass-through variants.
- Create error types per *unit of fallibility*, not one mega-enum, when failures
  differ in kind/message. (Attune's single `AttuneError` is the deliberate house
  exception — see `CODE_STYLE.md` §3.1.)
- Keep dependency error types out of the public enum (private `Kind`) so a dep
  upgrade isn't a breaking change. Provide `type Result<T> = ...` alias.
- For extensibility prefer `#[non_exhaustive] struct { ctx, kind }` + `Kind` enum
  (enum variant fields can't grow without breaking).

## 5. `std::error::Error` design

- Impl `Display` (lowercase, concise) and `Debug`; render the cause via
  `source()` **or** `Display`, never both.
- Library error types are `Error + Send + Sync + 'static`. Never use `()`/`String`.
- Don't impl deprecated `description()`/`cause()`.

## 6. anyhow / eyre for binaries

- Return `anyhow::Result<T>`; propagate with `?`; attach `.context("...")` /
  `.with_context(|| ...)` (closure form when the message allocates) at each layer.
- Use `bail!`/`ensure!` for early returns and checks.
- **Never expose `anyhow::Error`/`eyre::Report` in a library's public API.**
- Log an error **once**, at the boundary where it's handled — not at every `?`.

## 7. Locks, poisoning, IO

- `.lock().unwrap()` is the accepted idiom when you trust the invariant (a
  poisoned lock means another thread panicked mid-update). To recover, match and
  `poisoned.into_inner()`, then `clear_poison()`. (Attune uses `parking_lot`,
  which **doesn't poison** — `.lock()` returns the guard directly, no `.unwrap()`.)
- Do not rely on poisoning for soundness in `unsafe` code.
- **Never `.unwrap()` real IO results in production;** propagate with `?` + context.

## 8. Option/Result combinators

- Use `map`/`and_then`/`unwrap_or(_else)`/`ok_or(_else)`/`map_err` for simple
  transforms instead of verbose `match`.
- `Option`→`Result` via `ok_or`/`ok_or_else` when you can explain the absence.
- `Option<T>` for plain absence; `Result<T, E>` only when failure has a reason.
- Prefer a linear `?` sequence over a deep combinator chain when it reads clearer.

## 9. FFI & panic safety

- A Rust panic unwinding across a plain `extern "C"` boundary **aborts** the
  process; a foreign exception entering Rust across a non-`unwind` ABI is **UB**.
- Wrap panicking exported-to-C code in `catch_unwind` → error code, or use
  `extern "C-unwind"` only when deliberately interoperating with unwinding code.
- Prefer explicit error codes/out-params over panics at any FFI boundary.

## 10. Boundaries, constructors, validation

- Validate inputs at the boundary by constructing a type whose invariant then
  holds everywhere downstream (`Guess::new(v)` once vs re-checking).
- Fallible constructors return `Result` (or `try_new`) for recoverable bad input;
  panic in a constructor only when bad input is a *caller bug*.
- Keep fields private so the validating constructor is the only entry.

## 11. Don't swallow errors

- Don't `let _ = fallible();` unless you state why; reserve it for genuinely
  ignorable results with a comment.
- Don't collapse a structured error into a `String` inside a library.
- `Result` is `#[must_use]`; honor it, and add `#[must_use]` to your own
  result-like/builder types.

## Quick table

| Situation | Do |
|---|---|
| Foreseeable failure | `Result` |
| Violated invariant / bug | `panic!` |
| Library | `thiserror`, typed, `Send + Sync`, no `unwrap` |
| Binary/app | `anyhow`/`eyre` + `.context()` per layer |
| `parking_lot` lock | `.lock()` (no unwrap) |
| Must panic | `.expect("X should be Y")` |
| `Option`→`Result` | `ok_or`/`ok_or_else` |
| Exported to C | `catch_unwind` → code, or `extern "C-unwind"` |

## Sources

thiserror / anyhow / eyre docs · Rust Book ch.9 (00/02/03) · BurntSushi "Error
Handling in Rust" · Sabrina Jewson "Modular Errors in Rust" · `std::error::Error`
· `std::sync::Mutex` (poisoning) · API Guidelines (C-GOOD-ERR) · Rustonomicon FFI
· LogRocket "panic vs error".
