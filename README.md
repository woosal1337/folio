# Attune

Local-first meeting transcription for macOS. Captures system audio + microphone independently, transcribes on-device with Whisper, writes per-meeting markdown to your vault. Audio never leaves your machine.

**Status:** v0 in active development. Target ship: 2026-07-15. Currently building the audio capture vertical slice (week 1).

## What it does

- Sits in the menu bar. Watches your calendar and audio devices.
- When a meeting starts (calendar event, meeting URL click, or manual hotkey), records both system audio and your microphone as separate streams.
- Transcribes both locally with Whisper (large-v3 or distil-large-v3).
- Labels speakers: `me` (mic channel) vs `others` (system channel). Multi-speaker diarization on the system channel lands in v1.
- Writes a markdown file per meeting to your chosen vault path. Frontmatter includes attendees, duration, model, source.
- Audio never leaves your machine. The only network calls are: Whisper model download (once), Sparkle update check (daily).

## Install (target UX, not yet shipped)

```sh
brew install --cask woosal1337/attune/attune
```

For now, build from source. See [Development](#development).

## Architecture

The full architecture and v0 plan live in the design vault, not in this repo:

- `projects/attune/architecture/` — system overview, shell decision, audio capture, transcription pipeline, diarization, vault write, meeting detection, calendar integration, state management, FFI boundary, privacy and consent
- `projects/attune/plan/` — tech prerequisites, v0 shipping plan, v1 roadmap, distribution plan
- `projects/attune/notes/` — decision records

In this repo:

```
attune/
  Cargo.toml                workspace
  rust-toolchain.toml       Rust 1.78 pinned
  crates/
    attune-core/            library: audio, whisper, vault, store
    attune-cli/             CLI for testing the core
    attune-gui/             dark-theme egui GUI (interim shell, ships with v0)
  apps/
    Attune.xcodeproj        (added week 4) SwiftUI shell
    Attune/                 (added week 4) Swift sources
  scripts/                  release.sh, build-xcframework.sh, update-cask.sh
  migrations/               refinery SQL migrations
  models/                   manifest of supported Whisper models
  .github/workflows/        ci.yml, release.yml
```

**Stack:** Rust core (~70%) compiled as universal dylib, exposed to a thin SwiftUI shell via UniFFI bindings. CoreAudio HAL Tap + AVAudioEngine for capture. whisper.cpp + Metal for inference. SQLite + markdown for storage. Sparkle 2 for updates. Distribution via Homebrew Cask.

## Development

Requirements:

- macOS 14.4+ on Apple Silicon (Intel Macs work but are not the perf target)
- Rust 1.88 via rustup
- Xcode 16+ command line tools

```sh
git clone git@github.com:woosal1337/attune.git
cd attune
cargo build --workspace --release
```

### GUI

```sh
cargo run -p attune-gui --release
```

A dark-theme window with input device picker, system audio toggle, output directory picker, and a recording button with live duration counter. Recordings land in the selected output directory as `<timestamp>/mic.wav` (16 kHz mono PCM).

### CLI

```sh
# List input devices
cargo run -p attune-cli --release -- devices

# Record from the default device for 60 seconds
cargo run -p attune-cli --release -- record --seconds 60

# Record from a specific device
cargo run -p attune-cli --release -- record --seconds 60 --mic-device "MacBook Pro Microphone"
```

This produces `./recordings/<timestamp>/mic.wav`. System audio capture is stubbed until week 2.

On first run, macOS will prompt for microphone permission (and screen recording permission for system audio capture on macOS 13+).

## License

MIT. See [LICENSE](./LICENSE).
