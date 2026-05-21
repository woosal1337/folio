# Open Source Standards

Attune is moving to a public repository. This document is the checklist
and the rules for keeping it healthy after the door opens.

## Pre-release checklist

Code:

- [ ] No secrets in any committed file (`.env*`, API keys, hardcoded
      credentials). `git log --all -S "sk-" -p` returns nothing.
- [ ] No internal URLs, paths, or proprietary names anywhere in the
      tree. Code, comments, docs, error messages, log lines.
- [ ] `cargo fmt --check` clean.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace --lib --bins` green.
- [ ] `pnpm tsc -b` and `pnpm lint` green.
- [ ] `pnpm prettier --check .` green (or whatever frontend formatter
      is wired).
- [ ] `cargo audit` clean. RUSTSEC advisories addressed or accepted
      with a documented reason.
- [ ] Bundle builds: `cargo tauri build` produces a signed `.app`
      (signing/notarisation may be deferred but the build must succeed).

Repository:

- [ ] `README.md` describes what the project is, who it's for, how to
      build, and how to contribute.
- [ ] `LICENSE` is correct (MIT, see `LICENSE` file at the root).
- [ ] `CONTRIBUTING.md` explains the workflow for outside contributors.
- [ ] `CODE_OF_CONDUCT.md` is present (Contributor Covenant 2.1).
- [ ] `SECURITY.md` has a reachable contact for vulnerability reports.
- [ ] `CHANGELOG.md` updated.
- [ ] `.github/ISSUE_TEMPLATE/` covers bug reports and feature requests.
- [ ] `.github/PULL_REQUEST_TEMPLATE.md` exists.
- [ ] `.github/dependabot.yml` configured for Cargo + npm.

CI:

- [ ] CI fails on every check above. Reviewers should trust the badge.
- [ ] Required status checks set on `main` (cannot merge without
      green CI).
- [ ] Branch protection requires at least one review.

## License

- MIT. See `LICENSE`.
- Every new top-level Rust file gets the SPDX header on the first line:
  `// SPDX-License-Identifier: MIT`. Module docs follow on subsequent
  lines.
- Every new top-level TypeScript file gets a similar header:
  `// SPDX-License-Identifier: MIT`.
- Vendored code (rare) keeps its own license headers and gets a line
  in `THIRD_PARTY_LICENSES.md`.

## Code of Conduct

- Contributor Covenant v2.1.
- Reports go to the email in `CODE_OF_CONDUCT.md`. Triage within 72
  hours.

## Contributing

`CONTRIBUTING.md` is the entry point for outside contributors. It covers:

- How to set up the dev environment (Rust toolchain, pnpm, Tauri CLI).
- How to run tests, format, lint.
- The branch / PR conventions in `standards/commits.md`.
- Where to ask questions (issues for code, discussions for design
  conversations).

We accept PRs that:

- Reference an open issue, or are small enough that the PR description
  is sufficient context.
- Pass CI.
- Include tests when adding behaviour.
- Follow the formatting, naming, and style rules in `standards/`.

We decline PRs that:

- Add features that contradict the privacy posture ("Audio never
  leaves your machine").
- Add dependencies without justification.
- Bypass review with `--force-push` / `--no-verify`.
- Rewrite history on shared branches.

## Issue triage

- New issues get a label within one working day: `bug`, `feature`,
  `question`, `docs`, or `triage` if more info is needed.
- "Good first issue" applies to issues where the fix is < 50 lines and
  doesn't require deep context. Tag generously; outside contributors
  start here.
- Stale issues (no activity for 90 days, no priority label) are closed
  with a polite message inviting a reopen.

## Versioning

- SemVer per `standards/commits.md`. v0.x.y while building toward v1.
- Breaking changes between v0 minor versions are allowed and called
  out in `CHANGELOG.md` under `### Breaking`.
- v1.0 is a commitment to API stability for `attune-core` and the
  Tauri command surface.

## Release cadence

- v0.x: released when there's enough to ship a coherent slice.
- v1.0: targeting 2026-07-15 per the v0 plan in the design vault.
- Patch releases (`v0.x.y`) are unscheduled; cut when a fix or
  security update warrants.

## Communication

- GitHub Issues: bugs, concrete feature requests, build problems.
- GitHub Discussions: design conversations, "would this be welcome?"
  questions, end-user help.
- Security reports: email per `SECURITY.md`, never public.

## Governance (current)

- Maintainer: project owner. All merges go through their review until
  the project grows past one full-time committer.
- Contributors retain copyright on their contributions; the MIT
  license grants the project the rights it needs.
- Major direction changes (architecture rewrites, license changes,
  governance restructuring) are RFCs in `architecture/` of the design
  vault.

## Dependencies (public-facing)

- Each new top-level dependency requires a one-line justification in
  the relevant `Cargo.toml` / `package.json`.
- Dependency upgrades that change behaviour are flagged in the PR
  description (`Changes upstream`).

## Documentation

- `README.md` is the marketing surface and the build-from-source guide.
- `standards/` is the conventions surface.
- `AGENTS.md` is the contract with AI agents.
- Design docs live in the Obsidian vault per `AGENTS.md`. Public-facing
  excerpts (architecture overview, threat model) get a section in
  `README.md` once the vault is the source of truth.
- API documentation: `cargo doc --workspace --no-deps` runs in CI;
  publish to a project page once we have a stable surface.

## Telemetry posture (public)

- No telemetry in v0. No crash reporting. No analytics. The README and
  privacy stance say so.
- Future opt-in crash reporting redacts audio paths and content.
- Any future telemetry change requires a PR with the justification and
  the privacy stance update in the same PR.

## Trademark and naming

- "Attune" is a project name; we do not claim trademark. Forks are
  welcome and should rename to avoid confusion if they diverge.
- Logo and visual identity files in `src/assets/` and
  `src-tauri/icons/` are MIT-licensed alongside the code.
