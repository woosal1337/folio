# Contributing to Attune

Thanks for taking the time to look. Attune is a small, focused project: a
local-first meeting transcription app for macOS. Contributions are welcome,
and there are a few conventions that keep the codebase healthy.

If you only read one thing, read [`AGENTS.md`](./AGENTS.md). It is the
contract every contributor — human or AI — follows. The deeper conventions
live in [`standards/`](./standards/).

## Getting started

Requirements:

- macOS 14.4+ on Apple Silicon (Intel works but is not the perf target).
- Rust 1.88 via [rustup](https://rustup.rs).
- Node.js 22 LTS and [pnpm](https://pnpm.io) 10.x.
- Xcode 16+ command line tools.

```sh
git clone https://github.com/woosal1337/attune.git
cd attune
pnpm install
cargo build --workspace
```

Run the desktop app:

```sh
pnpm tauri dev
```

Run the CLI:

```sh
cargo run -p attune-cli -- --help
```

## Workflow

1. **Pick or open an issue.** Small fixes don't need an issue first; new
   features and any user-visible behaviour change do.
2. **Create a branch.** Naming: `feature/<short>`, `fix/<short>`,
   `chore/<short>`, `docs/<short>`. See
   [`standards/commits.md`](./standards/commits.md).
3. **Write the change.** Follow the standards documents:
   - [Rust](./standards/rust.md)
   - [TypeScript](./standards/typescript.md)
   - [React](./standards/react.md)
   - [Audio](./standards/audio.md)
   - [Security](./standards/security.md)
   - [Accessibility](./standards/accessibility.md)
   - [Performance](./standards/performance.md)
4. **Run the local gates.**

   ```sh
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --lib --bins
   pnpm tsc -b
   pnpm lint
   pnpm prettier --check .
   ```

   Or install [pre-commit](https://pre-commit.com) and let it run them
   automatically:

   ```sh
   brew install pre-commit
   pre-commit install
   ```

5. **Open a PR.** Title in commit-subject style; description that covers
   what, why, how to verify, and any risk. Screenshots for UI changes.

## What we accept

We happily merge:

- Bug fixes with a clear reproduction.
- Small, focused features that move the v0/v1 plan forward.
- Documentation improvements.
- Test additions.
- Tooling and CI improvements.

We will ask you to revise:

- Changes that violate the privacy posture ("Audio never leaves your
  machine" — see [`standards/security.md`](./standards/security.md)).
- Features that contradict the architecture (see the design vault).
- New dependencies without a justification.
- Diff-bombs (~1000+ lines). Stack the change instead.

## Commit style

```
<scope>: <imperative summary>

Body explaining WHY. Wrapped at 72 chars.
```

`scope` is the area: `core`, `app`, `cli`, `ui`, `build`, `ci`, `docs`,
`deps`, `standards`. See [`standards/commits.md`](./standards/commits.md)
for the full guide and examples.

## Reviewing

If you have commit access:

- Aim to leave a substantive comment within one working day, or pass
  the review on.
- Reply to author responses promptly.
- "LGTM" alone is not a review — at least one line documenting what
  you checked.

## Reporting issues

- **Bugs**: open an issue with the `bug` template. Include macOS
  version, hardware (Apple Silicon vs Intel), recent commit you tried,
  steps to reproduce, expected behaviour, actual behaviour, logs.
- **Features**: open an issue with the `feature` template. Describe
  the user problem first, then a proposed shape if you have one.
- **Security**: do not file a public issue. See [`SECURITY.md`](./SECURITY.md).

## Communication

- **Issues**: bugs, specific feature requests, build problems.
- **Discussions**: design questions, "would this be welcome?" before
  starting a large change.
- **Direct contact**: only for security and conduct reports.

## Code of conduct

By participating, you agree to follow
[`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md). It applies to issues,
PRs, discussions, and anywhere this project is represented.

## License

Attune is MIT-licensed. By contributing, you agree that your work is
contributed under the same license. See [`LICENSE`](./LICENSE).
