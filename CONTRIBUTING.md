# Contributing to Attune

Thanks for considering a contribution. This document covers setup, conventions, and the review process.

## Setup

### Prerequisites

- macOS 13.3 or later (Apple Silicon recommended)
- Rust 1.88+ via `rustup` (the toolchain is pinned in `rust-toolchain.toml`)
- [Bun](https://bun.sh) 1.3+ (the only JS package manager + runtime this repo uses)
- Xcode command-line tools: `xcode-select --install`

### First-time setup

```sh
git clone git@github.com:woosal1337/attune.git
cd attune
bun install
pre-commit install
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push
bun tauri dev
```

If `pre-commit` is not installed system-wide:

```sh
brew install pre-commit         # or: pip install pre-commit
```

### Editor

The repo ships with:

- `.editorconfig` for indentation and line endings
- `.prettierrc.json` for TypeScript / React / CSS formatting
- `rustfmt.toml` for Rust formatting
- `eslint.config.js` for TypeScript / React linting
- `clippy.toml` for Rust linting

Recommended VS Code / Cursor extensions: `rust-analyzer`, `tauri-vscode`, `tailwindcss-intellisense`, `prettier-vscode`, `dbaeumer.vscode-eslint`.

## Conventions

The authoritative style contract lives at [`docs/CODE_STYLE.md`](docs/CODE_STYLE.md). Read it first. It covers naming, comments, errors, logging, tests, concurrency, performance, security, git hygiene, and the public-release hygiene checklist. The summary below highlights the rules contributors hit most often.

### Rust

- Typed errors via `thiserror`. New variants go in `crates/attune-core/src/error.rs` (`AttuneError`).
- No `unwrap()` outside `#[cfg(test)]`. `expect("reason")` is acceptable for invariants.
- All `unsafe` blocks need a `// SAFETY:` justification comment.
- Logging via `tracing`, never `println!`.
- 4-space indentation, 100-char width (enforced by `rustfmt.toml`).
- No allocations on audio hot paths; pre-allocate buffers.
- **No inline `//` body comments.** Doc-comments above declarations only — see [`docs/CODE_STYLE.md` §1](docs/CODE_STYLE.md#1-comments) for the rule and its four exceptions.

### TypeScript / React

- Strict mode is on. No `any`. Use `import type` for type-only imports.
- Function components only. Effects must have a cleanup if they subscribe.
- Tauri command names live in `src/shared/lib/ipc.ts`; types are generated from Rust via `ts-rs` into `src/shared/types/` — never edit those by hand.
- **No inline `//` or `/* */` body comments.** JSDoc above the export only — see [`docs/CODE_STYLE.md` §1](docs/CODE_STYLE.md#1-comments).
- Effects MUST clean up subscriptions; race-condition guards use a local `cancelled` flag. See [`docs/CODE_STYLE.md` §6.2](docs/CODE_STYLE.md#62-typescript).

### Git

Conventional-commit style with a scope, lowercase subject, imperative mood:

```
<type>(<scope>): <subject>

<optional body explaining WHY>
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Enforced by `commit-msg` hook.

- Commits are **GPG-signed**. The pre-commit hooks enforce this.
- Commits are **never co-authored by an AI agent.** No `Co-Authored-By:` trailers.
- The full git hygiene section is [`docs/CODE_STYLE.md` §10](docs/CODE_STYLE.md#10-git-hygiene).

## Workflow

### Branching

```sh
git checkout -b feat/<short-name>     # new feature
git checkout -b fix/<short-name>      # bug fix
git checkout -b refactor/<scope>      # internal restructuring
git checkout -b docs/<topic>          # documentation only
git checkout -b chore/<scope>         # tooling, build, deps
```

### Local commands

```sh
# Frontend (bun runs the npm scripts in package.json)
bun run dev           # vite dev server
bun run typecheck     # tsc --noEmit
bun run lint          # eslint
bun run format        # prettier --write

# Backend
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo fmt --all

# Full app
bun tauri dev         # development build
bun tauri build       # release build
```

`pre-commit` runs the relevant subset of these automatically on every commit.

### Pull requests

- Open a PR against `main`.
- CI must be green before review: `fmt`, `clippy`, `cargo test`, frontend `lint` + `typecheck`, and `cargo-deny`.
- Include a clear description of the _why_; the diff already shows the _what_.
- Link to the issue if one exists.
- Keep PRs scoped to one concern. Refactors and feature work go in separate PRs.

### Reporting bugs and proposing features

Use the GitHub issue templates under `.github/ISSUE_TEMPLATE/`. For non-trivial features, open a discussion or issue first to align on approach before writing code.

## Code of conduct

Be kind, be precise, assume good faith. Hostile or harassing behavior is grounds for removal.

## License

By contributing, you agree that your contributions will be licensed under the MIT License, the same terms as the rest of the project (see `LICENSE`).
