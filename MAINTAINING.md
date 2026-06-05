# Maintaining Folio

How the maintainer's day-to-day workflow runs. Strangers shipping a
one-off fix should start with [`CONTRIBUTING.md`](./CONTRIBUTING.md)
instead — this doc describes maintainer conventions and is not required
for outside contributions.

## How the work is organized

Design docs drive the architecture, this repo drives the code, and an
issue tracker drives the work. Every shipped PR closes one tracked
issue and, where relevant, cites the design rationale behind it. Issues
use the `GET-<n>` identifier scheme (e.g. `GET-42`) — that prefix shows
up throughout the commit and PR history tying code back to the decision
that introduced it.

## Daily flow

1. **Pick an issue.** Move it to `In Progress` and snapshot the
   description into the PR body.
2. **Branch.** `git checkout -b <type>/get-<n>-<slug>` per
   `CONTRIBUTING.md` §"Branching".
3. **Write the change.** Follow `docs/CODE_STYLE.md`.
4. **Test locally.** `cargo fmt && cargo clippy --workspace
--all-targets -- -D warnings && cargo test --workspace
--all-targets && bun run typecheck && bun run lint && bun run test`.
5. **Commit.** Conventional commit (`feat(get-<n>): <subject>`).
6. **Push + PR.** `gh pr create --title "<type>(get-<n>): <title>"
--body "Closes GET-<n>"`.
7. **Merge.** `gh pr merge --merge --delete-branch`. Branch protection
   on `main` requires CI green; use `--admin` for emergency fixes only.
8. **`git pull --ff-only`** back on main.

## Code-style enforcement

`docs/CODE_STYLE.md` is the contract. Every PR reviewer runs its
checklist. New violations are tracked as issues rather than silently
merged.

## Release cadence

See `docs/guidelines/release-engineering.md`. tl;dr:

- Patch releases (`1.0.x`) ship on the `release.yml` pipeline whenever
  there's a bug-fix accumulation.
- Minor releases (`1.x.0`) ship on a roughly monthly cadence.
- Major (`x.0.0`) is reserved for incompatible IPC changes.

## Where to go when stuck

For unanswered questions, file a GitHub issue and let the maintainer
answer publicly so the answer is searchable.
