# Attune Code Style

The contract every file in this repository follows. Reviewers will reject PRs that violate the rules in this document. AI agents (Claude Code, Codex, etc.) MUST treat these rules as overriding any default behaviour they might otherwise apply.

This document is the authoritative source. The `AGENTS.md` and `CONTRIBUTING.md` files at the repo root reference it.

---

## 0. North star

Attune is open-source software written for an audience of strangers. Every line should be **legible cold** — readable by someone who has never seen the rest of the codebase — and every public interface should be **stable**: rename it now if you ever will, because we will not break it for downstream users.

When the rules below conflict with one another, the order of precedence is:
1. Correctness
2. Security
3. Public-API stability
4. Performance on the audio / transcription hot paths
5. Legibility
6. Brevity

---

## 1. Comments

### 1.1 Rule: doc-comments above declarations only

Inline `//` (Rust) and `//`, `/* */` (TypeScript) comments **inside function bodies are not permitted**. The compiler enforces what the code does; the doc-comment above the declaration explains *why*.

The only permitted comments inside a body are:

| Tag | Rust | TypeScript | When |
|---|---|---|---|
| `// SAFETY: …` | required | n/a | Above every `unsafe { … }` block. |
| `// TODO(<owner>): …` | allowed | allowed | When deferring work; must name an owner. |
| `// FIXME(<owner>): …` | allowed | allowed | Known bug, must name an owner. |
| `// NOTE: <why>` | allowed | allowed | Load-bearing WHY a reader cannot derive from the code (a hidden constraint, a subtle invariant, a workaround for a specific upstream bug). Must explain *why*, never *what*. |

Everything else gets deleted in the next refactor pass. If a comment says what the code is doing, the code is not clear enough — rename the function, split the expression, or extract a helper.

### 1.2 What good doc-comments look like

**Rust:**

```rust
/// Atomically write `bytes` to `path` via a sibling `.tmp` file
/// followed by a rename. The temp file is fsynced before the rename
/// so a power loss between the two cannot leave the destination
/// truncated.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    /* implementation */
}
```

- One paragraph for the WHY.
- Use `///` for items and `//!` for module-head docs.
- `Errors` / `Panics` / `Examples` sections are encouraged on public APIs.

**TypeScript:**

```ts
/**
 * Compute the centre point of `rect` clamped to the visible
 * viewport. Used by the share sheet so the picker never anchors
 * off-screen on multi-monitor setups.
 */
function clampedCentre(rect: DOMRect): { x: number; y: number } {
    /* implementation */
}
```

- JSDoc above every exported function / type / component.
- Inline param tags only when the parameter name does not explain itself.

### 1.3 Module / file head docs

Each Rust module file begins with a `//!` paragraph that explains its purpose and links to the relevant vault doc (or v2 finding id, e.g. `v2 finding 037 / GET-40`) when one exists. TypeScript files do **not** need a head paragraph — components are documented at the export.

### 1.4 What gets removed in the refactor sweep

- `// increment i`
- `// open the file`
- `// loop through items`
- `// added for issue #123`
- `// changed to fix the bug`
- `// kept for backwards compat` — either remove the code or replace with `// NOTE: <why>`.
- Multi-line ASCII-art separators (`// ---- foo ----`).
- Commented-out code. Delete it; git remembers.

---

## 2. Naming

### 2.1 Rust

| Item | Convention | Example |
|---|---|---|
| Module / crate | `snake_case` | `cloud_guard`, `attune_core` |
| Struct / enum / trait | `UpperCamelCase` | `CloudGuard`, `MemoryStore` |
| Function / method | `snake_case` | `enforce_cap`, `resume_offset` |
| Constant / static | `SCREAMING_SNAKE_CASE` | `DEFAULT_DEBOUNCE` |
| Type parameter | One uppercase letter, then `UpperCamelCase` for clarity | `T`, `R: Reindexer` |
| Lifetime | Single lowercase letter | `'a`, `'static` |

### 2.2 TypeScript / React

| Item | Convention | Example |
|---|---|---|
| Component | `UpperCamelCase` | `VoiceDebriefSheet` |
| Hook | `useUpperCamelCase` | `useTranscriberCopy` |
| File for a component | `kebab-case.tsx` matching the component | `voice-debrief-sheet.tsx` |
| Plain helper | `camelCase` | `formatDuration`, `clampedCentre` |
| Type / interface | `UpperCamelCase` | `RecordingSummary` |
| Constant | `SCREAMING_SNAKE_CASE` only when truly a global; otherwise `camelCase` | `MAX_SECONDS` |

### 2.3 Naming substance

- **Names describe behaviour, not implementation.** `enforce_cap()` is better than `delete_old_files_if_over_size()`.
- **No abbreviations that are not in the public domain.** `evt`, `ctx`, `mgr`, `usr` are forbidden. `cfg`, `db`, `id`, `url`, `http` are accepted as universal.
- **Booleans read as predicates.** `is_airgap`, `has_transcript`, `should_retry`. Never `airgap_flag`.
- **Setters are verbs, getters are nouns.** `set_airgap(true)`, `is_airgap()`.

---

## 3. Errors

### 3.1 Rust

- Every error type derives from `thiserror` and lives next to its module.
- The crate-level `AttuneError` enum in `crates/attune-core/src/error.rs` is the public surface that exits the library boundary. Module-local errors implement `From<…> for AttuneError`.
- No `unwrap()` outside `#[cfg(test)]`. `expect("<reason>")` is permitted for invariants that genuinely cannot fail; the reason text must explain the invariant.
- Errors that cross IPC into Tauri commands convert to `String` at the boundary, not earlier.
- `Result<T>` aliases the crate's `Result<T, AttuneError>`; module-local results may use `Result<T, MyError>` directly.

### 3.2 TypeScript

- Every IPC call wraps the Tauri promise and either returns the value or throws an `IpcError`. Call sites use `try`/`catch` + `toast.error()` + `console.error()`. Never silently swallow.
- Component-local errors live in state as `error: string | null`. Never `error: any`.
- Forbidden: `try { … } catch {}` with an empty catch body.

---

## 4. Logging and telemetry

### 4.1 Rust

- All logging is `tracing` macros. `println!` and `eprintln!` are forbidden outside of `attune-cli` user-facing output.
- Use `tracing::info!` for state changes the user could plausibly want to read; `debug!` for everything else; `warn!` for recoverable failures; `error!` for ones the program cannot recover from.
- Always include the relevant identifier as a structured field: `info!(session_dir = %dir.display(), "started recording")`.

### 4.2 TypeScript

- `console.log` only in development scaffolding; remove before merge.
- `console.error` is acceptable for IPC failures and unexpected exceptions; pair with a user-visible `toast.error()`.
- Never log secrets, API keys, or transcript contents.

---

## 5. Tests

### 5.1 What needs tests

- **Every public function in `attune-core` that has logic.** Pure helpers (`atomic_write`, `locate_span`, `enforce_cap`, `judge`, `decide`) are non-negotiable.
- **Every Tauri command that does work the React side could not.** Validate path traversal guards, atomic writes, IPC contract.
- **Behaviour, not implementation.** Tests that read like the spec survive refactors; tests that assert internal call orders die on the next refactor.

### 5.2 Layout

- Unit tests live under `#[cfg(test)] mod tests` inside the same file as the code.
- Integration tests live in `crates/<crate>/tests/<topic>.rs`.
- React tests live next to the component as `<name>.test.tsx`.

### 5.3 What good tests look like

```rust
#[test]
fn airgap_blocks_external_hosts() {
    set_airgap(true);
    let err = ensure_allowed("api.openai.com").unwrap_err();
    assert!(matches!(err, CloudGuardError::Airgapped { .. }));
    set_airgap(false);
}
```

- One behaviour per test.
- Test name reads as a sentence describing the behaviour.
- No shared mutable state between tests; if global state is involved, reset it at the start AND end of the test.

---

## 6. Concurrency and async

### 6.1 Rust

- Cross-thread state goes behind `Arc<parking_lot::Mutex<…>>` or `Arc<tokio::sync::Mutex<…>>`. Never `std::sync::Mutex` (no poisoning, faster in practice).
- Audio capture and the file-system watcher run on dedicated threads with bounded channels (`crossbeam-channel::bounded`). The bound matters — unbounded queues hide backpressure problems.
- Tauri commands are `async fn` and use `tauri::async_runtime::spawn_blocking` for any blocking IO. Holding a `tauri::State` reference across an `.await` is forbidden; clone what you need first.
- No `block_on` inside an async function. Restructure the API instead.

### 6.2 TypeScript

- Effects MUST clean up subscriptions: every `addEventListener`, `setInterval`, `setTimeout`, `MediaRecorder.start`, and Tauri `listen` returns a teardown — call it from the cleanup function.
- Race conditions across rapid re-renders are guarded with a `cancelled` flag in the effect body.
- No `.then().catch()` chains; use `async`/`await` + `try`/`catch`.

---

## 7. Performance

### 7.1 Hot paths

- The audio capture thread, the WAV writer, and the VAD inner loop must not allocate. Pre-allocate buffers via `Vec::with_capacity` at startup and reuse them.
- The transcription Whisper call must not hold any lock across the whisper.cpp `full()` invocation.
- React render functions must not call IPC; effects do.

### 7.2 Cold paths

- Premature optimisation is a style smell. If you reach for `unsafe`, an inline cache, or a custom allocator, the doc-comment above must justify it with a measured benchmark.

### 7.3 Bundle size

- New TypeScript dependencies require a one-line justification in the PR description. Anything pulling in more than 50 KB gzipped also needs a second-opinion review.
- Lucide icons: only import the icons you use (`import { Mic, MicOff } from "lucide-react"`).
- No moment.js, no lodash. Use the standard `Intl` API and `date-fns` (already in the workspace).

---

## 8. Security

### 8.1 Inputs

- Every Tauri command that accepts a path canonicalises it and asserts it lives under the user's configured root. Existing reference: `read_transcript`, `save_debrief`.
- Every external string that enters a filename gets sanitised: forward-slash, backslash, leading-dot rejected.
- HTML rendered from any LLM response goes through a sanitiser (`Markdown` component in `src/shared/ui/`). Never `dangerouslySetInnerHTML` with untrusted input.

### 8.2 Secrets

- Keys live in the macOS Keychain (`keystore.rs`), never on disk in plain text and never in env files committed to git.
- `.env*` is gitignored; verify before adding any new entry.
- Webhook signing uses HMAC-SHA256 (`webhook_sign.rs`); the signing key never crosses to the frontend.
- Logging secrets is a release-blocking bug; the IpcError formatter must redact known keys before serialising the cause.

### 8.3 Network

- All outbound HTTP goes through `cloud_guard::ensure_allowed(host)` before opening the socket. Privacy Mode is not optional in some call sites — it is a hard cutoff.
- Localhost (`localhost`, `127.0.0.1`, `::1`, `0.0.0.0`) is never blocked.

---

## 9. Architecture rules

### 9.1 Crate boundaries

- `attune-core` is the library. It does not depend on Tauri, on the React app, or on any UI framework.
- `attune-cli` and `attune-app` (the Tauri shell) consume `attune-core`. They do not call each other.
- Public types that will cross the UniFFI boundary live in `crates/attune-core/src/ffi/`.

### 9.2 Frontend boundaries

- `src/features/<feature>/` is self-contained: route, components, hooks specific to the feature.
- `src/shared/` is the cross-feature surface: `ui/` (design system), `lib/` (helpers + IPC), `hooks/`, `stores/`, `types/`. The `types/` folder is auto-generated from Rust via `ts-rs`; NEVER hand-edit a file in there.
- `src/chrome/` is the app shell: sidebar, drag strip, deep-link handler, dialogs that cross features.

### 9.3 Module organisation inside a crate

- One concept per file. `cloud_guard.rs` owns the airgap toggle and nothing else.
- `mod.rs` does nothing but declare submodules and re-export the crate's public surface.
- New public types get added to the crate's documented `pub use` list in `lib.rs` only when they are stable.

---

## 10. Git hygiene

### 10.1 Commits

- Conventional commits with scope: `feat(get-42): privacy mode / cloudguard`.
- Subject lowercase, imperative mood, 70 characters max.
- Body explains the WHY; the diff already shows the WHAT.
- Commits are **GPG-signed**. Pre-commit hooks enforce this — never disable signing with `commit.gpgsign=false` unless you have written approval.
- Commits are **never co-authored by an AI agent.** No `Co-Authored-By:` trailers. The human approving the PR is the author.

### 10.2 Branches

```
feat/<scope>          new feature
fix/<scope>           bug fix
refactor/<scope>      internal restructuring with no behavioural change
docs/<topic>          docs only
chore/<scope>         tooling, build, deps
```

### 10.3 PRs

- One concern per PR. Refactors and feature work go in separate PRs.
- CI must be green before review: `fmt`, `clippy -D warnings`, `cargo test`, `bun run typecheck`, `bun run lint`, `cargo-deny`.
- Auto-generated ts-rs types (`src/shared/types/`) get committed in the same PR as the Rust change that produced them.

---

## 11. Pre-merge checklist

Before opening a PR, run locally:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun run typecheck
bun run lint
```

The pre-commit hooks run a subset of these on every commit; the full set runs in CI. If a pre-commit hook auto-fixes a file, re-stage and re-commit — never `--no-verify` to bypass.

### 11.1 Public-release hygiene checklist

Before merging anything that touches the public README, license, install instructions, or top-level docs, also verify:

- [ ] No `console.log` in production code paths.
- [ ] No `unwrap()` outside `#[cfg(test)]`.
- [ ] No commented-out code blocks.
- [ ] No inline `//` body comments (Section 1).
- [ ] No hardcoded user paths (`/Users/<name>/…`) outside tests.
- [ ] No API keys, JWTs, signing secrets in the diff. Run `git diff main --name-only | xargs grep -l 'sk-\|sk_\|Bearer '` and verify the matches.
- [ ] Every new dependency has a license compatible with MIT.
- [ ] Every new public Rust type has a doc-comment.
- [ ] Every new React component has a JSDoc above the export.

---

## 12. Tools

- **Rust toolchain** pinned in `rust-toolchain.toml` (1.88+).
- **Bun** 1.3+ — the only JS package manager and runtime.
- **rustfmt** and **clippy** configured by `rustfmt.toml` + `clippy.toml`. Do not override per-file.
- **eslint** (`eslint.config.js`) and **prettier** (`.prettierrc.json`). `--max-warnings 0`.
- **pre-commit** (`.pre-commit-config.yaml`) runs fmt, clippy, prettier, eslint, typos, taplo.
- **typos** spell-check runs on every commit; allowed terms live in `_typos.toml`.
- The `SKIP=taplo-lint` workaround is permitted while the upstream hook is broken; this should be removed once upstream ships a fix.

---

## 13. When you do not know

- For architecture questions, read the matching doc in `docs/guidelines/` (audio pipeline, frontend architecture, Rust async, error handling, Tauri architecture).
- For product / design rationale, read the vault at `~/Documents/GitHub/obsidian.md/projects/attune/`.
- For an answer this document doesn't cover, ask the human reviewer. Do not invent.

---

## 14. Document history

- 2026-05-26 — Initial version. Establishes Section 1 (strict no-inline-comments rule), Section 11.1 (public-release hygiene checklist), and the precedence order in Section 0. Referenced by `AGENTS.md` and `CONTRIBUTING.md`.
