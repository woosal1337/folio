# Changelog

All notable changes to Folio will be documented in this file. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [SemVer](https://semver.org).

## [1.0.0] — Unreleased

Public-readiness sweep. Version matches `Cargo.toml`, `package.json`, and `tauri.conf.json`. The bullet list under `[Unreleased]` below is the in-development changelog for this release; landing PRs append there until the public flip.

### Added

- **Homebrew install** — `Casks/folio.rb` (this repo doubles as its own tap) plus an `update-homebrew-cask` workflow that pins the version + DMG checksums on every published release. `brew tap woosal1337/folio https://github.com/woosal1337/folio && brew install --cask folio`.
- **`folio-mcp` crate** — a local MCP stdio server exposing read-only access to transcripts, tasks, and memories (10 tools) to MCP-aware clients (Claude Desktop, Cursor, Claude Code). Bundled into release builds as a sidecar so Settings → Connectors works out of the box.
- **On-device speaker diarization** on the system-audio track (pyannote-segmentation-3.0 + WeSpeaker embedding via sherpa-onnx); the microphone is always labelled **You**.
- **Settings → Analytics** now computes real on-device activity totals (meetings, minutes, action items, decisions, memories) over a selectable date range.
- **Settings → Calendar** lists upcoming events from macOS Calendar (EventKit) when access is granted.
- **Silent-microphone detection** — the recording bar warns when a backend captures near-silent audio.
- `docs/CODE_STYLE.md` rev 2 — folded in the eight-document architecture research pass; now the authoritative style contract.
- `docs/refactor/PHASE-3-PUNCH-LIST.md` — four-agent audit punch list driving the public-release sweep.
- `NOTICE` — third-party license attributions.
- ~50 v2-roadmap MVP modules across `folio-core` and `src-tauri` shipped under through.

### Changed

- AI auto-agents, diarization, and live-transcription toggles are disabled (with a hint) until their model is downloaded or an API key is configured, so a toggle is never "on" while its dependency is missing.
- Error toasts strip the internal `ipc … failed:` plumbing and surface plain-language messages.
- README describes local Whisper as the default backend (the OpenAI Whisper API is the fallback); previously claimed local "lands in v1".
- `folio-core::audio::wav_writer` uses `parking_lot::Mutex` instead of `std::sync::Mutex` (`docs/CODE_STYLE.md` §6.1).
- `folio-core::memory::watcher` reindex channel is bounded (capacity 256) instead of unbounded.
- `folio-core::audio::devices` uses `tracing::warn!` instead of `eprintln!`.

### Security

- Capability split per window class, strict CSP, narrowed asset-protocol scope, OpenAI key moved from on-disk `Settings` to the macOS Keychain. Full P0 list in `docs/refactor/PHASE-3-PUNCH-LIST.md` §2.

## [Unreleased]

### Added

- **folio-gui** crate — egui + eframe based dark-theme window:
  - Live recording state with a pulsing red indicator and `mm:ss` duration counter.
  - Input device dropdown listing every input visible to cpal, with refresh button. Shows sample rate, channel count, and notes the 16 kHz resample step.
  - System audio toggle (disabled at runtime until ScreenCaptureKit lands in week 2; UX explains the state).
  - Output directory picker via native rfd file dialog.
  - Recent recordings list with duration, file sizes, and "reveal in Finder" buttons.
  - Persistent state across launches (eframe storage).
- **folio-core** device enumeration: `list_input_devices` returning name, default flag, sample rate, channels.
- **folio-core** `MicCapture::start` now accepts an optional device name. Falls back to the system default.
- **folio-cli** new `devices` subcommand. `record` gains `--mic-device "<name>"`.
- Repo scaffolding: Cargo workspace, Rust 1.88 toolchain pin, rustfmt + editorconfig, GitHub Actions CI stub. Workspace-level clippy lint config.
- Audio capture vertical slice:
  - `audio::resampler` — rubato polyphase with stereo downmix.
  - `audio::wav_writer` — thread-safe hound writer with clamp + quantize.
  - `audio::mic` — cpal capture with f32/i16/u16 sample-format support.
  - `audio::system` — stub; ScreenCaptureKit integration tracked for week 2.
  - `audio::capture` — orchestrator with graceful fallback to mic-only.

## [0.0.1] — 2026-05-20 (initial commit)

Project initialized. See [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) for the architecture.
