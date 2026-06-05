# 09 — Tauri v2 (Commands, State, Security)

From the Tauri v2 docs (Calling Rust, State Management, Security, Capabilities).
`CODE_STYLE.md` §6.3 (canonical command shape), §8 (security), §9.4 (IPC contract)
are canon; `docs/guidelines/tauri-architecture.md` is the subsystem doc.

## 1. Command design

- Mark exposed fns `#[tauri::command]`; register them in a **single**
  `invoke_handler(tauri::generate_handler![...])` (only the last call wins).
- **Keep commands thin and async** for any IO/blocking work so the UI thread
  never freezes. Folio's canonical shape: clone the needed `State` field, do the
  work in `spawn_blocking`, convert errors at the boundary.
- Use **owned** argument types (`String`, not `&str`) in async command signatures
  (borrowed refs won't compile across the await).
- **Return `Result<T, E>`; never `unwrap`/`panic` in a command** (a panic crashes
  the backend; `Err` becomes a rejected Promise).
- **Every return value and error type must be `serde::Serialize`** (it crosses
  IPC). Folio converts `FolioError` → `String` at the boundary.
- Validate every command argument at entry (ranges, formats, path traversal) —
  IPC input is untrusted.
- Return large/binary payloads via `tauri::ipc::Response`; stream via `Channel<T>`.
- Take `AppHandle`/`WebviewWindow` as params for app-/window-scoped actions.

## 2. Managed state

- Register state once with `app.manage(...)` in `setup`; read via
  `tauri::State<'_, T>` params. `State<T>` already shares ownership across
  threads — **don't add your own `Arc`.**
- Wrap mutable state in a `Mutex`/`RwLock`; state must be `Send + Sync`.
- Default to `std`/`parking_lot::Mutex`; `tokio::sync::Mutex` only when a lock
  must be held across `.await`.
- Release locks promptly (drop the guard at scope end); never hold across a long/
  awaiting op. Access state outside commands via `app_handle.state::<T>`.
- Make the `State<T>` type match the managed type exactly (a mismatch is a runtime
  panic) — alias it (`type AppState =...`).

## 3. Security / capabilities / permissions

- **Treat the frontend as compromised; enforce all authorization in Rust.**
- **Least privilege:** grant the minimum permissions per window via capability
  files in `src-tauri/capabilities/`; scope to named **window labels**, not `"*"`.
- Explicitly list active capabilities; allowlist custom commands where supported.
- **Never expose raw `fs`/`shell`;** restrict with path-glob scopes. Set a strict
  CSP (`default-src 'self'`). _(Folio canonicalizes every path under an allowed
  root — `CODE_STYLE.md` §8.1.)_
- Use platform-scoped capabilities + the `$schema` reference for IDE validation.
- Capabilities don't protect against malicious Rust or WebView 0-days — keep
  back-end soundness yours and run `cargo audit`/`npm audit` in CI.

## Sources

Tauri v2: Calling Rust from the Frontend · State Management · Security ·
Capabilities & Permissions.
