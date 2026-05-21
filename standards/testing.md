# Testing Standards

Tests exist to keep refactors honest. We write the test that, if it
passes, gives confidence the change is safe.

## What we test (v0)

- **Pure Rust logic in `attune-core`** — resampler, WAV writer, error
  conversions, device enumeration shape. These are deterministic.
- **Command shape integration** in `src-tauri` — at least one test per
  command that exercises happy-path and one failure mode.
- **Manual UI smoke** for the frontend — Tab through every screen,
  start/stop a recording, open Settings. Once the test surface
  stabilises we add Vitest + Testing Library.

We do not test:

- The cpal + SCK platform integration in CI. CI runners may not have
  audio devices; the SCK API requires real screen recording permission.
  Real-device tests are `#[ignore]` and run manually.

## Rust

### Unit tests

- Live in `#[cfg(test)] mod tests { ... }` at the bottom of the file
  under test. The convention is `use super::*;` and one `#[test]` fn
  per behaviour.
- Tests are deterministic. No `rand::random()` without a seed. No
  reliance on filesystem layout outside `tempfile::tempdir()`.
- Audio tests use synthetic signals: sine, silence, white noise.
  Generated inline:

  ```rust
  let sine: Vec<f32> = (0..N).map(|i| ((i as f32) / 50.0).sin()).collect();
  ```

- Property assertions over numeric data tolerate small drift:
  `assert!((a - b).abs() < 1e-6)`.

### Integration tests

- Live in `crates/<crate>/tests/`. Each `.rs` file is its own binary.
- They exercise the public API, no `pub(crate)` reach-in. If a test
  needs internal access, the test belongs in the unit-test module.

### Ignored tests

- `#[ignore]` on tests that need a real audio device or network. Run
  them locally with `cargo test -- --ignored`.

### Doc tests

- `///` examples on public functions run via `cargo test --doc`. Keep
  them small and deterministic. Mark long-running examples `no_run`.

### Test runner

- Prefer `cargo nextest run` for speed. Falls back to `cargo test`
  cleanly if nextest is unavailable.
- CI runs `cargo test --workspace --lib --bins`.

## Frontend (TypeScript / React)

### Plan

- Vitest as the runner (Vite-native; no separate Jest config).
- Testing Library for component tests.
- A thin fake of `src/lib/api.ts` for Tauri command mocking.

### Rules

- Test behaviour, not implementation. "User sees a recording in the
  list after stop()" beats "useRecording sets a state variable".
- No snapshot tests. They give false confidence and rot.
- Custom hooks are tested through a component that uses them, not via
  `renderHook` unless the hook is genuinely standalone.

### When tests land

- Each route gets one happy-path test.
- The Tauri command wrapper layer gets a test per call shape (it's
  thin; the test ensures the argument names match the Rust side).
- The audio-player scrubber gets a test for the click-to-seek and
  drag-to-seek interactions.

## End-to-end

- Out of scope for v0. When it lands, use Tauri's webdriver bridge
  with Playwright, not a headless browser. The Tauri runtime is the
  thing we want to exercise.

## CI

- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --lib
  --bins` on every PR.
- Frontend: `tsc -b` and `eslint` once configured (see `tooling.md`).
- Release: full `cargo build --workspace --release` + `pnpm build`.

## Coverage

- No enforced coverage threshold. Coverage as a goal incentivises bad
  tests. We track it for visibility (`cargo llvm-cov`) once the
  test surface is stable.

## Test data

- Synthetic only. We do not commit `.wav` or `.mp3` files. If a test
  needs a known-good binary, generate it deterministically in the test
  itself.
- `tempfile::tempdir()` for filesystem-touching tests. Never write to
  the cwd, never leave files behind.

## Failure messages

- A failing assertion should explain what was expected and what was
  observed. The `assert_eq!`/`assert!` defaults are usually enough.
- For ranges and tolerances, format the bounds:

  ```rust
  assert!(
      out.len() >= lower && out.len() <= upper,
      "expected ~{} samples, got {}",
      target, out.len(),
  );
  ```

## Anti-patterns

- Tests that depend on order. Each test is independent.
- Tests that depend on time-of-day. Use deterministic timestamps.
- Tests that depend on a specific allocator. Don't assert on memory
  numbers.
- Tests that mock the standard library. Mock the boundary, not the
  ground.
