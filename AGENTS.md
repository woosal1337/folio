# Agent Conventions for Attune

Guidance for AI agents (Claude Code, Codex, etc.) working in this repo. Mirrors the human-facing CONTRIBUTING.md when that exists.

## Source of truth

- **Design and rationale** live in the Obsidian vault at `~/Documents/GitHub/obsidian.md/projects/attune/`. The architecture docs (`architecture/`), shipping plans (`plan/`), and decision records (`notes/`) are canonical. This repo implements those docs; it does not redefine them.
- **Code** lives here. Do not duplicate architecture prose into this repo's docs — link out to the vault paths instead.

## Commands

- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Test: `cargo nextest run` (preferred) or `cargo test`
- Build all: `cargo build --workspace --release`
- Run CLI: `cargo run -p attune-cli --release -- <subcommand>`

## Style

- 4-space indent, 100-char width (`rustfmt.toml` enforces).
- Errors via `thiserror` enums in `crates/attune-core/src/error.rs`. `AttuneError` is the public error type; new variants get added there, not invented per-module.
- Logging via `tracing`, never `println!`. Spans for cross-async work.
- No `unwrap()` outside tests. `expect("<reason>")` is acceptable for invariants that cannot fail.
- Comments only for non-obvious *why*. Code should not need comments to explain what it does.

## Architecture rules

- The crate boundary matters. `attune-core` is the library; `attune-cli` and the future `attune-app` (Swift) consume it. Do not let CLI-specific code leak into core.
- Public types that will cross the FFI boundary (UniFFI) live in a future `crates/attune-core/src/ffi/` module. For now, model types live alongside their owning module.
- Audio thread code must not allocate on hot paths. Use pre-allocated buffers via `crossbeam-queue::ArrayQueue` or similar.
- All cross-platform code is the default. macOS-specific code is gated by `#[cfg(target_os = "macos")]`.

## Tests

- Unit tests live alongside the code under `#[cfg(test)] mod tests`.
- Integration tests live in `crates/<crate>/tests/`.
- Audio code tests use synthetic signals (sine waves, silence, white noise) rather than real audio files where possible.
- Tests that need a real audio device are marked `#[ignore]` and run manually.

## When you do not know

- Read the matching vault doc first: e.g., for audio capture questions read `~/Documents/GitHub/obsidian.md/projects/attune/architecture/audio-capture.md`.
- For decisions not yet recorded, ask the user. Do not invent.
