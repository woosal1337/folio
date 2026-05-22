# Architecture

This is the on-disk architecture of the repository. For the higher-level
product design (why we capture mic and system audio separately, why the
v0 transcription path uses OpenAI Whisper rather than local
`whisper.cpp`, the future Swift app shell, etc.) see the design vault
referenced from `README.md`.

## Top-level layout

```
attune/
├── Cargo.toml                       # workspace root + shared deps + lints + profiles
├── rust-toolchain.toml              # pinned Rust 1.88, both Apple targets
├── rustfmt.toml, clippy.toml        # Rust formatting + linting policy
├── deny.toml                        # supply-chain audit policy (cargo-deny)
├── _typos.toml                      # spell-check allowlist
├── eslint.config.js                 # flat ESLint config
├── .prettierrc.json, .prettierignore
├── .pre-commit-config.yaml          # local CI mirror
├── .github/
│   ├── workflows/ci.yml             # rust + frontend + deny + typos
│   ├── ISSUE_TEMPLATE/, PULL_REQUEST_TEMPLATE.md
│   ├── CODEOWNERS, dependabot.yml
├── crates/
│   ├── attune-core/                 # the library — see § attune-core
│   └── attune-cli/                  # test harness binary
├── src-tauri/                       # the Tauri desktop binary
└── src/                             # the React frontend
```

## attune-core (`crates/attune-core/`)

The framework-agnostic library. Talks to the OS for audio; produces
WAV files on disk; will talk to OpenAI for transcription. Designed to be
embedded by either the Tauri desktop app, the CLI test harness, or a
future Swift app via UniFFI.

```
src/
├── lib.rs                # crate root, module declarations + re-exports
├── error.rs              # AttuneError — the single public error enum
├── audio/                # capture pipeline
│   ├── mod.rs            # Channel + CaptureConfig
│   ├── capture.rs        # CaptureSession orchestrator + RecordingStatus/Result
│   ├── devices.rs        # list_input_devices, DeviceInfo
│   ├── mic.rs            # MicCapture (cpal)
│   ├── system.rs         # SystemCapture (ScreenCaptureKit, macOS only)
│   ├── resampler.rs      # StreamingResampler (rubato polyphase)
│   └── wav_writer.rs     # AudioWavWriter (hound, mono 16-bit PCM)
├── storage/              # persistence
│   ├── mod.rs            # re-exports
│   ├── settings.rs       # Settings + SettingsStore (atomic JSON)
│   └── session.rs        # RecordingSummary + scan_recordings
├── transcription/        # pluggable STT backends
│   ├── mod.rs            # Transcriber trait + Transcript + TranscriptSegment
│   ├── openai.rs         # OpenAiTranscriber (scaffold)
│   └── stub.rs           # StubTranscriber (no-op for tests)
└── ffi/                  # UniFFI surface (placeholder)
    └── mod.rs
```

### Rules

- `AttuneError` is the single public error type. New error categories
  are added there, not invented per module.
- Logging uses `tracing`, never `println!`. Audio callbacks are
  alloc-free hot paths; do not log from inside the cpal /
  ScreenCaptureKit callback bodies.
- macOS-specific code is gated by `#[cfg(target_os = "macos")]` and
  has a stub for non-macOS targets so the workspace still builds.
- Types that cross the Tauri IPC boundary derive `ts_rs::TS` with
  `#[ts(export, export_to = "../../../src/shared/types/")]`. `cargo
test` regenerates the bindings; CI catches drift.

## src-tauri (`src-tauri/`)

The Tauri 2 desktop binary. Thin wrapper: imports `attune-core`,
exposes commands to the React frontend, and owns macOS-specific
window glue (Dock icon).

```
src/
├── main.rs               # binary entry, prevents the Windows console window
├── lib.rs                # tauri::Builder setup: plugins, state, invoke_handler
├── app/
│   ├── mod.rs
│   ├── state.rs          # AppState (settings + SettingsStore + session + timer)
│   └── dock_icon.rs      # macOS Dock icon helper (uses cocoa, marked deprecated)
└── commands/             # one module per domain
    ├── mod.rs
    ├── health.rs         # ping
    ├── devices.rs        # list_input_devices
    ├── settings.rs       # get_settings, save_settings (persists via SettingsStore)
    ├── recording.rs      # recording_status, start_recording, stop_recording
    └── library.rs        # list_recordings, delete_recording, reveal_in_finder
```

### IPC contract

Every `#[tauri::command]` is the contract with the frontend. Command
names and argument shapes are stable; renaming one is a breaking
change. Argument and return types are defined in `attune-core` and
generated as TypeScript by `ts-rs`:

| Command              | Args                 | Returns                      |
| -------------------- | -------------------- | ---------------------------- |
| `ping`               | `name?: string`      | `string`                     |
| `list_input_devices` | —                    | `DeviceInfo[]`               |
| `get_settings`       | —                    | `Settings`                   |
| `save_settings`      | `settings: Settings` | `void` (persists atomically) |
| `recording_status`   | —                    | `RecordingStatus`            |
| `start_recording`    | —                    | `RecordingStatus`            |
| `stop_recording`     | —                    | `RecordingResult`            |
| `list_recordings`    | —                    | `RecordingSummary[]`         |
| `delete_recording`   | `sessionDir: string` | `void`                       |
| `reveal_in_finder`   | `path: string`       | `void`                       |

Errors flow back as JSON strings on the `Err` side of the Result. The
frontend wraps them in `IpcError` for transport failures; domain
errors come through as strings.

## src/ (React frontend)

Feature-based layout.

```
src/
├── App.tsx               # router + providers (ErrorBoundary, Toaster)
├── main.tsx              # React mount, applyInitialTheme before paint
├── error-boundary.tsx    # root render-error fallback
├── shared/
│   ├── ui/               # shadcn primitives (button, dialog, …)
│   ├── lib/
│   │   ├── ipc.ts        # typed wrappers around invoke() + IpcError
│   │   └── utils.ts      # cn, formatDuration, formatBytes
│   ├── stores/
│   │   ├── recording-store.ts  # Zustand: session state + timer
│   │   └── settings-store.ts   # Zustand: cached Settings
│   ├── hooks/
│   │   ├── use-theme.ts        # light/dark + localStorage
│   │   └── use-window-drag.ts  # Tauri window drag/maximize handlers
│   └── types/                  # GENERATED by ts-rs — do not hand-edit
├── features/
│   ├── recording/        # Record page + audio player + recording row
│   ├── library/          # placeholder route
│   ├── editor/           # placeholder route
│   ├── tasks/            # placeholder route
│   └── settings/         # modal route + 4 section components
├── chrome/               # window chrome (sidebar, drag-strip)
└── styles/
    └── globals.css       # Tailwind layers + CSS-variable theme tokens
```

### Rules

- `@/shared/types/*` is the single source of truth for IPC types.
  Never define a Tauri-side type by hand in TS; add it to `attune-core`
  with a `TS` derive and re-run `cargo test`.
- Cross-route state lives in Zustand stores under `shared/stores/`.
  Page-local state stays in `useState` inside the feature.
- Tauri calls go through `shared/lib/ipc.ts`. Components never call
  `invoke` directly.
- The error boundary in `error-boundary.tsx` catches render errors;
  the `sonner` Toaster mounted in `App.tsx` surfaces non-fatal IPC
  failures with a description.

## Data flow

```
┌─────────────────────────────────────────────────────────┐
│              React (src/)                               │
│  features/* — Zustand stores — shared/lib/ipc.ts        │
└────────────────────┬────────────────────────────────────┘
                     │ invoke() — JSON over Tauri IPC
                     ▼
┌─────────────────────────────────────────────────────────┐
│              src-tauri/                                 │
│  commands/* — app/state.rs — attune-core re-exports     │
└────────────────────┬────────────────────────────────────┘
                     │ direct fn calls
                     ▼
┌─────────────────────────────────────────────────────────┐
│              attune-core                                │
│  audio:: — storage:: — transcription:: — ffi::          │
└────────────────────┬────────────────────────────────────┘
                     │ OS APIs (CoreAudio, ScreenCaptureKit, fs)
                     ▼
                  Disk + Hardware
```

## CI

`.github/workflows/ci.yml` runs on every push to `main` and every PR
against `main`. Jobs (all required):

- `rust-fmt` — `cargo fmt --all -- --check`
- `rust-clippy` — `cargo clippy --workspace --all-targets -- -D warnings`
- `rust-test` — `cargo build --workspace --all-targets`, then
  `cargo test --workspace --lib --bins`
- `rust-deny` — `cargo deny check`
- `typos` — `crate-ci/typos`
- `frontend` — `pnpm lint`, `pnpm typecheck`, `pnpm format:check`,
  `pnpm test`

`.pre-commit-config.yaml` mirrors most of these locally so the same
gates run on every commit, well before CI sees the branch.

## Conventions

See `AGENTS.md` for Rust-specific rules, `CONTRIBUTING.md` for the
human-facing setup and PR flow, and `SECURITY.md` for vulnerability
reporting.
