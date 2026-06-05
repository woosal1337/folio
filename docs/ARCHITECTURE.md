# Architecture

This is the on-disk architecture of the repository: the module map, the
IPC contract between the Rust core and the React frontend, and the key
design decisions (why mic and system audio are captured separately, why
the default transcription path is local Whisper rather than a cloud API).

## Top-level layout

```
folio/
├── Cargo.toml                       # workspace root + shared deps + lints + profiles
├── rust-toolchain.toml              # pinned Rust 1.88, both Apple targets
├── rustfmt.toml, clippy.toml        # Rust formatting + linting policy
├── deny.toml                        # supply-chain audit policy (cargo-deny)
├── _typos.toml                      # spell-check allowlist
├── eslint.config.js                 # flat ESLint config
├── .prettierrc.json, .prettierignore
├── .pre-commit-config.yaml          # local CI mirror
├── docs/
│   ├── ARCHITECTURE.md              # this document
│   └── guidelines/                  # Rust + Tauri + frontend deep-dives
├── scripts/
│   ├── check-no-telemetry.sh        # CI guard: no Sentry/Mixpanel/etc. in lock files
│   └── rasterize-icon.mjs           # icon prep for the app bundle
├── .github/
│   ├── workflows/ci.yml             # rust + frontend + deny + typos + no-telemetry
│   ├── ISSUE_TEMPLATE/, PULL_REQUEST_TEMPLATE.md
│   ├── CODEOWNERS, dependabot.yml
├── crates/
│   ├── folio-core/                 # the library — see § folio-core
│   └── folio-cli/                  # test harness binary
├── src-tauri/                       # the Tauri desktop binary
└── src/                             # the React frontend
```

## folio-core (`crates/folio-core/`)

The framework-agnostic library. Talks to the OS for audio capture;
produces WAV files on disk; runs local Whisper for transcription;
diarizes the system-audio track on-device (pyannote-segmentation-3.0
plus a WeSpeaker embedding model, both run through sherpa-onnx, with the
microphone always labelled **You**); owns the agent + memory + task
stores; talks to OpenAI for cloud transcription, chat completions, and
embeddings. Designed to be embedded by either the Tauri desktop app, the
CLI test harness, or a future Swift app via UniFFI.

The diarization pipeline lives in `diarization/`: `models.rs` resolves
and downloads the ONNX weights (gated by `cloud_guard`), `runtime.rs`
runs segmentation + embedding + clustering, `identify.rs` matches voice
clusters against the speaker registry, and `session_speakers.rs` /
`label.rs` turn clusters into the speaker labels surfaced in the note.

```
src/
├── lib.rs                # crate root, module declarations + re-exports
├── error.rs              # FolioError — the single public error enum
├── ask_folio.rs         # cross-library RAG citation contract
├── audio/                # capture pipeline (cpal + VPIO + ScreenCaptureKit)
├── calendar.rs           # EventKit + conference-URL helpers
├── cloud_guard.rs        # Privacy-Mode airgap toggle
├── diarization/          # on-device speaker diarization (sherpa-onnx)
│   ├── models.rs / runtime.rs / embedding.rs / identify.rs
│   ├── label.rs / session_speakers.rs / mod.rs
├── encryption.rs         # AES-256-GCM + Argon2id
├── evals.rs              # transcription quality eval helpers
├── ffi/                  # UniFFI surface (placeholder)
├── highlight_reel.rs     # decision-dense MP4 picker
├── import.rs             # Granola / Otter / Fathom switcher import
├── live_notes.rs         # /action /decision /question parser
├── llm/                  # AI providers + agents + chat plumbing
│   ├── agent_run.rs / agent_toml.rs / agents.rs / confidence.rs
│   ├── keystore.rs / live_agent.rs / local_llm.rs / marketplace.rs
│   ├── provider.rs / providers/openai.rs / rate_limit.rs / router.rs
│   ├── run_card.rs / skills.rs / templates.rs / types.rs
├── mcp_client.rs         # .folio/mcp.toml client config
├── mcp_server.rs         # folio-mcp tool surface
├── memory/               # Camp-2 context-substrate memory layer
│   ├── dream_loop.rs / embed.rs / embedding_cache.rs / embedding_provider.rs
│   ├── git_commit.rs / index.rs / page.rs / store.rs / types.rs / watcher.rs
├── onboarding.rs         # canned-demo bundle
├── paths.rs              # canonicalize_under helper (§8.1)
├── permissions.rs        # TCC walkthrough types
├── qos.rs                # macOS QoS class hints
├── share_page.rs         # public share-page payload
├── storage/              # persistence (settings, sessions, tasks, …)
│   ├── atomic_write.rs / decisions.rs / digest.rs / egress_log.rs
│   ├── fs_io.rs / git_sync.rs / retention.rs / session.rs / settings.rs
│   ├── share_bundle.rs / showcase.rs / snapshot.rs / spotlight.rs
│   ├── tasks.rs / vault_layout.rs
├── transcription/        # pluggable STT backends + chunking
│   ├── adaptive.rs / chunker.rs / hallucination_filter.rs / local.rs
│   ├── locate.rs / models.rs / model_lru.rs / openai.rs / stub.rs
│   ├── upload_state.rs / vad.rs
└── webhooks.rs           # signed outbound webhooks
```

### Rules

- `FolioError` is the single public error type. New error categories
  are added there, not invented per module.
- Logging uses `tracing`, never `println!`. Audio callbacks are
  alloc-free hot paths; do not log from inside the cpal /
  ScreenCaptureKit callback bodies.
- macOS-specific code is gated by `#[cfg(target_os = "macos")]` and
  has a stub for non-macOS targets so the workspace still builds.
- Types that cross the Tauri IPC boundary derive `ts_rs::TS` with
  `#[ts(export, export_to = "../../../src/shared/types/")]`. `cargo
test` regenerates the bindings; CI catches drift.
- Two-phase write for any file-backed store: write the canonical
  on-disk file first (`.md` for memory, `.json` for tasks), update
  the derived index second. Index is rebuildable from files.
- New deps land in `[workspace.dependencies]` first (`Cargo.toml`),
  then crates reference them with `{ workspace = true }`.

## src-tauri (`src-tauri/`)

The Tauri 2 desktop binary. Thin wrapper: imports `folio-core`,
exposes commands to the React frontend, and owns macOS-specific
window glue (Dock icon).

```
src/
├── main.rs               # binary entry, prevents the Windows console window
├── lib.rs                # tauri::Builder setup: plugins, state, invoke_handler
├── app/
│   ├── mod.rs / state.rs
│   ├── dock_icon.rs      # macOS Dock icon helper (cocoa)
│   ├── share_sheet.rs    # NSSharingServicePicker
│   ├── tray.rs           # menu-bar tray icon
│   └── vibrancy.rs       # NSVisualEffectView
└── commands/             # one module per domain
    ├── mod.rs / health.rs / devices.rs / settings.rs / recording.rs
    ├── library.rs        # list / get / delete / reveal_in_finder / share_paths / save_debrief
    ├── transcription.rs  # transcribe / read / save / locate_span / whisper_model_*
    ├── llm.rs            # list_providers / set_provider_key / test_provider / list_provider_models
    ├── agents.rs         # list / run / list_runs / delete_run
    ├── tasks.rs / memory.rs / maintenance.rs / captions.rs / webhooks.rs
    ├── permissions.rs    # list_permissions / open_permission_settings
    ├── preferences.rs    # open_preferences_window
    ├── tray.rs           # set_tray_recording bridge
    └── windows.rs        # open_record_window / open_library_window / open_editor_window
```

### Capabilities

`src-tauri/capabilities/` holds **one capability file per window class** per
`docs/CODE_STYLE.md` §8.4:

- `default.json` — main window only. Narrowed: no recursive `$HOME` fs grants;
  opener allowlist is per-host, not blanket `https://*`.
- `captions.json` — captions window. Renderer only; no fs / opener grants.
- `preferences.json` — Cmd-, NSWindow. No fs (everything funnels through
  canonicalised Tauri commands); opener limited to docs + Apple system settings.
- `secondary.json` — record-_, library-standalone, editor-_ secondary windows
  from. Same surface as main minus the home-recursive fs grants.

Add new URL schemes via the relevant capability file's
`opener:allow-open-url` allowlist — never in JS.

### IPC contract

Every `#[tauri::command]` is the contract with the frontend. Command
names and argument shapes are stable; renaming one is a breaking
change. Argument and return types are defined in `folio-core` and
generated as TypeScript by `ts-rs`. Browse `src/shared/lib/ipc.ts`
for the authoritative list; `cargo test` regenerates the bindings.

Errors flow back as JSON strings on the `Err` side of the Result.
The frontend wraps them in `IpcError` for transport failures; domain
errors come through as strings.

## src/ (React frontend)

Feature-based layout.

```
src/
├── App.tsx               # router + providers (ErrorBoundary, Toaster, modals)
├── main.tsx              # React mount, applyInitialTheme before paint
├── error-boundary.tsx    # root render-error fallback
├── shared/
│   ├── ui/               # shadcn primitives (button, dialog, meta-list, switch, …)
│   ├── lib/
│   │   ├── ipc.ts        # typed wrappers around invoke + IpcError
│   │   ├── utils.ts      # cn, formatDuration, formatBytes
│   │   ├── share.ts      # memoryToMarkdown, taskToMarkdown, obsidianHref, openInObsidian, copyToClipboard
│   │   ├── cost-estimate.ts  # Whisper $ + size estimator (used by cost-confirm modal)
│   │   ├── feedback.ts   # Web Audio synthesised lifecycle sounds
│   │   └── power.ts      # Web Battery API helpers
│   ├── stores/                   # Zustand
│   │   ├── recording-store.ts    # session state + timer + post-transcription chain
│   │   ├── settings-store.ts     # cached Settings
│   │   ├── settings-ui-store.ts  # Settings modal open/section state
│   │   ├── tasks-store.ts        # kanban data + optimistic CRUD
│   │   ├── memories-store.ts     # /memory page data + filters
│   │   ├── jobs-store.ts         # cross-cutting "in-flight job" pills
│   │   └── cloud-cost-confirm-store.ts  # promise-based confirm dialog
│   ├── hooks/
│   │   ├── use-theme.ts          # light/dark + localStorage
│   │   └── use-window-drag.ts    # Tauri window drag/maximize handlers
│   └── types/                    # GENERATED by ts-rs — do not hand-edit
├── features/
│   ├── recording/             # Record page + StatusPill + AudioPlayer + RecordingRow + voice-debrief
│   ├── library/               # Library list + filters + stats strip + quick-look sheet
│   ├── editor/                # Per-recording editor: transcript + audio + agents + briefing-card
│   ├── inbox/                 # /inbox — today's open actions + run-cards
│   ├── tasks/                 # Kanban with @dnd-kit + inline composer + edit dialog
│   ├── memory/                # /memory cards by kind, search, pin/archive
│   ├── captions/              # borderless caption window
│   ├── onboarding/            # one-screen first-run conductor
│   ├── preferences-window/    # /preferences-window route for the Cmd-, NSWindow
│   └── settings/              # in-app modal route (legacy; staged for removal)
├── chrome/                    # window chrome
│   ├── sidebar.tsx / drag-strip.tsx / job-strip.tsx
│   ├── cheatsheet-overlay.tsx / command-palette.tsx
│   ├── deep-link-handler.tsx / global-shortcuts.tsx
│   ├── cloud-cost-confirm-dialog.tsx
│   └── home-redirect.tsx      # `/` → /library when recordings exist
└── styles/
    └── globals.css       # Tailwind layers + CSS-variable theme tokens + prefers-reduced-motion
```

### Rules

- `@/shared/types/*` is the single source of truth for IPC types.
  Never define a Tauri-side type by hand in TS; add it to `folio-core`
  with a `TS` derive and re-run `cargo test`.
- Cross-route state lives in Zustand stores under `shared/stores/`.
  Page-local state stays in `useState` inside the feature.
- Tauri calls go through `shared/lib/ipc.ts`. Components never call
  `invoke` directly.
- For Zustand selectors, subscribe to raw fields and `useMemo` derived
  values inside the consumer — returning a new array/object reference
  from the selector triggers React's "Maximum update depth exceeded"
  guard. Use `useShallow` only for object selectors with stable keys.
- Sounds, motion, and confirm-dialog patterns route through their
  shared `lib/` helpers + Zustand stores so they can be triggered
  from any layer (recording-store, settings, agent panel) without
  prop drilling.
- The error boundary in `error-boundary.tsx` catches render errors;
  the `sonner` Toaster mounted in `App.tsx` surfaces non-fatal IPC
  failures with a description.

## Data flow

```
┌─────────────────────────────────────────────────────────┐
│              React (src/)                               │
│  features/* — Zustand stores — shared/lib/ipc.ts        │
└────────────────────┬────────────────────────────────────┘
                     │ invoke — JSON over Tauri IPC
                     ▼
┌─────────────────────────────────────────────────────────┐
│              src-tauri/                                 │
│  commands/* — app/state.rs — folio-core re-exports     │
└────────────────────┬────────────────────────────────────┘
                     │ direct fn calls
                     ▼
┌─────────────────────────────────────────────────────────┐
│              folio-core                                │
│  audio:: — llm:: — memory:: — storage:: — transcription:: │
└────────────────────┬────────────────────────────────────┘
                     │ OS APIs + OpenAI + whisper.cpp + SQLite
                     ▼
                  Disk + Hardware + Network
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
- `no-telemetry` — `scripts/check-no-telemetry.sh` (v2 R11)
- `frontend` — `bun run lint`, `bun run typecheck`,
  `bun run format:check`, `bun run test`

`.pre-commit-config.yaml` mirrors most of these locally so the same
gates run on every commit, well before CI sees the branch.

## Workflow — Linear + GitHub + Obsidian

This project tracks work in three places that round-trip:

| Layer                           | What lives there                                                                        |
| ------------------------------- | --------------------------------------------------------------------------------------- |
| **Issue tracker**               | Status, priority, assignment. The live source of truth. Issues are `GET-<n>` (e.g. ``). |
| **GitHub** (`woosal1337/folio`) | Code, issues, and pull requests.                                                        |

### Workflow for a new feature

1. **Pick a Linear issue** from the backlog (sort by priority desc, size asc within a tier).
2. **Move it to In Progress** with `mcp__linear-server__save_issue` (state: `In Progress`).
3. **Branch** from `main`: `feat/<linear-id>-<slug>` or `fix/<linear-id>-<slug>`
   (e.g. `feat/get-49-collapsible-sidebar`).
4. **Implement.** Follow the guideline doc for the area you're touching:
   - Rust changes — `docs/guidelines/rust-architecture.md`, `rust-error-handling.md`, `rust-async.md`
   - Tauri commands — `docs/guidelines/tauri-architecture.md`
   - Audio callback paths — `docs/guidelines/audio-pipeline.md` (**§1 is non-negotiable**)
   - React frontend — `docs/guidelines/frontend-architecture.md`
5. **Run gates locally** before commit: `bun run typecheck && bun run lint && bun run test && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.
6. **Commit signed**: `git -c commit.gpgsign=true commit -m "<type>: <linear-id> — <one-line>"`. Do NOT include `Co-Authored-By` trailers.
7. **Push + open PR**. Title format: `<type>(<linear-id>): <summary>`. Body references the Linear issue: `Closes GET-<n>` so Linear auto-resolves.
8. **Merge** with `gh pr merge --merge --delete-branch` once gates are green.
9. **Mark issue Done** in Linear with the PR URL attached via `mcp__linear-server__save_issue` `links: [{ url, title }]`.

### Commit message format

```
<type>(<linear-id>): <one-line summary in present tense, lowercase>

<optional body explaining WHY, not WHAT — the diff explains what.
Wrap at 72 chars. Reference design docs / prior PRs / Linear issue
ids as needed.>
```

Types: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`,
`style`, `build`, `ci`.

Linear id is optional for code that doesn't map to a roadmap item
(e.g. `chore: bump dependencies`), but include it whenever there is
a corresponding issue.

**No `Co-Authored-By` trailers.** This is explicit user policy.

### PR description template

```markdown
## Summary

Closes GET-<n>.

<1-3 bullets summarising the change>

## Test plan

- [ ] Reproducible test step 1
- [ ] Reproducible test step 2
      …

## Gates

- cargo test + clippy + fmt
- bun typecheck + lint + test + format:check
- pre-commit hooks (taplo-lint excepted if upstream broken)
```

## Code styling — quick reference

For the full source-cited guidance see `docs/guidelines/`. This is the
condensed cheat sheet.

### Rust

- **Format**: `cargo fmt` (rustfmt config in `rustfmt.toml`). Run
  before every commit; pre-commit hook enforces.
- **Lint**: `cargo clippy --workspace --all-targets -- -D warnings`.
  Treat warnings as errors. Use `#[allow(clippy::name)]` with a
  one-line comment explaining why, never workspace-wide unless
  there's a known false positive.
- **Errors**: return `Result<T, FolioError>`. New error categories
  go on the public enum in `folio-core/src/error.rs`; never invent
  per-module error types.
- **Async**: prefer `tauri::async_runtime::spawn_blocking` for any
  IPC command that touches the filesystem, SQLite, or whisper.cpp —
  keeps the Tauri runtime free. See `rust-async.md`.
- **Locks**: `parking_lot::Mutex` for sync code; `tokio::sync::Mutex`
  only when held across `.await`. Never hold either across a network
  call.
- **Naming**: `snake_case` for modules / functions / variables,
  `PascalCase` for types / enums / traits, `SCREAMING_SNAKE` for
  consts.
- **Visibility**: prefer `pub(crate)` over `pub` unless the symbol
  is part of the external API. New `pub` types crossing the IPC
  boundary must derive `Serialize`, `Deserialize`, and `ts_rs::TS`.
- **Tests**: every public function gets at least a round-trip /
  happy-path test in the same module under `#[cfg(test)]`.
  Integration tests in `tests/`. CI runs `cargo test --workspace
--lib --bins` — the `--lib --bins` part is intentional (no doctests
  in CI; they go in `cargo test --doc` locally).
- **Doc comments**: `///` on every `pub` item. First line is a
  noun phrase. Body explains WHY when non-obvious, never just WHAT.

### TypeScript / React

- **Format**: `bun x prettier --write`. The pre-commit hook runs
  prettier on the staged files.
- **Lint**: `bun run lint` (`eslint src --max-warnings 0`). Same
  zero-warning policy as Rust.
- **Files**: `kebab-case.tsx` / `kebab-case.ts`. Components export
  one default per file when the file IS the component; named exports
  for utilities.
- **Imports**: order — React/3rd party → lucide-react → shadcn UI →
  `@/shared/...` → `@/features/...` → relative. Prettier import-
  sort runs automatically.
- **State**: Zustand for cross-route state, `useState` for local.
  Selectors return primitives (or stable references); derive arrays
  - objects with `useMemo` inside the component.
- **IPC**: only `src/shared/lib/ipc.ts` imports
  `@tauri-apps/api/core`. Every command goes through a typed
  wrapper there.
- **Types**: never hand-edit `@/shared/types/*` — those are
  generated by `ts-rs`. Add the type to the Rust crate, run
  `cargo test`, the binding regenerates.
- **Styling**: Tailwind utilities + CSS variables in
  `globals.css`. No hard-coded hex colours in components. Use
  `text-foreground`, `bg-card`, etc.
- **Motion**: honour `prefers-reduced-motion`. Globals handle it
  for built-in Tailwind transitions; explicit `framer-motion` or
  `@dnd-kit` motion goes through the `motion-safe:` / `motion-reduce:`
  Tailwind variants.

### Markdown

- Code comments in `.md` use fenced blocks with language tags
  (`rust`, `tsx`, `bash`, `json`).
- Em-dash + en-dash usage: inside this repo we use whichever reads
  best; PR descriptions stay terse.

## Adding a new feature — checklist

Use this when starting work on any Linear issue.

```
[ ] Linear issue: GET-<n> moved Todo → In Progress (mcp__linear-server__save_issue)
[ ] Read the relevant docs/guidelines/*.md
[ ] Branch from main: <type>/get-<n>-<slug>
[ ] If the type crosses IPC: add to folio-core with ts_rs derive; run `cargo test`
[ ] If the type adds a Tauri command: register it in src-tauri/src/lib.rs invoke_handler
[ ] Add a typed wrapper in src/shared/lib/ipc.ts
[ ] Component / store / hook lives under shared/ or features/ per FSD
[ ] New deps: workspace-level Cargo.toml first, then `{ workspace = true }` in the crate
[ ] CSS variables only — no hex literals in JSX
[ ] Tests: at minimum a round-trip + an error-path test
[ ] Run all gates locally (see Workflow §5)
[ ] Signed commit, no Co-Authored-By
[ ] PR title: <type>(get-<n>): <summary>; body has Closes GET-<n>
[ ] Merge with --merge --delete-branch
[ ] Linear issue → Done with PR url attached as a link
```

## Conventions

See `docs/guidelines/` for the deep-dives, `AGENTS.md` for Rust-
specific rules, `CONTRIBUTING.md` for the human-facing setup and PR
flow, and `SECURITY.md` for vulnerability reporting.

## Removed scope

The following capabilities were explicitly removed from the roadmap.
Future agents should not re-introduce them without an updated decision
record in `docs/`.

| Capability                            | Removed in | Replaced by                                                  | Reason                                                                                                                                                                                                 |
| ------------------------------------- | ---------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `folio-api` cloud sync service        |            | Git remote sync                                              | Duplicates `git push`. Introduces account, server, billing surface, attack surface, and outage mode. Sync over the user's own git remote covers the same use case with zero new server infrastructure. |
| In-app Settings modal                 |            | Real Preferences NSWindow                                    | Cmd-, now opens a separate 640×520 NSWindow rendered at `/preferences-window`. The in-app modal stays for one release as a fallback, then gets removed in a tiny cleanup PR.                           |
| Flat reverse-chronological `/ai` page | +          | Unified `/inbox` route with today's open actions + run-cards | The flat agent-runs list was a debugging artifact. The Inbox shows what needs you today, and the editor's run-cards subsume the old per-recording detail.                                              |
| Manual "Reindex" memory button        |            | Debounced fs-watch + auto-reindex orchestrator               | The manual button was a debugging artifact. External edits (Obsidian, `git pull`) now trigger a debounced background reindex automatically.                                                            |
