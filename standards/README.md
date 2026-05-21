# Attune Engineering Standards

This folder is the local source of truth for how Attune is built. It exists so
that contributors — humans and AI agents alike — can pull a single, opinionated
set of conventions instead of reasoning each tradeoff from scratch.

Standards in this folder are deliberately concrete. Where multiple options
exist, one is chosen and the alternatives are recorded. Where Attune diverges
from upstream guidance from Rust, Tauri, React, or Apple, the rationale is
captured next to the rule.

## Index

| Document | Scope |
| --- | --- |
| [`rust.md`](./rust.md) | Rust 2021 edition style, error model, async patterns, performance |
| [`audio.md`](./audio.md) | Real-time audio safety, cpal, rubato, ScreenCaptureKit |
| [`tauri.md`](./tauri.md) | Tauri 2 commands, capabilities, IPC, packaging |
| [`react.md`](./react.md) | React 18 patterns, hooks, state, accessibility |
| [`typescript.md`](./typescript.md) | TypeScript strictness, type-only imports, module layout |
| [`styling.md`](./styling.md) | Tailwind, shadcn/ui, design tokens, dark mode |
| [`security.md`](./security.md) | Threat model, CSP, capabilities, secret handling |
| [`accessibility.md`](./accessibility.md) | Keyboard, screen reader, focus, motion |
| [`performance.md`](./performance.md) | Profiling, hot-path discipline, allocation budget |
| [`testing.md`](./testing.md) | Unit, integration, audio-with-synthetic-signals |
| [`commits.md`](./commits.md) | Commit message style, branch names, PR shape |
| [`open-source.md`](./open-source.md) | Public release checklist, license headers, governance |
| [`tooling.md`](./tooling.md) | Pre-commit, formatters, CI, dependency hygiene |

## How to use this folder

1. When adding new code, scan the relevant document and follow it.
2. When a rule blocks a real task, change the rule first (PR to this folder),
   then change the code. Standards drift if exceptions stack up silently.
3. When you genuinely disagree with a standard, open an issue. "It's there
   because someone wrote it once" is not a defence.

## How this differs from `AGENTS.md`

`AGENTS.md` at the repo root is the contract with AI agents about *how to work
in this repo* (which commands to run, which conventions to follow, where to
look for design rationale). This folder is the body of conventions those
agents follow. Keep them in sync: if you tighten a rule here, mention it in
`AGENTS.md` if it affects agent behaviour.
