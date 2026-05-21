# Tauri Standards

Attune ships as a Tauri 2 desktop app. The Rust side hosts the audio
pipeline; the web side renders the UI. Communication is via Tauri's IPC
bridge.

## Project layout

- `src-tauri/` — the Rust application crate, named `attune-app`. Owns the
  Tauri builder, commands, plugins, and capabilities.
- `src/` — the React + TypeScript frontend, served by Vite in dev and
  bundled into `dist/` for builds.
- `crates/attune-core/` — pure-Rust library, used by both `attune-app`
  and `attune-cli`. No Tauri-specific code lives here.

## Commands

- Every Rust function callable from the frontend is `#[tauri::command]`,
  declared in `src-tauri/src/commands.rs`, and registered in
  `tauri::generate_handler![…]` in `lib.rs`.
- Commands take typed inputs (`State<'_, AppState>`, deserializable
  structs, primitive values) and return `Result<T, String>` for fallible
  operations. The frontend sees the `String` as the rejection reason.
- Long-running commands return quickly (start the work, return a handle
  or status). Polling commands (`recording_status`) are cheap, idempotent,
  and safe to call on every UI tick.
- Commands run on Tauri's runtime worker pool. They may be called from
  any thread; do not assume the same thread across two calls. Shared state
  goes through `tauri::State` with `parking_lot::Mutex`.

## Type sharing

- Frontend types in `src/lib/types.ts` mirror the Rust structs. Field
  names match exactly (snake_case both sides). Adding a field is a
  two-step change: Rust struct → TS type. The diff is small enough to do
  manually for v0.
- When type drift becomes painful, adopt `specta` + `tauri-specta` to
  generate TS types from Rust at build time. Tracked in `tooling.md`.
- The typed wrapper layer in `src/lib/api.ts` is the only file in the
  frontend that calls `invoke`. Screens import `startRecording()`, not
  `invoke('start_recording')`.

## Capabilities

- `src-tauri/capabilities/default.json` lists every permission the main
  window has. Add permissions when commands need them; never grant a
  capability speculatively.
- Path-scoped capabilities (`fs:allow-home-read`) are preferred over
  open access. The asset protocol scope in `tauri.conf.json` limits which
  paths the webview can load with `convertFileSrc`.
- The "ambient" Tauri permissions (`core:default`, `core:window:*`) are
  fine; the dangerous ones (`fs:allow-write-recursive` on `$HOME`) require
  explicit user value to justify.

## Window

- Single window in v0. `titleBarStyle: "Overlay"` + `hiddenTitle: true`
  gives a chromeless look with macOS traffic lights overlaying our
  custom drag strip.
- Drag regions are marked with `data-tauri-drag-region` AND wired to
  `startDragging()` because Tauri's data-attribute path silently fails on
  some macOS / WebKit combinations. See `src/hooks/use-window-drag.ts`.
- Minimum window size enforces a usable layout (`880×600`). Resizable but
  not full-screen by default.

## Plugins

- `tauri-plugin-opener` — open files/URLs in the platform default app.
- `tauri-plugin-dialog` — native file/folder pickers (used by the
  Settings → Storage section when folder pickers land).
- `tauri-plugin-fs` — scoped filesystem access. The scope is
  `$HOME/**` plus `/tmp/**` for the asset protocol.
- Add a plugin only when no Rust crate covers the platform call.

## Asset protocol

- `assetProtocol.enable = true` in `tauri.conf.json` lets the webview load
  local files via `convertFileSrc(path)` → `asset://...` URLs. This is how
  the audio player streams recorded WAVs.
- The protocol is scoped to `$HOME/**`, `/Users/**`, `/var/folders/**`,
  `/tmp/**`. Files outside the scope return 403.
- Never call `convertFileSrc` on a path the user did not implicitly
  choose. The scope is a defence in depth, not a primary safeguard.

## Tracing

- Initialize `tracing_subscriber` from `EnvFilter::try_from_default_env()`
  with a sensible fallback (`info,cpal=warn,reqwest=warn`). Users can
  raise the level by exporting `RUST_LOG`.
- Tauri commands log on entry/exit with structured fields. Spans are
  recommended for long-running operations.
- `tracing_subscriber::fmt().compact()` keeps log lines short; we don't
  enable JSON output by default to keep dev logs readable.

## State

- `AppState` is `#[derive(Default)]` and registered via `app.manage()` in
  `lib.rs::run()`. Each piece is its own `Mutex` so commands lock only
  what they touch.
- Audio capture state (`session`, `recording_started`) is held under
  separate mutexes from settings so saving settings during a recording
  does not stall the audio thread.
- Settings persistence to disk is on the roadmap; the doc string on
  `Settings` flags this. Until then, settings reset on app restart.

## Build profile

- Tauri inherits `[profile.release]` from the workspace `Cargo.toml`.
- Universal binary (`aarch64 + x86_64`) is built via the
  `--target universal-apple-darwin` flag once `cargo-bundle` is wired in
  the release workflow.
- Signing and notarisation are out of scope for v0 builds; Sparkle
  updates and the Homebrew Cask flow assume signed `.app` bundles and
  land with the v0 release plan.

## Dev mode quirks

- The Dock icon is set explicitly at startup via `dock_icon::set_dock_icon`
  on macOS because dev builds run a raw Mach-O binary without an `.app`
  bundle. Production builds pick up the icon from `Info.plist`.
- HMR works across the Tauri bridge; Rust changes require a `pnpm tauri dev`
  restart. Frontend changes hot-reload.
- `console.log` from the webview lands in the WebKit console, not stdout.
  Use `tracing` for everything that must reach the user via logs.

## Error surfaces

- Tauri commands return `Result<T, String>`. The frontend gets the string
  via the rejected promise; UI code formats it for display.
- Treat command errors as user-visible: the message should explain what
  to do, not just what failed. ("microphone permission missing — open
  System Settings → Privacy & Security → Microphone" beats "error code 1003".)
- Never panic in a command handler. A panic crashes the Tauri runtime
  thread that handled the invoke and leaves the frontend hanging.
