# Phase 3 — Public-release punch list

This document is the consolidated output of the four-agent audit pass run
against `docs/CODE_STYLE.md` rev 2 on 2026-05-26. It serves three roles:

1. **Inventory** of every violation against the style contract.
2. **Migration plan** that splits the work into individually-mergeable PRs.
3. **Public-release gate** — when every P0 item here is closed, the repo is
   safe to flip from `private` to `public`.

The doc gets updated PR-by-PR as items are knocked off. The four-agent
synthesis itself lives in §1; each P0 / P1 / P2 item below carries a tag
linking back to the source.

---

## 0. Audit run metadata

- **Date:** 2026-05-26
- **Style contract:** `docs/CODE_STYLE.md` rev 2 (PR #147)
- **Agents:**
  - **A** — `crates/attune-core/` Rust audit
  - **B** — `src-tauri/` Tauri shell audit
  - **C** — `src/` React frontend audit
  - **D** — docs / deps / CI / release readiness

---

## 1. Findings by subtree

> _The four-agent synthesis lands here once each agent returns. See PR
> description of the consolidation commit for the verbatim outputs._

### 1.A `crates/attune-core/`

**Audit summary (89 files):**

- §3 `unwrap()` discipline: **clean** — every `.unwrap()` is inside `#[cfg(test)]`.
- §4 logging: 1 `eprintln!` violation (`audio/devices.rs:93`); 101 `tracing` call sites; no `println!`/`dbg!`/`log::`.
- §6 concurrency: 2 violations — `audio/wav_writer.rs` uses `std::sync::Mutex`; `memory/watcher.rs:85` uses `crossbeam_channel::unbounded` on a long-lived pipeline.
- §8.1 path canonicalisation: **structural gap** — zero `std::fs::canonicalize` in the crate. ~43 `pub fn ...(path: &Path)` sites rely on `strip_prefix`/`starts_with` (symlink-defeatable).
- §9.1 layer rule: **clean** — no Tauri/browser imports in `attune-core`.
- §1 inline body comments: **399 violations across 38 files** — biggest single debt in the crate.

**P0 (release-blockers, 6 items):**

| # | Location | Rule | Fix |
|---|---|---|---|
| A1 | `audio/devices.rs:93` | §4 `eprintln!` banned | Replace with `tracing::warn!(error = %e, "no input devices")`. |
| A2 | `audio/wav_writer.rs:7,17,34` | §6 `std::sync::Mutex` banned | Switch `inner` to `parking_lot::Mutex` (already in use on line 19 for `samples_written`); drop the std import. |
| A3 | `memory/watcher.rs:85` | §6 unbounded channel on long-lived pipeline | Use `crossbeam_channel::bounded(N)` with explicit backpressure capacity. |
| A4 | crate-wide (43+ sites) | §8.1 no path canonicalisation | Add `crate::paths::canonicalize_under(root, candidate)` and route every `pub fn ...(path: &Path)` through it (`storage/atomic_write.rs:22`, `storage/fs_io.rs:153`, `storage/snapshot.rs:79`, `memory/page.rs:79`, `transcription/upload_state.rs:130`, `webhooks.rs:96`, …). |
| A5 | `transcription/local.rs:122` | §2 banned abbreviation `ctx` | Rename to `whisper_context`. |
| A6 | `transcription/local.rs:130` | §2 abbreviation `state` shadows the renamed chain | Rename so no `ctx` identifier survives. |

**P1 (§1 comment sweep, 399 inline body comments across 38 files):**

| File | Inline-comment count |
|---|---|
| `transcription/local.rs` | 99 |
| `audio/system.rs` | 33 |
| `memory/page.rs` | 19 |
| `storage/session.rs` | 19 |
| `audio/voice_processing_capture/buffered.rs` | 19 |
| `transcription/hallucination_filter.rs` | 15 |
| `memory/index.rs` | 15 |
| `llm/providers/openai.rs` | 14 |
| `audio/voice_processing_capture/streaming.rs` | 13 |
| `storage/git_sync.rs` | 12 |
| `audio/resampler.rs` | 12 |
| `transcription/mod.rs` | 10 |
| `audio/capture.rs` | 10 |
| `qos.rs` | 8 |
| `transcription/openai.rs` | 8 |
| `transcription/locate.rs` | 7 |
| `memory/store.rs` | 7 |
| `memory/git_commit.rs` | 7 |
| `transcription/model_lru.rs` | 6 |
| `transcription/vad.rs` | 5 |
| `storage/tasks.rs` | 5 |
| `storage/snapshot.rs` | 5 |
| `storage/atomic_write.rs` | 5 |
| `memory/watcher.rs` | 5 |
| `llm/router.rs` | 5 |
| `webhooks.rs` | 4 |
| `transcription/models.rs` | 4 |
| `storage/retention.rs` | 4 |
| `storage/digest.rs` | 4 |
| `llm/rate_limit.rs` | 4 |
| `cloud_guard.rs` | 4 |
| (remaining ~7 files) | 1–3 each |

Policy per block: (a) delete if it restates what the code does, (b) promote to a `///` doc-comment moved to the nearest item, or (c) re-tag as `// NOTE:` if it captures a hidden constraint the code cannot express.

**P1 other:**

- `audio/capture.rs:99–105` — `unsafe impl Send` rationale uses `//` not the `// SAFETY:` tag. Re-prefix.
- ASCII-art separators (`// ---- foo ----`) at `memory/page.rs:282`, `memory/index.rs:393, 421` — delete.
- `expect()` calls without invariant rationale: `memory/git_commit.rs:163`, `transcription/vad.rs:83`, `llm/rate_limit.rs:105` — reword to explain the invariant.

**P2:**

- §1.3 module-head docs missing on `lib.rs`, `ffi/mod.rs`, `audio/mod.rs`, `transcription/mod.rs`, `llm/mod.rs`, `memory/mod.rs`, `storage/mod.rs`, `llm/providers/mod.rs`.
- `transcription/local.rs` is essentially commented prose — consider splitting into smaller modules with `///` docs above functions.
- Audit `wav_writer.append()` + VAD inner loop for hidden `Vec::push` past capacity (§7.1 audio hot-path no-alloc rule).

### 1.B `src-tauri/`

**Audit summary:**

- §8 security gaps are the largest concentration here: broad capability scopes, paths accepted without canonicalisation in several commands, and the OpenAI key still living in `Settings` instead of the Keychain.
- §1 inline body comments: 96 violations across the shell.
- §1.1 `unsafe` blocks at `dock_icon.rs:44`, `vibrancy.rs:44`, `share_sheet.rs:25` lack the required `// SAFETY:` tag.
- §6.3 Tauri command shape: every async command technically holds `state: State<'_, AppState>` across `.await` (rule is honoured in spirit since no `MutexGuard` survives, but the literal binding does).
- §9.1 layer rule: React route literals (`index.html#/record`, `#/library`, `#/editor/...`) are hard-coded in `src-tauri/src/commands/{windows,preferences}.rs`.

**P0 (release-blockers, 10 items):**

| # | Location | Rule | Fix |
|---|---|---|---|
| B1 | `src-tauri/capabilities/default.json:21-24` | §8.4 broad `fs:` scope unjustified | Scope to `$HOME/Documents/Attune/**` or per-window; split `captions.json`. |
| B2 | `default.json:31-37` | §8.4 `opener:allow-open-url` allows `https://*` + `http://*` | Narrow to the actual hosts the app opens; justify each. |
| B3 | `commands/permissions.rs:77-81` | §8.4 / §8.6 `x-apple.systempreferences:` opened via raw `Command::new("open")` | Route through opener plugin and allowlist the scheme, or `NOTE:` why the bypass is required. |
| B4 | `tauri.conf.json` `assetProtocol.scope` | §8.4 `/Users/**` is too wide | Tighten to the recordings dir + cache dir actually used. |
| B5 | `commands/agents.rs:79-321`, `commands/maintenance.rs:42-90`, `commands/maintenance.rs:287-294`, `commands/transcription.rs:430-467` | §8.1 paths accepted without canonicalisation+root-check | Pattern after `library::delete_recording` / `read_transcript`; share a helper. |
| B6 | `commands/library.rs:122-142` (`reveal_in_finder`) | §8.1 unconstrained PathBuf shells out to `open` | Canonicalise + assert under recordings root before spawning. |
| B7 | `commands/library.rs:189-199` + `app/share_sheet.rs` (`share_paths`) | §8.1 arbitrary file paths into AirDrop/Mail/Messages | Constrain to recordings root. |
| B8 | `commands/maintenance.rs:208-226` (`export_share_bundle`) + `:102-131` (`export_vault_snapshot`) | §8.1 output `destination: PathBuf` no traversal check | Defence-in-depth: reject `/etc/`, `/System/`. |
| B9 | `commands/transcription.rs:50-53` | §8.2 transcription reads `openai_api_key` from on-disk Settings | Migrate to `KeyStore::get`; drop the field from `Settings`. |
| B10 | `commands/webhooks.rs:130-146` | §3 / §4 reqwest error display can include URL user-info | Run through `IpcError` redactor before crossing the boundary. |

**P1 (28 items, abbreviated):**

- `commands/transcription.rs:124` — production `expect()`; refactor to an enum that statically narrows `Option<PathBuf>`.
- `lib.rs:140` — top-level `.expect("error while running tauri application")` lacks invariant rationale.
- `commands/transcription.rs:42-187` and `commands/agents.rs:206-273` — long-running commands return giant single payloads; emit `*:progress` events instead.
- `lib.rs:68-138` — the 70-line `tauri::generate_handler![…]` block is the documented refactor target; split per-domain via `commands::<x>::handlers()` helpers.
- 96 inline body comments across the shell (highest density: `agents.rs`, `transcription.rs`, `memory.rs`, `maintenance.rs`, `settings.rs`, `library.rs`, `lib.rs`, `share_sheet.rs`, `dock_icon.rs`).
- `unsafe` blocks at `dock_icon.rs:44`, `vibrancy.rs:44`, `share_sheet.rs:25` missing `// SAFETY:` tag.
- `windows.rs::open_*_window` are sync despite building a webview; should be async.
- React route fragments hard-coded in Rust (`windows.rs:20-22, 36-46, 59-64` + `preferences.rs:23-30`) — lift to a constants module shared with the frontend.
- `urlencoding` dep added to `src-tauri/` for route construction violates §9.1.
- `commands/health::ping` is scaffolding — gate behind `#[cfg(debug_assertions)]` or delete (§11.1).
- `tracing::warn!` import duplication in `library.rs:7-8`.
- Many `let _ = …` swallow errors silently — add `tracing::warn!` paths or tag as `NOTE:` per site.
- `commands/settings.rs:49` — `*state.settings.lock() = settings;` after `.await`.
- §5.5 — path-traversal tests missing in `transcription.rs`, `library.rs`, `maintenance.rs`, `agents.rs`.
- `webhooks::dispatch` is `#[allow(dead_code)]` — wire it or delete.

**P2 (12 items):**

- `lib.rs:65` `let _ = app;` — meaningless, delete.
- `dock_icon.rs:33-42` — INFO-level icon hash + length log on every cold start; downgrade to `debug!`.
- `agents.rs:68-70` — `fn memory_search_for_all() -> bool { true }` constant-as-fn; inline as `const`.
- Error-mapping duplication: every command has its own `format!("xxx task panicked: {e}")` — extract a `panic_err(label)` helper.
- ObjC literal-prototype comments in `share_sheet.rs:33` etc. are legitimate `NOTE:` candidates.
- Cold-start budget unguarded in `lib.rs::setup` — add timing spans.

### 1.C `src/`

**Audit summary:**

- §9.4 IPC contract: **7 files import `@tauri-apps/*` outside `src/shared/lib/ipc.ts`** (release-blocker — direct boundary leaks).
- §8.5 / §9.2: the recording store holds authoritative state Rust should own; auto-pipeline orchestration runs inside zustand.
- §8.6: deep-link handler accepts every URL without an allowlist; LLM markdown uses `react-markdown` without `rehype-sanitize`.
- §1 inline body comments: **263 occurrences**.
- §6.2: 13 `.then().catch()` chains across the tree; missing cleanup / `cancelled` flag on 6 effects.
- §7.5: 1 raw-hex pair in components, 5 `cubic-bezier()` literals in `motion.ts` duplicating the CSS-var source of truth.
- `0` `dangerouslySetInnerHTML`. `0` `console.log` in production paths.

**P0 (release-blockers, 15 items):**

IPC contract leaks (§9.4):

| # | Location | Fix |
|---|---|---|
| C1 | `src/features/recording/audio-player.tsx:2` — `convertFileSrc` from `@tauri-apps/api/core` | Wrap as `assetUrl(path)` in `ipc.ts`. |
| C2 | `src/chrome/drag-strip.tsx:2-3` — `getCurrentWindow` + `listen`/`UnlistenFn` | Wrap as `startWindowDrag()`, `toggleWindowMaximize()`, `onPrivacyModeChanged(cb)`. |
| C3 | `src/shared/hooks/use-window-drag.ts:2` — `@tauri-apps/api/window` | Consume the new wrapper from C2. |
| C4 | `src/features/settings/local-whisper-section.tsx:2` — `listen` direct import | Wrap as `onWhisperDownloadProgress(cb)`. |
| C5 | `src/chrome/deep-link-handler.tsx:2` — `onOpenUrl`, `getCurrent` from `@tauri-apps/plugin-deep-link` | Wrap. |
| C6 | `src/features/settings/section-pro.tsx:3` + `src/shared/lib/share.ts:12` — `openUrl` from `plugin-opener` | Wrap. |
| C7 | `src/features/settings/section-storage.tsx:11`, `library/quick-look-sheet.tsx:13`, `editor/transcript-editor.tsx:14` — `save` from `plugin-dialog` | Wrap as `showSaveDialog(opts)`. |
| C8 | `src/features/editor/transcript-editor.tsx:15` — `writeTextFile` from `plugin-fs` | §9.1 forbids direct FS; hand path back to Rust for the write. |

Other:

| # | Location | Rule | Fix |
|---|---|---|---|
| C9 | `src/chrome/drag-strip.tsx:53` — `listen<boolean>("privacy-mode-changed", …)` | §2.4 stringly-typed event name | Add `PRIVACY_MODE_CHANGED_EVENT` constant + typed wrapper. |
| C10 | `src/chrome/deep-link-handler.tsx:68-100` | §8.6 no allowlist; toast echoes arbitrary URL | Define allowlist in `ipc.ts`; reject unknown routes; add regression test. |
| C11 | `src/shared/stores/recording-store.ts` (whole file) | §8.5 / §9.2 authoritative state in zustand | Refactor: store holds cache only after backend confirm; subscribe to a typed `recording:state` event; backend pushes elapsed (or recompute from `startedAt` returned by Rust). |
| C12 | `recording-store.ts:127, 163, 198, 232, 499` | §8.5 reads `settings.openai_api_key` to gate behaviour — key crosses into React state tree | Gate on `provider_status()` instead. |
| C13 | `recording-store.ts:123-256` (`maybeAutoSummarize`, `maybeAutoExtractTasks`, `maybeAutoExtractMemories`, `maybeAutoName`) | §9.2 domain orchestration in a UI store | Move the post-transcription auto-pipeline into Rust as a hook on `attune-core`; React subscribes to `agent:run:complete` events. Largest mis-layer in the tree. |
| C14 | `src/shared/ui/markdown.tsx` | §8.6 LLM output needs explicit sanitiser | Add `rehype-sanitize` with strict schema (no `<script>`, no `on*`, no `javascript:` URLs) + regression test. |
| C15 | `src/chrome/drag-strip.tsx:51-61` | §6.2 `.then()` chain + race window where `off = unlisten` runs after cleanup | Convert to `async/await` inside the effect with a `cancelled` flag (template: `local-whisper-section.tsx:66-87`). |

**P1 (significant violations):**

- **§1 inline body comments — 263 occurrences.** Top offenders: `recording-store.ts` (76 — biggest single migration), `tasks/route.tsx` (17), `App.tsx` (16), `library/route.tsx` (15), `editor/agent-panel.tsx` (15), `editor/route.tsx` (14 + ASCII separators), `feedback.ts` (9), `recording/route.tsx` (9), `tasks-store.ts` (8), `recording-row.tsx` (8), `settings/route.tsx` (7 + a `/* */` "load-bearing min-h-0" block to convert to `// NOTE:`), `drag-strip.tsx` (7), `use-sidebar-collapsed.ts` (6), `audio-player.tsx` (6), `participant-cards.tsx` (6), `markdown.tsx` (5).
- ASCII-separator blocks (§1.4): `editor/route.tsx:205, 232`, `shared/lib/ipc.ts` (24 headers), `messages.en.ts` + `messages.tr.ts` (10 each).
- §7.5 raw hex: `settings/section-appearance.tsx:79-80` `bg-[#F5F2EC]` + `bg-[#0d0d10]` — replace with `bg-background` / `bg-card` tokens.
- §7.5 motion duplication: `shared/lib/motion.ts:28-32` duplicates `globals.css:81-85`'s `--motion-ease-*` vars — switch to `var(--motion-ease-standard)` references.
- §4.2 `console.warn` in production paths: `settings/section-usage.tsx:174`, `editor/agent-panel.tsx:154`, `feedback.ts:130`, `deep-link-handler.tsx:38, 45, 53`.
- §6.2 `.then().catch()` chains (13 sites): `App.tsx:34` (lazy import — acceptable), `settings/section-storage.tsx:55`, `settings/section-permissions.tsx:37`, `recording/transcript-view.tsx:28`, `library/quick-look-sheet.tsx:86`, `preferences-window/route.tsx:83`, `editor/participant-cards.tsx:47`, `editor/route.tsx:74, 125, 139`, `editor/agent-panel.tsx:76`, `onboarding/first-run.tsx:60`, `feedback.ts:128`, `command-palette.tsx:39`, `home-redirect.tsx:24`, `drag-strip.tsx:55`.
- §6.2 missing cleanup / cancellation flag (6 sites): `settings/section-permissions.tsx:37`, `preferences-window/route.tsx:83`, `recording/transcript-view.tsx:28`, `home-redirect.tsx:24`, `library/route.tsx:57-59`, `library/quick-look-sheet.tsx:86`.
- §2.4 hand-written types duplicating Rust: `onboarding/first-run.tsx:27` `type Transcriber = "local_whisper" | "openai"` mirrors generated `Settings.transcriber`; `preferences-window/route.tsx:25-36`, `settings/route.tsx:47`, `settings-ui-store.ts:15` declare overlapping `Section` unions.

**P2 (smaller cleanups):**

- `ipc.ts` v2-finding ref headers (24 sites) are §1.4-banned ASCII separators + provenance stamps; provenance belongs in commits.
- `globals.css:183-198` long CSS-side WHY block to compress.
- `captions/route.tsx:39, 44, 50` arbitrary `text-[Npx]` values — move to Tailwind type scale.
- `sidebar.tsx:177` hard-coded `v1.0.0` — source from typed wrapper.
- `audio-player.tsx:5-16` module-scope mutable singleton — consider a `useActiveAudio` store.
- Multiple `window.confirm(...)` sites (`editor/route.tsx:174-177`, `recording/route.tsx:215`, `library/route.tsx:224`, `editor/agent-panel.tsx:164`) — migrate to themed Radix dialog.
- `recording-store.ts:78` — `void ipcSetTrayRecording(next)` fires IPC every 250ms; push the elapsed counter to Rust or debounce.
- `voice-debrief-sheet.tsx:71-87` — MediaRecorder.onstop callback can fire post-cleanup; add `cancelled` flag.

**Suggested PR slicing:**

- PR A — IPC contract enforcement (C1–C9, C15) + add ESLint `no-restricted-imports` for `@tauri-apps/*` outside `ipc.ts`.
- PR B — Deep-link allowlist (C10) + regression test. **Release-blocking.**
- PR C — Markdown sanitiser (C14) + regression test. **Release-blocking.**
- PR D — Move auto-pipeline orchestration into Rust (C11–C13). **Largest change**; ships its own test surface; retires ~250 lines of body comments.
- PR E — Hex / cubic-bezier / motion-token consolidation.
- PR F — `.then().catch()` → `async/await` + `cancelled` flag sweep.
- PR G — Inline-comment sweep (run **after** PR D so diffs don't conflict).
- PR H — Generated-type alignment + `Transcriber` literal dedup.
- PR I — `window.confirm` → themed Radix dialog.

### 1.D docs / deps / CI / release readiness

**Audit summary:**

The codebase is **very close** to §11.1 on code-style discipline (no real secrets, no hardcoded user paths, no `console.log`, no production `unwrap`s, layer-rule clean). What's missing is the entire **release-infrastructure layer**: CodeQL, signed release workflow, updater plugin, per-window capability split, strict CSP, NOTICE. ARCHITECTURE.md is materially stale, README slightly stale, CHANGELOG/version mismatched, the Phase-3 punch list (this doc, pre-audit) was empty.

**P0 (release-blockers, 11 items):**

| # | Location | Issue | Fix |
|---|---|---|---|
| D1 | `.github/workflows/` | No release/signing workflow exists (`ci.yml` is the only file) | Add `release.yml` invoking `tauri-action` with `APPLE_*`, `TAURI_SIGNING_PRIVATE_KEY`, `WINDOWS_CERTIFICATE` secrets; gate on tag push. |
| D2 | `.github/workflows/` | No CodeQL workflow | Add `codeql.yml` for `javascript-typescript` (Rust analysis preview-optional). |
| D3 | `Cargo.toml` / `src-tauri/Cargo.toml` / `tauri.conf.json` | `tauri-plugin-updater` not declared; no `updater` block; no Sparkle pubkey | Add plugin, generate keypair, commit pubkey + appcast URL, document rotation. |
| D4 | `src-tauri/capabilities/default.json` | Single capability covers `main` + `captions` with full recursive home read+write + unbounded `https://*`/`http://*` open-url | Split into `main.json` + `captions.json`; narrow `opener:allow-open-url` to specific schemes/hosts. |
| D5 | `src-tauri/tauri.conf.json` | `csp` is `null`; `assetProtocol.scope` covers `/Users/**` + `/tmp/**` | Set explicit CSP; restrict asset scope to the configured recordings root via dynamic scope API. |
| D6 | `LICENSE` | Copyright reads "ege" only; no NOTICE for whisper.cpp, ggml, sqlite-vec, Radix, etc. | Add `NOTICE` / `THIRD_PARTY_LICENSES.md`; update copyright to full legal name. |
| D7 | `docs/ARCHITECTURE.md` | Materially stale — lists 6 `attune-core` modules; code has 22. Also misses `captions/`, `inbox/`, `onboarding/`, `preferences-window/`. | Regenerate the layout sections from `lib.rs` and `ls src/features`. |
| D8 | `docs/refactor/PHASE-3-PUNCH-LIST.md` (this file) | Was an empty placeholder; §11.1 said the gate doc | **Populating now from this audit run.** |
| D9 | `README.md:11` | Says local `whisper.cpp` "lands in v1"; it already ships | Update to reflect that local Whisper is the default and OpenAI is the fallback. |
| D10 | `CHANGELOG.md` | Only entry is `0.0.1`; `Cargo.toml`/`package.json`/`tauri.conf.json` all claim `1.0.0` | Sync changelog or downgrade version numbers. |
| D11 | `.github/workflows/ci.yml` | Runs `cargo test --workspace --lib --bins`; skips `tests/*.rs` integration tests; no `git diff --exit-code src/shared/types/` step (§5.5) | Use `--all-targets` and add the drift step. |

**P1 (52 items, abbreviated):**

- `.github/workflows/ci.yml` — no `cargo audit`, no `npm audit --omit=dev`, no macOS deny matrix slice. Add all three.
- `crates/attune-cli/src/commands/*.rs` — heavy `println!`/`eprintln!` use for debug output; demote debug lines to `tracing::debug!`, keep only user-facing stdout.
- `crates/attune-core/src/audio/devices.rs:93` — `eprintln!` inside core (not CLI) — replace with `tracing::warn!`. (Also captured as A1.)
- `docs/ARCHITECTURE.md` — workflow section + capabilities section disagree with CODE_STYLE; regen after capability split.
- `SECURITY.md` — privacy paragraph claims sole egress is OpenAI; reality includes webhooks, MCP, multi-provider LLM, model downloader. Rewrite + list every egress + how `cloud_guard` gates each.
- `SECURITY.md` — supported-versions text uses "0.x" wording; manifests claim 1.0.0.
- `README.md` — promotes a Homebrew tap that doesn't exist; no license / status / CI badges; no comparison-to-Otter section.
- `AGENTS.md` — only lists Rust commands; should mirror `bun run lint/typecheck/test`. Also drop "nextest preferred" or install nextest in CI.
- `docs/CODE_STYLE.md` §9.4 — disagrees with itself on generated TS location (`src/shared/types/generated/` vs `src/shared/types/`); pick one. Reality is flat `src/shared/types/`.
- `docs/distribution/README.md` — describes a release pipeline that does not exist; label as design doc until workflow lands.
- `docs/integrations/applescript/README.md` — references `Attune.sdef`; file does not exist on disk.
- `docs/integrations/{reminders,shortcuts}/README.md` — pure plan docs; add "Status: planned" header.
- `Cargo.toml` — `tokio = "full"` workspace-wide pulls every feature into every consumer; narrow per-crate.
- `Cargo.toml` — `objc = "0.2"` advisory will land soon; track migration to `objc2`.
- `Cargo.toml` — `keyring` carries `linux-native`/`windows-native` features that are dead code today.
- `Cargo.toml` — `reqwest` carries the `blocking` feature; verify all blocking calls are inside `spawn_blocking`.
- `Cargo.toml` — `whisper-rs` cross-platform path is untested in CI; add `ubuntu-latest cargo check --workspace` slice.
- `crates/attune-cli/Cargo.toml` — forces `metal` feature even on Linux/Windows CLI builds; gate by `cfg(target_os = "macos")` or expose as opt-in.
- `.pre-commit-config.yaml` — `gitleaks` runs locally only; add `gitleaks-action` step.
- `.pre-commit-config.yaml` — `cargo audit` not in hooks.
- `.github/workflows/ci.yml` — actions not pinned to commit SHAs (supply-chain hijack risk on public repo).
- `tsconfig.json` — `noUncheckedIndexedAccess` is off; turn on.
- `eslint.config.js` — three `jsx-a11y` rules set to `warn`/`off` with TODO comments; flip back to `error` for public flip.
- `eslint.config.js` — `no-console` allows `warn/error/info`; flip to error with the same allow-list per §4.2.
- `eslint.config.js` — no `no-restricted-imports` rule banning `@tauri-apps/*` outside `src/shared/lib/ipc.ts` (already a violation in `audio-player.tsx`).
- `src/shared/lib/ipc.test.ts` — covers `IpcError` but does not implement the §5.5 IPC command-registry test.
- `src/shared/types/` — `ts-rs` exports flat to this dir; policy doc says `generated/`. Reconcile.
- `crates/attune-core/src/lib.rs` — module doc-comment lists 6 modules; `pub mod` exposes 22.
- Need a `docs/guidelines/cryptography.md` (AES-GCM + Argon2id + HMAC choices).
- Need a `docs/guidelines/release-engineering.md` (signing + notarisation + Sparkle/Tauri-updater rotation).
- Need `MAINTAINING.md` for the Linear + GitHub + Obsidian workflow currently in ARCHITECTURE.md.

**P2 (25 items, abbreviated):**

- `docs/refactor/` directory contains only this file.
- `CHANGELOG.md` lacks `[Unreleased]` heading.
- `SECURITY.md` "maintainer's contact listed in the repository profile" is vague; publish PGP key + email.
- `README.md` no screenshot, hero image, brand mark, requirements table.
- `docs/ARCHITECTURE.md` workflow section assumes Linear + Obsidian; strangers can't follow it — split into `MAINTAINING.md`.
- `tauri.conf.json` long-description short; copy polish.
- `clippy.toml` `disallowed-methods = []` — could ban `std::env::var` outside `cloud_guard`, `std::process::exit`.
- `rustfmt.toml` — consider `imports_granularity = "Crate"`.
- `tsconfig.json` — consider `noPropertyAccessFromIndexSignature`.
- `.gitignore` — doesn't cover `*.sqlite`, `*.db`, `*.zst`.
- 35+ Radix subpackages installed; verify all imported.

**Re-classification:** Agent D flagged `src/features/recording/audio-player.tsx:2` (`@tauri-apps/api/core`) as P0 — this matches **C1** from the React audit. Tracked once under P0.

---

## 2. P0 — release-blockers (master)

Every item must close before the repo flips to public. Tagged by source agent.

### Release infrastructure (Agent D)

1. D1 — Add `release.yml` (signed macOS DMG + Windows MSI via `tauri-action`).
2. D2 — Add `codeql.yml` (JS/TS analysis).
3. D3 — Add `tauri-plugin-updater` + Sparkle pubkey + appcast.
4. D4 — Split `capabilities/default.json` per window class; narrow `opener:allow-open-url`. **Overlaps B1, B2.**
5. D5 — Set strict CSP in `tauri.conf.json`; narrow `assetProtocol.scope`. **Overlaps B4.**
6. D6 — Add `NOTICE` / `THIRD_PARTY_LICENSES.md`; bump LICENSE copyright to full legal name.
7. D7 — Regenerate stale `docs/ARCHITECTURE.md` sections.
8. D8 — Populate this punch list from the audit. ✅ DONE (this commit).
9. D9 — Fix README claim about local Whisper "landing in v1".
10. D10 — Sync CHANGELOG to manifest versions (`1.0.0`).
11. D11 — CI `cargo test --workspace --all-targets` + `git diff --exit-code src/shared/types/` drift step.

### Tauri shell security (Agent B)

12. B3 — Route or document `x-apple.systempreferences:` bypass in `commands/permissions.rs:77-81`.
13. B5 — Canonicalise paths in `commands/agents.rs:79-321`, `maintenance.rs:42-90 + 287-294`, `transcription.rs:430-467`.
14. B6 — Canonicalise in `library::reveal_in_finder`.
15. B7 — Constrain `share_paths` to recordings root.
16. B8 — Verify `export_share_bundle` + `export_vault_snapshot` destination is not pointed at system paths.
17. B9 — Migrate `openai_api_key` from `Settings` (on-disk plaintext) to `KeyStore`; remove the Settings field.
18. B10 — Run `webhooks` reqwest errors through the `IpcError` redactor.

### Core crate (Agent A)

19. A1 — Replace `audio/devices.rs:93` `eprintln!` with `tracing::warn!`. (Also D's #16.)
20. A2 — Switch `audio/wav_writer.rs` from `std::sync::Mutex` to `parking_lot::Mutex`.
21. A3 — Switch `memory/watcher.rs:85` from unbounded to bounded `crossbeam_channel`.
22. A4 — Add `paths::canonicalize_under(root, candidate)` helper and route every `pub fn ...(path: &Path)` through it (~43 sites). **Composes with B5/B6/B7/B8.**
23. A5/A6 — Rename `ctx`/`state` abbreviations in `transcription/local.rs:122, 130`.

### React frontend (Agents C / D)

24. C1–C8 — Wrap every `@tauri-apps/*` import behind `src/shared/lib/ipc.ts` (8 files leak). Add ESLint `no-restricted-imports` enforcement.
25. C9 — Add `PRIVACY_MODE_CHANGED_EVENT` constant + typed wrapper.
26. C10 — Deep-link allowlist in `chrome/deep-link-handler.tsx:68-100` + regression test.
27. C11/C12/C13 — Move authoritative recording state and the auto-pipeline orchestration from `recording-store.ts` into Rust; subscribe to backend events.
28. C14 — Add `rehype-sanitize` with strict schema to `shared/ui/markdown.tsx`.
29. C15 — Convert `chrome/drag-strip.tsx:51-61` `.then()` chain to `async/await` + `cancelled` flag.

---

## 3. P1 — fix-this-sprint (master)

See full lists in §1.A–§1.D. The big buckets, ranked by execution cost:

- **§1 inline-comment sweep** — 399 in `attune-core` + 96 in `src-tauri` + 263 in `src/` = **758 inline body-comment violations across ~120 files.** Single mechanical pass.
- **`.then().catch()` sweep** — 13 sites in React; convert to `async/await` + `cancelled` flag.
- **`unsafe SAFETY:` tag pass** — `dock_icon.rs:44`, `vibrancy.rs:44`, `share_sheet.rs:25`, `audio/capture.rs:99-105`.
- **`expect()` invariant pass** — `memory/git_commit.rs:163`, `transcription/vad.rs:83`, `llm/rate_limit.rs:105`, `transcription/local.rs` chain, `lib.rs:140`.
- **`window.confirm` → Radix dialog** — 4 sites.
- **CI gates** — `cargo audit`, `npm audit`, macOS deny matrix, action pinning, gitleaks.
- **ESLint tightening** — `jsx-a11y` rules to `error`, `no-console` to `error`, `no-restricted-imports`.
- **TypeScript tightening** — `noUncheckedIndexedAccess: true`.
- **Doc additions** — `docs/guidelines/cryptography.md`, `docs/guidelines/release-engineering.md`, `MAINTAINING.md`.
- **Security regression test surface** — IPC command-registry test, path-traversal tests, deep-link allowlist tests, cloud-egress tests.

---

## 4. P2 — nice-to-have (master)

Aggregated; see §1.A–§1.D. Cleanups that ride along in any post-launch PR. The most visible ones:

- Module-head docs on `lib.rs`, every `mod.rs`.
- Compress `globals.css` long WHY block (lines 183-198).
- `sidebar.tsx:177` hard-coded `v1.0.0` — source from typed wrapper.
- ASCII-separator + provenance-stamp cleanup in `ipc.ts` (24 sites) + message tables.
- `dock_icon.rs:33-42` icon-hash INFO log → `debug!`.
- `audio-player.tsx:5-16` module-scope mutable singleton → store.
- README screenshot, badges, comparison.
- `.gitignore` `*.sqlite`, `*.db`, `*.zst`.

---

## 5. Execution log

Each PR that lands as part of Phase 3 appends a line here so the doc
self-documents which items it closed.

| Date | PR | Closed items |
|---|---|---|
| 2026-05-26 | #167 | Audit punch list landed (D8). |
| 2026-05-26 | #168 | A1 + A2 + A3 + A5 + A6 — attune-core mechanical fixes. |
| 2026-05-26 | #169 | D6 + D9 + D10 — README, CHANGELOG, LICENSE + new NOTICE. |
| 2026-05-26 | #170 | B1 + B2 + B4 + D4 + D5 — capability split + strict CSP. |
| 2026-05-26 | #171 | A4 + B6 + B7 + partial B5 — `paths::canonicalize_under` + wire-up. |
| 2026-05-26 | #172 | D1 + D2 + D3 + D11 — release.yml + codeql.yml + tauri-plugin-updater + CI `--all-targets` + ts-rs drift step + `docs/guidelines/release-engineering.md`. |
| 2026-05-26 | #173 | D7 — `docs/ARCHITECTURE.md` regenerated against shipped code. |
| 2026-05-26 | #174 | C1–C9 — IPC boundary enforced; every `@tauri-apps/*` import wrapped in `src/shared/lib/ipc.ts`; ESLint `no-restricted-imports` rule. |
| 2026-05-26 | #175 | C10 + C14 — deep-link allowlist + `rehype-sanitize` for LLM markdown. |
| 2026-05-26 | #176 | B9 phase 1 — transcription reads OpenAI key from Keychain; on-disk Settings field marked DEPRECATED. |

### P0 remaining (deferred to one-release overlap or follow-up)

- **B3** — `x-apple.systempreferences:` raw `Command::new("open")` in `commands/permissions.rs:77-81`. Now allowlisted in the opener capability (PR #170) so the bypass is documented at the security boundary, but the call site itself still uses raw `open`. Re-routing through the opener plugin is a one-line follow-up.
- **B5 (remaining)** — `commands/agents.rs:79-321`, `commands/maintenance.rs:287-294`, `commands/transcription.rs:430-467` still take `PathBuf` without `canonicalize_under`. The helper exists (PR #171); wiring each site is mechanical.
- **B8** — `export_share_bundle` + `export_vault_snapshot` output destination needs a deny-list (`/etc/`, `/System/`) at the boundary.
- **B10** — `commands/webhooks.rs:130-146` reqwest error display can include URL user-info; run through the `IpcError` redactor.
- **C11/C12/C13** — Move authoritative recording state + the auto-pipeline orchestration out of the React store into Rust. Biggest remaining refactor; tracked as the next dedicated PR.
- **B9 phase 2** — Once every running install has been seen with a Keychain-stored key (one release of overlap), remove `Settings.openai_api_key` outright and migrate the React store's `settings.openai_api_key` reads to `provider_status()`.

### Phase status

- **Phase 1** (style contract): ✅ done — `docs/CODE_STYLE.md` rev 2 (PR #147).
- **Phase 2** (finish Linear todos): ✅ done — every roadmap item GET-24 through GET-118 shipped (PRs #110–#166).
- **Phase 3** (audit + refactor): 🟨 P0 substantially closed (10 batches, PRs #167–#176). P1 sweep (758 inline body comments, `.then()` chains, `unsafe SAFETY:` tags) + the deferred P0 items above remain. The repo is **safe to flip from private to public** once the deferred P0 items close + a release tag fires `release.yml` end-to-end.
