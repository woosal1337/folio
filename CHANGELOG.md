# Changelog

All notable changes to Attune will be documented in this file. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [SemVer](https://semver.org).

## [Unreleased]

### Added

- **attune-gui** crate — egui + eframe based dark-theme window:
  - Live recording state with a pulsing red indicator and `mm:ss` duration counter.
  - Input device dropdown listing every input visible to cpal, with refresh button. Shows sample rate, channel count, and notes the 16 kHz resample step.
  - System audio toggle (disabled at runtime until ScreenCaptureKit lands in week 2; UX explains the state).
  - Output directory picker via native rfd file dialog.
  - Recent recordings list with duration, file sizes, and "reveal in Finder" buttons.
  - Persistent state across launches (eframe storage).
- **attune-core** device enumeration: `list_input_devices()` returning name, default flag, sample rate, channels.
- **attune-core** `MicCapture::start` now accepts an optional device name. Falls back to the system default.
- **attune-cli** new `devices` subcommand. `record` gains `--mic-device "<name>"`.
- Repo scaffolding: Cargo workspace, Rust 1.88 toolchain pin, rustfmt + editorconfig, GitHub Actions CI stub. Workspace-level clippy lint config.
- Audio capture vertical slice:
  - `audio::resampler` — rubato polyphase with stereo downmix.
  - `audio::wav_writer` — thread-safe hound writer with clamp + quantize.
  - `audio::mic` — cpal capture with f32/i16/u16 sample-format support.
  - `audio::system` — stub; ScreenCaptureKit integration tracked for week 2.
  - `audio::capture` — orchestrator with graceful fallback to mic-only.

## [0.0.1] — 2026-05-20 (initial commit)

Project initialized. See architecture and v0 plan in the design vault under `projects/attune/`.
