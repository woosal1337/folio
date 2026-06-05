# Folio engineering guidelines

Five focused documents, each a source-cited synthesis from ~25 web searches in May 2026 of current professional practice. Re-read these when adding new code in the relevant area; cite specific sections in PR reviews.

| Document                                                 | When to consult                                                                                          |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| [`rust-architecture.md`](./rust-architecture.md)         | Adding a crate, new module, or public API; tightening visibility; planning a refactor.                   |
| [`rust-error-handling.md`](./rust-error-handling.md)     | Adding a new error variant; designing a fallible API; wiring an error across the Tauri IPC boundary.     |
| [`rust-async.md`](./rust-async.md)                       | Spawning tasks; choosing a mutex; designing cancellation; anything touching the audio realtime callback. |
| [`tauri-architecture.md`](./tauri-architecture.md)       | Adding or refactoring a `#[tauri::command]`, event, plugin, or capability file.                          |
| [`audio-pipeline.md`](./audio-pipeline.md)               | Editing the audio callback, ring buffer, resampler, or WAV writer. **§1 is non-negotiable.**             |
| [`frontend-architecture.md`](./frontend-architecture.md) | Creating React components, Zustand slices, or Tauri IPC wrappers.                                        |

## How to use these

- **Cross-link, don't duplicate.** When you reference a guideline in a PR description, link to the specific section heading.
- **Update with reality.** When a guideline is wrong because the codebase outgrew it, update the doc in the same PR that breaks the rule. Don't let docs lie.
- **New patterns get a section.** When you discover a new "Folio-specific refactor candidate" or pitfall, add it to the matching doc under the existing "Folio-specific recommendations" section.
- **Sources are load-bearing.** Every rule cites an authoritative source. If you add a rule, cite a source — official docs, well-known blogs (matklad, Palmieri, burntsushi, Bencina), or working repos (LukeMathWalker/zero-to-production, tauri-apps/plugins-workspace).

## Hierarchy of conventions

When two documents seem to disagree, the most-specific one wins:

1. `AGENTS.md` (project-wide AI rules, repo root) ← applies to every contribution
2. `CONTRIBUTING.md` (human-facing process, repo root) ← applies to every PR
3. `docs/guidelines/*` (this directory) ← applies when working in the relevant area
4. Inline comments at the call site ← applies to the specific line

## What's intentionally NOT in here

- **Codebase-specific architecture** — that lives in [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md).
- **Tool reference** — Tauri, Rust, Vite, etc. have authoritative docs. We cite them, we don't paraphrase them.
- **Stylistic micro-rules already enforced by tooling** — `rustfmt`, `clippy`, `prettier`, `eslint`. If it's a config setting, configure it; don't write prose about it.

## Reviewing PRs against these guidelines

A quick rubric for self-review (and code-review) checklist:

- [ ] Public Rust APIs follow the [API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html) — at minimum naming, common traits, error meaningfulness, doc examples.
- [ ] New `#[tauri::command]` returns `Result<T, CommandError>`, body ≤ 30 lines, lives in the right `commands/` file.
- [ ] No new `unwrap` outside tests. New `expect("...")` messages explain _why_ the panic is impossible.
- [ ] No allocations or locks were added to the audio callback path.
- [ ] New React components are kebab-case files with PascalCase exports; no new barrel `index.ts`.
- [ ] No new Tauri `listen` without a corresponding `unlisten` in the cleanup.
- [ ] If the change is structural, cite the matching guideline section in the PR description.
