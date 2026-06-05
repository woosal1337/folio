# Agent Conventions for Folio

Guidance for AI agents (Claude Code, Codex, etc.) working in this repo.
Mirrors the human-facing CONTRIBUTING.md.

## Source of truth

The code in this repo is the source of truth. Architecture and rationale
live in `docs/ARCHITECTURE.md`; coding standards live in
`docs/CODE_STYLE.md`. Read those before making structural changes.

## Commands

- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Test: `cargo test --workspace`
- Build all: `cargo build --workspace --release`
- Run CLI: `cargo run -p folio-cli --release -- <subcommand>`
- Frontend: `bun install`, `bun run typecheck`, `bun run lint`, `bun run test`

## Style

**No comments, ever** — see [`CLAUDE.md`](./CLAUDE.md). No inline, block,
or doc comments in Rust/TS/JS/CSS. The only exceptions are load-bearing
machine directives (`eslint-disable`, `@ts-expect-error`, triple-slash
`<reference>`, the ts-rs generated header). Code documents itself.

**Read `docs/CODE_STYLE.md`** — it is the authoritative source for
naming, errors, logging, tests, concurrency, performance, security, and
git hygiene.

- 4-space indent for Rust, 100-char width (`rustfmt.toml` enforces).
- Errors via `thiserror` enums in `crates/folio-core/src/error.rs`.
  `FolioError` is the public error type; new variants get added there,
  not invented per-module.
- Logging via `tracing`, never `println!`.
- No `unwrap()` outside tests. `expect("<reason>")` is acceptable for
  invariants that cannot fail.

## Architecture rules

- The crate boundary matters. `folio-core` is the library; `folio-cli`
  and the Tauri app (`src-tauri`) consume it. Do not let app- or
  CLI-specific code leak into core.
- Audio thread code must not allocate on hot paths. Use pre-allocated
  buffers.
- Cross-platform code is the default. macOS-specific code is gated by
  `#[cfg(target_os = "macos")]`.

## Tests

- Unit tests live alongside the code under `#[cfg(test)] mod tests`.
- Integration tests live in `crates/<crate>/tests/`.
- Audio code tests use synthetic signals (sine waves, silence, white
  noise) rather than real audio files where possible.
- Tests that need a real audio device are marked `#[ignore]`.

## When you do not know

For decisions not recorded in `docs/`, ask the user. Do not invent.
