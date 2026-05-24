# Tauri 2 architecture — Attune guidelines

Source-cited guidance for the `src-tauri` shell and the IPC boundary with the React frontend. Targets Tauri 2.x. Updated 2026-05-24.

The guiding philosophy: **thin shell, fat domain.** `src-tauri` is glue between the WebView and the pure-Rust `attune-core`. All audio, transcription, and pipeline logic lives in `attune-core`, untainted by `tauri::*` types so it stays testable, swappable, and reusable from CLIs or future plugins.

## TL;DR — the rules

1. **Commands are IPC adapters, ≤ ~30 lines each.** Parse → call domain service → map error. No business logic in `#[tauri::command]` bodies. ([Calling Rust](https://v2.tauri.app/develop/calling-rust/))
2. **Return `Result<T, AppError>`, never `Result<T, String>`.** Tagged enum that serializes as `{ kind, message }` so the frontend branches on `kind`. ([dev.to](https://dev.to/hiyoyok/rust-error-handling-in-tauri-commands-the-pattern-that-actually-works-35le))
3. **One `AppState` struct with sub-handles, not a soup of `Mutex<T>`.** Commands take `State<'_, AppState>`. Mutex types never leak into command signatures. ([State Management](https://v2.tauri.app/develop/state-management/))
4. **Events for fan-out / lifecycle, channels for streams.** "The event system is not designed for low latency or high throughput situations." ([Calling Frontend](https://v2.tauri.app/develop/calling-frontend/))
5. **One capability file per feature area** under `src-tauri/capabilities/`, least-privilege scoped, listed explicitly in `tauri.conf.json`. ([Capabilities](https://v2.tauri.app/security/capabilities/))
6. **`lib.rs` stays under 100 lines** — builder, plugin registrations, state init, `generate_handler!` list. Everything else moves to sibling modules.

## Command design

A `#[tauri::command]` is an IPC adapter, nothing more. Its job is: deserialize args, fetch handles, call into `attune-core`, serialize the result.

From the Tauri 2 docs:

> "If your application defines a lot of components or if they can be grouped, you can define commands in a separate module instead of bloating the `lib.rs` file." — [Calling Rust](https://v2.tauri.app/develop/calling-rust/)

> "Everything returned from commands must implement `serde::Serialize`, including errors... You may want to create your own error type which implements `serde::Serialize`."

> "Borrowed arguments like `&str` and `State<'_, Data>` are unsupported [in async commands]. Workarounds: take owned types (`String`), or wrap the return in `Result<T, E>`."

**Rules for Attune:**

- Command signature is always `async fn` and always returns `Result<T, AppError>`. Even when "this can't fail today" — the type contract should pre-allocate the failure slot.
- No business logic inside a command body. Commands read like:

  ```rust
  #[tauri::command]
  pub async fn start_recording(
      state: State<'_, AppState>,
      opts: StartRecordingOpts,
  ) -> Result<RecordingId, CommandError> {
      let session = state.recording().start(opts.into_domain()).await?;
      Ok(session.id().into())
  }
  ```

- `StartRecordingOpts` / `RecordingId` are **wire DTOs** that live in `src-tauri/src/dto/`. They `impl From<DomainType>` and vice versa. `attune-core` never sees these.
- Async commands take owned `String` / `PathBuf`, never `&str`.

**What does NOT belong in a command body:**

- Audio device enumeration logic
- File I/O sequencing
- Whisper inference orchestration
- Retry loops
- Mixing two tracks

All of that lives in `attune-core` behind a trait like `RecordingService`, `TranscriptionService`, called from a command via the `AppState`.

**Long-running work:** a command should return in milliseconds. If the work takes longer (transcription, model download), the command should _spawn_ it on a background task owned by the state subsystem and return a handle. Subscribe to progress via a `tauri::ipc::Channel<T>` or `app.emit_to(...)`.

## Tagged error enum

> "Since the return type must implement `serde::Serialize`, most errors don't work directly... The `thiserror` + tagged enum pattern is the correct default for Tauri app error handling. Set it up on day one." — [dev.to](https://dev.to/hiyoyok/rust-error-handling-in-tauri-commands-the-pattern-that-actually-works-35le)

Pattern Attune should adopt:

```rust
// src-tauri/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("model not installed")]
    ModelMissing,

    #[error("recording session is busy")]
    SessionBusy,

    #[error("permission denied")]
    PermissionDenied,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl serde::Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("CommandError", 2)?;
        let kind = match self {
            CommandError::ModelMissing => "ModelMissing",
            CommandError::SessionBusy => "SessionBusy",
            CommandError::PermissionDenied => "PermissionDenied",
            CommandError::Other(_) => "Other",
        };
        st.serialize_field("kind", kind)?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}
```

The frontend then branches without parsing English:

```ts
try {
  await invoke("transcribe_recording", { id });
} catch (e: any) {
  if (e?.kind === "ModelMissing") showDownloadCta();
  else if (e?.kind === "PermissionDenied") promptForPermission();
  else toast.error(e?.message ?? String(e));
}
```

## State management

> "It is ok and often preferred to use the ordinary `Mutex` from the standard library in asynchronous code." — [State Management](https://v2.tauri.app/develop/state-management/)

> "You don't need to use `Arc` for things stored in `State` because Tauri will do this for you."

> "If you use the wrong type for the `State` parameter, you will get a runtime panic instead of a compile-time error." — [Manager trait](https://docs.rs/tauri/latest/tauri/trait.Manager.html)

**Mutex choice:** default to `std::sync::Mutex` or `parking_lot::Mutex`. Use `tokio::sync::Mutex` only when the guard must be held across `.await` — see [`rust-async.md`](./rust-async.md). The recording pipeline almost certainly should NOT hold a lock across `.await`; it should send frames over an `mpsc` channel and let the consumer own the buffer.

**Layout: one `AppState`, sub-handles per subsystem.**

```rust
// src-tauri/src/app/state.rs
pub struct AppState {
    recording: RecordingManager,        // owns its own internal Mutex / channels
    transcription: TranscriptionEngine, // owns a worker thread + mpsc
    settings: SettingsStore,            // tiny, fine to lock briefly
    llm: LlmRegistry,
}

impl AppState {
    pub fn recording(&self) -> &RecordingManager { &self.recording }
    pub fn transcription(&self) -> &TranscriptionEngine { &self.transcription }
    pub fn settings(&self) -> &SettingsStore { &self.settings }
    pub fn llm(&self) -> &LlmRegistry { &self.llm }
}
```

In `lib.rs`:

```rust
tauri::Builder::default()
    .setup(|app| {
        let state = AppState::initialize(app.handle())?;
        app.manage(state);
        Ok(())
    })
    .invoke_handler(tauri::generate_handler![/* … */])
    .run(tauri::generate_context!())?;
```

Commands take `State<'_, AppState>`, never `State<'_, Mutex<Foo>>`. The `Mutex` type never leaks into a command signature. You can refactor synchronization internally without touching command IPC contracts.

**Don't put a raw `Mutex<Subsystem>` in `State`** — it serializes every command and turns the app into a queue. Instead, wrap each subsystem in a handle type that itself uses fine-grained internal synchronization or message passing.

## Events vs channels vs commands

From [Calling Frontend](https://v2.tauri.app/develop/calling-frontend/):

> "The event system was designed for situations where small amounts of data need to be streamed or you need to implement a multi consumer multi producer pattern."

> "The event system is not designed for low latency or high throughput situations."

> "Channels are designed to be fast and deliver ordered data. They are used internally for streaming operations such as download progress, child process output and WebSocket messages."

| Need                                                                           | Use                                                                   |
| ------------------------------------------------------------------------------ | --------------------------------------------------------------------- |
| One-shot RPC, frontend asks → backend answers                                  | `#[tauri::command]`                                                   |
| Continuous high-rate stream (partial transcript, VU meter, waveform samples)   | `tauri::ipc::Channel<T>` returned from an `init_*_stream` command     |
| Cross-cutting notification (session ended, settings changed, model downloaded) | `app.emit(...)` (global) or `app.emit_to(label, ...)` (window-scoped) |
| One window must talk to another window only                                    | `emit_to(label, ...)`                                                 |

**Avoid chatty events.** Don't emit a `frame_captured` event per PCM frame — events are JSON-encoded and pumped through the WebView bridge. Use a `Channel<TranscriptDelta>` for partial transcripts and batch waveform updates at ≤ 30 Hz.

## Typed payloads — `ts-rs` vs `tauri-specta`

Attune currently uses `ts-rs` (cross-language type generation). It works but has limits:

- Type-by-type generation; doesn't recursively export dependent types.
- No typed event support.
- Manual TS wrapper functions in `src/shared/lib/ipc.ts`.

`tauri-specta` v2 (current best-in-class) generates TypeScript for commands AND events, eliminating manual `invoke<T>` and `listen<T>` wrappers that drift from Rust. Migrate when:

- The IPC surface grows past ~30 commands, or
- You start needing typed events (the bigger lever), or
- The manual TS wrapper duplication becomes painful.

Migration is mechanical: add `#[specta::specta]` next to `#[tauri::command]`, register with `tauri_specta::ts::export`, replace `src/shared/lib/ipc.ts` with the generated `bindings.ts`.

**Naming convention for event names**: `domain:verb-noun` (e.g. `recording:state-changed`, `transcription:partial`, `settings:updated`). Define the strings once in a constants module on both sides, never bare string literals at call sites.

## Plugins

From [Plugin Development](https://v2.tauri.app/develop/plugins/):

> "By design, the Tauri core does not contain features not needed by everyone. Instead it offers a mechanism to add external functionalities into a Tauri application called plugins."

Extract into a `tauri-plugin-*` when **all three** are true:

1. The functionality has its own lifecycle (init/teardown), permissions, and commands.
2. It is, or might plausibly become, useful in another Tauri app or to the community.
3. It needs its own capability namespace and JS API package.

For Attune:

- **Keep in `src-tauri`**: recording control, settings, app menu, dock icon. They're app-specific glue.
- **Candidate for an internal plugin crate**: macOS system-audio capture via ScreenCaptureKit if it grows beyond a single file. Lifecycle, permissions, OS-specific Swift package — that's plugin-shaped.
- **Already-used official plugins**: `tauri-plugin-opener`, `tauri-plugin-dialog`, `tauri-plugin-fs`. Add `tauri-plugin-store` for persisted settings, `tauri-plugin-log` for log file rotation.

## Capabilities & permissions

From [Capabilities](https://v2.tauri.app/security/capabilities/):

> "Capability files are either defined as a JSON or a TOML file inside the `src-tauri/capabilities` directory."

> "It is good practice to use individual files and only reference them by identifier in the `tauri.conf.json`."

> "All capabilities inside the `capabilities` directory are automatically enabled by default. Once capabilities are explicitly enabled in the `tauri.conf.json`, only these are used in the application build."

Rules for Attune:

- One capability file per feature area: `capabilities/recording.json`, `capabilities/transcription.json`, `capabilities/settings.json`, `capabilities/core.json`. Don't pile everything into `default.json`.
- Each file lists `windows: ["main"]` (or the actual labels used), and only the permissions that window needs.
- Custom Attune commands are NOT automatically callable from JS in Tauri 2 — capability files must grant them.
- Be explicit in `tauri.conf.json` about which capability identifiers are active per build target so dev-only capabilities don't ship.
- Prefer scoped `fs` permissions over blanket access. The capability `scope` field is the Tauri 2 replacement for the v1 allowlist's path globs.

## Recommended project layout

From [Project Structure](https://v2.tauri.app/start/project-structure/):

> "`src/lib.rs` contains the Rust code and the mobile entry point... `src/main.rs` is the main entry point for the desktop, and we run `app_lib::run()` in `main`."

Target tree for Attune (current vs target):

```
src-tauri/
  capabilities/
    core.json
    recording.json
    transcription.json
    settings.json
    llm.json
  icons/
  tauri.conf.json
  build.rs
  Cargo.toml
  src/
    main.rs                   # 3 lines: call attune_app_lib::run()
    lib.rs                    # ≤ 100 lines: Builder, manage(AppState), generate_handler!
    app/
      mod.rs
      state.rs                # AppState struct + initialize()
      dock_icon.rs            # macOS dock icon plumbing
    error.rs                  # CommandError (thiserror + Serialize)
    dto/                      # wire types (StartRecordingOpts, RecordingId, …)
      mod.rs
      recording.rs
      transcription.rs
    commands/                 # thin IPC handlers, grouped by domain
      mod.rs
      recording.rs
      transcription.rs
      settings.rs
      llm.rs
      agents.rs
      library.rs
      maintenance.rs
      devices.rs
      health.rs
    events/                   # typed event structs + name constants
      mod.rs
    workers/                  # background tasks owned by state subsystems
      mod.rs
      transcription_worker.rs
```

Current state vs target:

- ✅ Per-domain command files already exist under `src-tauri/src/commands/`.
- ✅ `lib.rs` is currently ~66 lines.
- ❌ No `error.rs` with `CommandError` yet — commands currently return `Result<T, String>`. Add this.
- ❌ No `dto/` directory — wire types live alongside the commands that use them. Extract once you have ≥ 3 commands sharing a DTO.
- ❌ No `events/` directory — events emitted with bare string literals. Centralize when you have ≥ 5 event names.
- ❌ No `workers/` directory — long-running work currently runs in-line in commands or is spawned ad-hoc. Promote to a worker when the operation has a lifecycle (pause/resume/cancel).

## Checklist for new commands

- [ ] Body ≤ 30 lines, all real work delegated to `attune-core`.
- [ ] Returns `Result<T, CommandError>`, not `Result<T, String>`.
- [ ] Async commands take owned args (no `&str`).
- [ ] No `Mutex<T>` type in the command signature; commands take `State<'_, AppState>`.
- [ ] No new event names as inline string literals; defined in `events/mod.rs`.
- [ ] High-rate streams use `tauri::ipc::Channel<T>`, not `emit`.
- [ ] New command listed in the relevant `capabilities/*.json`.
- [ ] Types crossing the boundary derive `ts_rs::TS` (or `specta::Type` after migration).
- [ ] If the feature has its own lifecycle, permissions, and reusability — consider a `tauri-plugin-*` instead.

## Sources

- [Calling Rust from the Frontend — Tauri v2](https://v2.tauri.app/develop/calling-rust/)
- [Calling the Frontend from Rust — Tauri v2](https://v2.tauri.app/develop/calling-frontend/)
- [State Management — Tauri v2](https://v2.tauri.app/develop/state-management/)
- [Plugin Development — Tauri v2](https://v2.tauri.app/develop/plugins/)
- [Capabilities — Tauri v2](https://v2.tauri.app/security/capabilities/)
- [Permissions — Tauri v2](https://v2.tauri.app/security/permissions/)
- [Project Structure — Tauri v2](https://v2.tauri.app/start/project-structure/)
- [tauri-specta on GitHub](https://github.com/specta-rs/tauri-specta)
- [tauri/plugins-workspace](https://github.com/tauri-apps/plugins-workspace)
- [Rust Error Handling in Tauri Commands](https://dev.to/hiyoyok/rust-error-handling-in-tauri-commands-the-pattern-that-actually-works-35le)
- [Tauri error handling recipes](https://tbt.qkation.com/posts/tauri-error-handling/)
- [Tauri Discussion #8538 — Share AppState between threads](https://github.com/tauri-apps/tauri/discussions/8538)
- [Tauri Discussion #6952 — Return errors as JSON](https://github.com/tauri-apps/tauri/discussions/6952)
- [Manage Global State in Tauri](https://tauritutorials.com/blog/manage-global-state-in-tauri)
- [Manager trait — docs.rs](https://docs.rs/tauri/latest/tauri/trait.Manager.html)
