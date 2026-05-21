# Tooling Standards

The tools that keep the codebase consistent without anyone having to ask.

## Required tools

| Tool | Version | Why |
| --- | --- | --- |
| Rust | 1.88 (pinned in `rust-toolchain.toml`) | Workspace baseline |
| `rustfmt` | bundled with toolchain | Formatting |
| `clippy` | bundled with toolchain | Linting |
| Node.js | 22 LTS | Vite + React build |
| pnpm | 10.x | Package manager (the lockfile we trust) |
| Tauri CLI | 2.x (via `@tauri-apps/cli`) | Dev / build |
| Xcode CLT | 16+ | macOS-only system frameworks |

## Optional but recommended

| Tool | Why |
| --- | --- |
| `cargo-nextest` | Faster test runner |
| `cargo-audit` | Vulnerability scanning |
| `cargo-deny` | License/advisory gating |
| `samply` | CPU profiling |
| `cargo-llvm-cov` | Coverage (not enforced) |

## Formatters

### Rust

- `cargo fmt --all` is the canonical formatter. `rustfmt.toml` is its
  config; do not hand-edit formatting decisions.

### TypeScript / JavaScript

- Prettier is the canonical formatter for `.ts`, `.tsx`, `.js`, `.mjs`,
  `.json`, `.md`. Config lives in `.prettierrc.json`.
- 2-space indent, 80-char line width, no trailing commas in JSON (the
  default), trailing commas everywhere else, double quotes for strings.
- Plugin: `prettier-plugin-organize-imports` orders TypeScript imports
  on save / format.

### Other

- `.editorconfig` covers the basics for every file: LF endings, UTF-8,
  trim trailing whitespace, final newline. YAML/JSON/Markdown use
  2-space indent; the rest uses 4-space.

## Linters

### Rust

- `cargo clippy --workspace --all-targets -- -D warnings` runs in CI
  and locally.
- The shared `[workspace.lints.clippy]` block holds per-rule
  exceptions. Comment each exception with the reason.

### TypeScript / React

- ESLint flat config at `eslint.config.js`. Plugins:
  - `@typescript-eslint`
  - `eslint-plugin-react`
  - `eslint-plugin-react-hooks`
  - `eslint-plugin-jsx-a11y`
- Rules summary:
  - `@typescript-eslint/no-explicit-any`: error
  - `@typescript-eslint/no-unused-vars`: error (allow `_` prefix to opt
    out)
  - `react-hooks/rules-of-hooks`: error
  - `react-hooks/exhaustive-deps`: warn
  - `jsx-a11y/no-static-element-interactions`: warn
- `pnpm lint` runs `eslint src --max-warnings 0`. CI fails on any
  warning.

### TypeScript type check

- `tsc -b` runs as part of `pnpm build` and in CI. Build fails on type
  errors.

### Markdown

- Prettier handles formatting. We do not enforce markdownlint; the
  ROI is low for the volume of prose we produce.

## Pre-commit

We use the `pre-commit` framework (`pre-commit run --all-files` works
identically with `pip`, `brew`, or any installer). Config lives in
`.pre-commit-config.yaml`.

Hooks:

- `trailing-whitespace`, `end-of-file-fixer`, `check-yaml`,
  `check-toml`, `check-json`, `mixed-line-ending`, `check-added-large-files`,
  `check-merge-conflict` — basic hygiene.
- `cargo fmt --check` — Rust formatting.
- `cargo clippy -- -D warnings` — Rust lint. Slow but worth it.
- `prettier --check` — frontend formatting.
- `eslint` — frontend lint.
- `typos` — common misspellings.
- `actionlint` — GitHub Actions YAML lint.

Install with:

```sh
brew install pre-commit
pre-commit install
```

The first run downloads each tool's pinned version. Subsequent runs
are fast.

## CI

`.github/workflows/ci.yml` runs on push to `main` and on every PR.
Jobs:

1. **rust-fmt** — `cargo fmt --all -- --check`.
2. **rust-clippy** — `cargo clippy --workspace --all-targets -- -D warnings`.
3. **rust-test** — `cargo build --workspace --all-targets` then
   `cargo test --workspace --lib --bins`.
4. **frontend-typecheck** — `pnpm tsc -b`.
5. **frontend-lint** — `pnpm lint`.
6. **frontend-format** — `pnpm prettier --check .`.

All jobs run on `macos-14` so platform-conditional code is exercised.

## Dependabot

`.github/dependabot.yml` configures weekly updates for:

- Cargo dependencies in the workspace.
- npm dependencies in the frontend.
- GitHub Actions.

Major-version bumps land in their own PR for review. Minor and patch
updates auto-merge if CI passes (manual review still encouraged).

## Releases

- `release.yml` (planned) builds universal macOS binaries, signs and
  notarises, and uploads to the GitHub release page. Tag-driven.
- The Cask update workflow runs after release to bump
  `homebrew-attune/Casks/attune.rb`.

## Local dev commands

```
cargo fmt --all                                                    # format Rust
cargo clippy --workspace --all-targets -- -D warnings              # lint Rust
cargo test --workspace --lib --bins                                # test Rust
cargo run -p attune-cli -- devices                                 # CLI test
cargo tauri dev                                                    # full app dev

pnpm install                                                       # install JS deps
pnpm dev                                                           # vite dev server (used by tauri dev)
pnpm build                                                         # tsc + vite build
pnpm lint                                                          # eslint
pnpm tsc -b                                                        # type check
pnpm prettier --write .                                            # format
pnpm prettier --check .                                            # check format

pre-commit install                                                 # install git hooks
pre-commit run --all-files                                         # run all hooks
```

## Adding a tool

When a new tool joins the list:

1. Add the version to the relevant `Cargo.toml` / `package.json`
   `devDependencies`.
2. Document the rule it enforces in the appropriate `standards/*.md`.
3. Add the hook to `.pre-commit-config.yaml` if it's pre-commit-able.
4. Add the job to CI.
5. Update this document.

Steps 4 and 5 are not optional. A tool that only some contributors run
is worse than no tool at all.
