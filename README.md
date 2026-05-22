# Attune

Local-first meeting transcription for macOS. Captures system audio + microphone independently, transcribes on-device, writes per-meeting markdown to your vault. Audio never leaves your machine.

**Status:** v0 in active development. Target ship: 2026-07-15.

## What it does

- Sits in the menu bar. Watches your calendar and audio devices.
- When a meeting starts, records both system audio and your microphone as separate streams via cpal + ScreenCaptureKit.
- Transcribes on-device. v0 uses the OpenAI Whisper API when the user opts in; the local `whisper.cpp` backend lands in v1.
- Labels speakers: `me` (mic channel) vs `others` (system channel). Multi-speaker diarization on the system channel lands in v1.
- Writes a markdown file per meeting to your chosen vault path. Frontmatter includes attendees, duration, model, source.
- Audio never leaves your machine. The only outbound calls are model downloads and (when you opt in) OpenAI transcription.

## Install (target UX, not yet shipped)

```sh
brew install --cask woosal1337/attune/attune
```

For now, build from source. See [Development](#development).

## Repository layout

```
attune/
├── Cargo.toml                # workspace root
├── rust-toolchain.toml       # Rust 1.88, both Apple targets
├── crates/
│   ├── attune-core/          # audio capture, storage, transcription
│   └── attune-cli/           # CLI test harness
├── src-tauri/                # Tauri 2 desktop binary
├── src/                      # React 18 + TypeScript + Tailwind frontend
├── docs/                     # repo-local docs (see ARCHITECTURE.md)
└── .github/workflows/        # CI
```

The full architectural rationale (why mic and system audio are separate streams, the v0 → v1 transcription path, the future Swift app shell via UniFFI, etc.) lives in the design vault referenced by the maintainer. For the on-disk module map and IPC contract see [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md).

**Stack:** Rust core (~70%) for audio + storage + transcription, Tauri 2 wrapping a React + TypeScript + Tailwind frontend. `cpal` for mic capture, `ScreenCaptureKit` for system audio, `rubato` for resampling, `hound` for WAV. TypeScript bindings for IPC types are generated from Rust via `ts-rs`.

## Development

Requirements:

- macOS 13.3+ on Apple Silicon (Intel Macs build fine; Apple Silicon is the perf target)
- Rust 1.88 via [`rustup`](https://rustup.rs/) (the toolchain is pinned in `rust-toolchain.toml`)
- Node 20+ and pnpm 9+
- Xcode command-line tools: `xcode-select --install`

```sh
git clone git@github.com:woosal1337/attune.git
cd attune
pnpm install
pre-commit install
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push
```

### Run the desktop app

```sh
pnpm tauri dev
```

First launch compiles the Rust workspace (~30 s on a warm cache). Subsequent launches reuse the cache.

### Run the CLI test harness

```sh
# List input devices
cargo run -p attune-cli --release -- devices

# Record from the default device for 60 seconds
cargo run -p attune-cli --release -- record --seconds 60

# Record from a specific device
cargo run -p attune-cli --release -- record --seconds 60 --mic-device "MacBook Pro Microphone"
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

# Frontend
pnpm typecheck
pnpm lint
pnpm format:check
pnpm test
```

`pre-commit` runs the relevant subset on every commit; the full suite also runs in `.github/workflows/ci.yml`.

## Contributing

See [`CONTRIBUTING.md`](./CONTRIBUTING.md). Security issues go through [`SECURITY.md`](./SECURITY.md).

## License

MIT. See [LICENSE](./LICENSE).
