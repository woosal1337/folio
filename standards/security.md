# Security Standards

Attune captures audio. Capture is the most sensitive operation a meeting app
performs. The defence model assumes one bug should never escalate to a leak
or a wipe.

## Threat model

We design against three classes of failure:

1. **Accidental exfiltration.** A bug or rogue dependency in either the
   Rust core or the React frontend tries to phone home with audio or
   transcripts.
2. **Filesystem misuse.** A bug in delete/move/save code damages user
   data outside Attune's scope.
3. **Permission escalation through the webview.** A future HTML feature
   (link, embed, blob) provides a path the webview was not meant to walk.

The aim is reachability, not perfection: the smallest blast radius for
each class of bug.

## Audio data path

- Audio is captured by Rust, written to WAV files in
  `~/Documents/Attune/Recordings/<timestamp>/`, and never streamed
  anywhere by the core. The OpenAI integration (when enabled in
  Settings → Transcription) is the only outbound audio path.
- The OpenAI key never leaves the user's machine *unless* the user
  enables that provider. Default provider for v0 ships as `openai`
  in `state.rs::default_provider()`; when local Whisper lands we may
  flip the default.
- API keys are stored in the settings JSON file alongside other prefs.
  Future revision will move secrets to the macOS Keychain via
  `security-framework`. Tracked in `architecture/state-management.md`.

## Capabilities (Tauri)

- `src-tauri/capabilities/default.json` is the authoritative list of
  what the main window can do. Adding a permission requires:
  1. A command that needs it.
  2. A comment in the capability JSON (or this document) explaining why.
- We grant `fs:allow-home-*` because settings + recordings live in
  `$HOME/Documents/Attune`. We do *not* grant `fs:allow-app-write` etc.
- `dialog`, `opener`, `fs` plugin permissions are scoped to the actions
  the UI exposes. No `*-recursive-allow-all`.

## Asset protocol scope

- The asset protocol scope in `tauri.conf.json` limits which paths the
  webview can load via `convertFileSrc`. Scope: `$HOME/**`, `/Users/**`,
  `/var/folders/**`, `/tmp/**`.
- Audio players in the UI call `convertFileSrc(item.session_dir + "/mic.wav")`.
  The path comes from the backend's list of recordings, never from a
  user-typed string.

## Path safety

- `delete_recording` canonicalises both the target path and the
  recordings root, then refuses to delete anything that is not strictly
  under the root. This blocks both symlink escapes and accidental
  `output_dir = "/"` configurations.
- Other write operations (settings save, future notes save) similarly
  validate paths before touching them.
- Path validation is `starts_with` over canonical forms, not string
  prefix comparison on raw paths. Symlinks must resolve to the
  authorised scope.

## CSP

- `tauri.conf.json` ships with `csp: null` today because the dev
  workflow injects styles inline and `pnpm dev` cannot prove a strict
  policy. **Before public release**, lock this down:

  ```
  default-src 'self';
  connect-src 'self' https://api.openai.com;
  img-src 'self' asset: data:;
  media-src 'self' asset:;
  style-src 'self' 'unsafe-inline';
  font-src 'self' data:;
  ```

  See `tooling.md` for the manual checklist tracked against this work.

## IPC trust boundary

- Tauri commands are the trust boundary. The frontend can call any
  registered command but cannot invent new ones. The Rust side validates
  its inputs.
- Inputs to commands are deserialised by serde. Don't accept untyped
  blobs. Refuse over-large strings (>1 MB) where the type makes that
  meaningful.
- Never `Command::new("sh").arg(user_input)`. The `reveal_in_finder`
  command spawns `open` with a `PathBuf`, not a shell string.

## Dependencies

- New Rust dependencies are vetted before adoption: license is
  MIT/Apache/BSD compatible, repo has at least one tagged release,
  the dependency tree adds less than ~5 unfamiliar transitives.
- `cargo deny` (TODO in `tooling.md`) gates licenses and advisories
  in CI.
- `cargo audit` runs as part of the release workflow and fails on
  unfixed RUSTSEC advisories.
- npm: `bun.lock` and `pnpm-lock.yaml` are committed. The single source
  of truth is `pnpm` (the lockfile that CI uses). `bun.lock` is a
  historical artefact and may be removed in a cleanup pass.

## Secrets

- No secrets in the repo. `.env*` is in `.gitignore`. The OpenAI API
  key is held in user settings and never logged.
- Log redaction: when `tracing` emits an event that includes a config
  struct, ensure the `Debug`/`Display` impl omits the API key. The
  current `Settings` struct has the key as a plain `String`; reviewing
  log output is part of pre-release hardening.

## Network

- Outbound network calls in the Rust core are limited to OpenAI when
  transcription is configured to use it. The frontend should not make
  direct `fetch` calls to any third party.
- `reqwest` is built with `rustls-tls`, not native TLS. Avoids the
  Schannel/OpenSSL dependency on Windows and the build complexity.
- Future model downloads (whisper.cpp weights) use HTTPS with a pinned
  SHA-256 checksum. The model is hashed after download and rejected if
  it doesn't match.

## Update mechanism

- v0 ships without an updater. v1's Sparkle 2 integration verifies
  signatures of update bundles before installing.
- The Homebrew Cask path is the supported channel for non-Sparkle users;
  Cask checksums are part of the release workflow.

## Privacy posture

- README and the public site state plainly: "Audio never leaves your
  machine" — and that must remain true unless the user enables a remote
  transcriber. Code reviews enforce this; no telemetry, no analytics.
- Crash reporting is opt-in, off by default, and excludes audio. Today
  no crash reporter is wired up.

## Hardening checklist (pre-public-release)

- [ ] CSP locked down per the policy above.
- [ ] OpenAI key moved into macOS Keychain.
- [ ] `Debug` impl for `Settings` redacts the API key.
- [ ] `cargo audit` clean.
- [ ] `cargo deny check` clean.
- [ ] All Tauri capabilities scoped as tightly as possible.
- [ ] `cargo clippy --workspace -- -D clippy::pedantic` reviewed (not all
      fixed, just reviewed for security-relevant findings).
- [ ] `npm audit` (or `pnpm audit`) clean of high/critical findings.

## Reporting vulnerabilities

`SECURITY.md` at the repo root is the public-facing contact. Reports go
to the email listed there; do not file public GitHub issues for security
bugs.
