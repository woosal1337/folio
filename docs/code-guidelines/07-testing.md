# 07 — Testing

From the Rust Book ch.11 and matklad "How to Test". `CODE_STYLE.md` §5 is canon
(what needs tests, layout, required surfaces).

## 1. Layout

- Unit tests in `src/` inside `#[cfg(test)] mod tests { use super::*; }` (compiled
  only under `cargo test`; can reach private items).
- Integration tests in top-level `tests/`, one crate per file, no `#[cfg(test)]`,
  exercising only the public API. _(Folio: `crates/folio-core/tests/
transcription_fixtures.rs`.)_
- Share integration helpers via `tests/common/mod.rs` (not `tests/common.rs`).
- Binary logic lives in the lib so integration tests can reach it.

## 2. Assertions

- Use `assert_eq!`/`assert_ne!` over `assert!(a == b)` (prints both sides).
- Attach a custom message when the failure isn't self-explanatory.
- Use `#[should_panic(expected = "...")]`, not bare `#[should_panic]`.
- Test error paths explicitly; `Result`-returning tests let you use `?`.

## 3. Test design (matklad)

- **No logic in tests; funnel cases through one `check(input, expected)` helper**
  marked `#[track_caller]` so failures point at the call site.
- Drive cases as **data** (table/parameterized), not procedural code.
- **Test at boundaries / through the public API;** don't unit-test private helpers
  or mock collaborators (mocks ossify implementation).
- Keep tests **deterministic and independent** — no shared mutable state, no
  ordering reliance.
- **Never `sleep`/timeout to wait;** use a primitive that blocks until done.
- Use **structured concurrency;** never fire-and-forget background work in a test.
- Separate IO from computation ("sans-io") and test the pure core for speed.
- Use **snapshot/expectation testing** (`insta`/`expect-test`) with auto-update
  for large outputs.
- Expose internal decisions via logging/coverage marks, not by reaching into
  private state.
- Encode format/license/doc/perf checks as `#[test]`s.

## Folio specifics

- Frontend: vitest unit + Playwright e2e (mocked-IPC harness in
  `e2e/fixtures/scenario.ts`). Keep the mock in sync with the real command
  contracts (a mismatch is a real bug — see the `account_update` envelope fix).
- Real Whisper pipeline: `cargo test -p folio-core --test transcription_fixtures
-- --ignored` against an on-disk `ggml-*.bin` (skips gracefully when absent).

## Sources

Rust Book ch.11 (11-01 writing tests, 11-03 organization) · matklad "How to Test".
