# Changelog

All notable changes to Attune will be documented in this file. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [SemVer](https://semver.org).

## [Unreleased]

### Added

- Repo scaffolding: Cargo workspace, Rust toolchain pin, rustfmt + editorconfig, GitHub Actions CI stub.
- `attune-core` crate skeleton with audio capture module structure.
- `attune-cli` crate with `record` subcommand (mic capture working via cpal; system audio capture pending).
- Audio capture vertical slice (week 1 of v0 plan):
  - Microphone capture via cpal with automatic device selection.
  - Resampler to 16kHz mono via rubato.
  - WAV writer via hound.
  - System audio capture stub with TODO markers for ScreenCaptureKit integration (week 2).
- MIT license, .gitignore covering Rust + Xcode + macOS, README with target install UX.

## [0.0.1] — 2026-05-20 (initial commit)

Project initialized. See architecture and v0 plan in the design vault under `projects/attune/`.
