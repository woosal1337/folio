<p align="center">
  <img src="docs/assets/banner.png" alt="Folio — local-first meeting notes for macOS" width="100%">
</p>

<p align="center">
  <a href="https://folio.chele.bi"><img src="https://img.shields.io/badge/website-folio.chele.bi-0E0E10.svg" alt="Website"></a>
  <a href="https://github.com/woosal1337/folio/actions/workflows/ci.yml"><img src="https://github.com/woosal1337/folio/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License"></a>
  <a href="#development"><img src="https://img.shields.io/badge/platform-macOS-lightgrey.svg" alt="Platform"></a>
  <a href="https://tauri.app"><img src="https://img.shields.io/badge/built%20with-Tauri%202-24C8DB.svg" alt="Built with Tauri"></a>
</p>

# [Folio](https://folio.chele.bi)

Local-first meeting transcription for macOS. Captures system audio + microphone independently, transcribes on-device by default, writes per-meeting markdown to your vault. Audio leaves your machine only if you opt in — cloud transcription, or a GPU server you host yourself ([`server/`](server/README.md)). Website: [folio.chele.bi](https://folio.chele.bi).

**Status:** `1.0.0`. Install via [Homebrew](#install) or the notarized DMG, or build from source.

## What it does

- Sits in the menu bar. Watches your calendar and audio devices via EventKit (no Google OAuth, no Microsoft Graph).
- When a meeting starts, records both system audio and your microphone as separate streams via cpal + ScreenCaptureKit.
- Transcribes locally by default. The bundled Whisper backend (whisper.cpp via `whisper-rs`, Metal-accelerated on Apple Silicon) is the primary path; OpenAI's Whisper API is an opt-in fallback for users who want a faster cloud transcription on long meetings.
- Diarizes the system-audio track on-device (Speaker 1, 2, 3…) with pyannote segmentation + a speaker-embedding model; your microphone is always labelled **You**. No cloud.
- Writes a markdown file per meeting to your chosen vault path. Frontmatter includes attendees, duration, model, source.
- Ships a local MCP server (`folio-mcp`). Any MCP-aware tool — Claude Desktop, Cursor, Claude Code — gets read-only access to your transcripts, tasks, and memories over stdio. No cloud, no proxy (Settings → Connectors).
- Audio never leaves your machine on the default path. Privacy Mode (Settings → Privacy) physically blocks every outbound HTTP call except `localhost` so the app keeps working end-to-end with Wi-Fi off.

## Privacy & consent

No telemetry, no analytics, no crash reporting — enforced in CI. Audio, transcripts, and notes stay on your machine on the default path; the only network calls are the opt-in cloud-AI and webhook paths plus the one-time model download, all of which Privacy Mode can block. Full details, including data retention, are in [`docs/PRIVACY.md`](./docs/PRIVACY.md).

Recording a conversation can be illegal without the other participants' consent, and the rules vary by US state and by country (many require all-party consent). Folio gives you the tool; obtaining consent is your responsibility. Tell people before you record. See [`docs/PRIVACY.md`](./docs/PRIVACY.md#recording-consent).

## Install

Requires macOS 13 (Ventura) or later, on Apple Silicon or Intel. The
first time you use local transcription or diarization, Folio downloads the
model weights (a few hundred MB) from Hugging Face and the sherpa-onnx
GitHub releases.

### Homebrew (recommended)

```sh
brew tap woosal1337/folio https://github.com/woosal1337/folio
brew install --cask folio
```

`brew upgrade --cask folio` tracks new releases.

### Direct download

Grab the latest **Apple Silicon** `.dmg` from the [Releases page](https://github.com/woosal1337/folio/releases/latest), open it, and drag **Folio** to Applications:

- Apple Silicon — `Folio_<version>_aarch64.dmg`

Releases are code-signed with a Developer ID and notarized by Apple, so they open without a Gatekeeper prompt. (Intel builds aren't published yet — build from source on an Intel Mac if you need one.)

### Build from source

See [Development](#development).

## Repository layout

```
folio/
├── Cargo.toml                # workspace root
├── rust-toolchain.toml       # Rust 1.88, both Apple targets
├── crates/
│   ├── folio-core/          # audio capture, storage, transcription, diarization
│   ├── folio-cli/           # CLI test harness
│   └── folio-mcp/           # local MCP stdio server (notes, tasks, memories)
├── src-tauri/                # Tauri 2 desktop binary
├── src/                      # React 18 + TypeScript + Tailwind frontend
├── Casks/                    # Homebrew cask (this repo doubles as its own tap)
├── docs/                     # repo-local docs (see ARCHITECTURE.md)
└── .github/workflows/        # CI + release
```

For the on-disk module map, the IPC contract, and the rationale behind the key design decisions (why mic and system audio are captured as separate streams, why the default transcription path is local Whisper rather than a cloud API), see [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md).

**Stack:** Rust core (~70%) for audio + storage + transcription, Tauri 2 wrapping a React + TypeScript + Tailwind frontend. `cpal` for mic capture, `ScreenCaptureKit` for system audio, `rubato` for resampling, `hound` for WAV. TypeScript bindings for IPC types are generated from Rust via `ts-rs`.

## Development

Requirements:

- macOS 13 (Ventura)+ on Apple Silicon (Intel Macs build fine; Apple Silicon is the perf target)
- Rust 1.88 via [`rustup`](https://rustup.rs/) (the toolchain is pinned in `rust-toolchain.toml`)
- [Bun](https://bun.sh) 1.3+ (the only JS package manager + runtime this repo uses)
- Xcode command-line tools: `xcode-select --install`

```sh
git clone git@github.com:woosal1337/folio.git
cd folio
bun install
pre-commit install
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push
```

### Run the desktop app

```sh
bun tauri dev
```

First launch compiles the Rust workspace (~30 s on a warm cache). Subsequent launches reuse the cache.

### Run the CLI test harness

```sh
# List input devices
cargo run -p folio-cli --release -- devices

# Record from the default device for 60 seconds
cargo run -p folio-cli --release -- record --seconds 60

# Record from a specific device
cargo run -p folio-cli --release -- record --seconds 60 --mic-device "MacBook Pro Microphone"
```

Output: `./recordings/<timestamp>/mic.wav` (mono 16-bit PCM at the device's native rate).

On first run macOS prompts for microphone permission (and screen recording permission for system audio capture).

### Local checks (mirror CI)

```sh
# Rust
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --bins
cargo deny check

# Frontend (bun runs the npm scripts defined in package.json)
bun run typecheck
bun run lint
bun run format:check
bun run test
```

`pre-commit` runs the relevant subset on every commit; the full suite also runs in `.github/workflows/ci.yml`.

## Contributing

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for setup and the workflow. The authoritative code-style contract is [`docs/CODE_STYLE.md`](./docs/CODE_STYLE.md). Security issues go through [`SECURITY.md`](./SECURITY.md).

## License

Apache-2.0. See [LICENSE](./LICENSE). Third-party attributions in [NOTICE](./NOTICE).
