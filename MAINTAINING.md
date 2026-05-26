# Maintaining Attune

How the maintainer's daily workflow runs. Strangers shipping a one-off
fix should start with [`CONTRIBUTING.md`](./CONTRIBUTING.md) instead —
this doc describes the toolchain the project author uses across
Linear, GitHub, and the local Obsidian vault. None of it is required
for outside contributions.

## The triad

```
   ┌─────────────────────┐
   │ Obsidian vault      │
   │ ~/Documents/GitHub/ │   architecture, plans, decisions
   │ obsidian.md/        │   live here in markdown.
   │ projects/attune/    │
   └──────────┬──────────┘
              │ cited from
              ▼
   ┌─────────────────────┐
   │ This repo           │
   │ github.com/         │   code, tests, CI, docs/.
   │ woosal1337/attune   │
   └──────────┬──────────┘
              │ tracked in
              ▼
   ┌─────────────────────┐
   │ Linear              │
   │ getattune team      │   GET-<n> issue per roadmap item.
   └─────────────────────┘
```

Vault drives the design, GitHub drives the code, Linear drives the
work. Every shipped PR cites at least one vault doc (or v2 finding
id like `GET-42`) and closes one Linear issue.

## Daily flow

1. **Pick an issue from Linear.** Filter by `state:Todo`. Move it to
   `In Progress` via `mcp__linear-server__save_issue`. Snapshot the
   description into the PR body.
2. **Branch.** `git checkout -b <type>/get-<n>-<slug>` per
   `CONTRIBUTING.md` §"Branching".
3. **Cite the vault doc.** Read the matching architecture or plan
   doc at `~/Documents/GitHub/obsidian.md/projects/attune/<section>/`.
   Note its filename in the commit body.
4. **Write the change.** Follow `docs/CODE_STYLE.md`.
5. **Test locally.** `cargo fmt && cargo clippy --workspace
   --all-targets -- -D warnings && cargo test --workspace
   --all-targets && bun run typecheck && bun run lint && bun run test`.
6. **Commit.** Conventional commit (`feat(get-<n>): <subject>`),
   GPG-signed, **no `Co-Authored-By:` trailers**.
7. **Push + PR.** `gh pr create --title "<type>(get-<n>): <title>"
   --body "Closes GET-<n>"`.
8. **Merge.** `gh pr merge --merge --delete-branch`. Branch protection
   on `main` requires CI green; the maintainer can `--admin` past
   for emergency fixes only.
9. **Close the Linear issue.** `mcp__linear-server__save_issue` →
   `state:Done`, attach the PR URL.
10. **`git pull --ff-only`** back on main.

The full ten-step sequence is automated end-to-end by the agents
working out of `~/.claude/agents/`.

## Vault → repo round-trip

When a vault doc changes (a new architecture decision, a new plan, a
revised roadmap):

1. Edit the markdown locally in `~/Documents/GitHub/obsidian.md/`.
2. `git commit + git push` from inside the vault repo (separate from
   the Attune repo).
3. eBrain pulls the change within ~60s and re-embeds it; queries via
   `mcp__ebrain__query` see the new content the next time they run.
4. If the doc relates to a roadmap item, file or update the
   corresponding Linear issue. The Linear issue stays a one-line
   summary + GET-<n> tag pointing at the vault doc.

## Code-style enforcement

`docs/CODE_STYLE.md` rev 2 (2026-05-26) is the contract. Every PR
reviewer runs the checklist in §11.1 mentally. The public-release
gate doc at `docs/refactor/PHASE-3-PUNCH-LIST.md` lists every audit
item; new violations land in §3 (P1) or §4 (P2) of that doc rather
than as one-off issues.

## Release cadence

See `docs/guidelines/release-engineering.md`. tl;dr:

- Patch releases (`1.0.x`) ship on the same `release.yml` pipeline
  whenever there's a bug-fix accumulation.
- Minor releases (`1.x.0`) ship on a roughly monthly cadence once
  the public flip happens.
- Major (`x.0.0`) reserved for incompatible IPC changes or paid-tier
  rollouts.

## Tooling note

The agents (Claude Code, Codex) use:

- `mcp__linear-server__*` for the Linear surface.
- `mcp__ebrain__*` for vault search.
- `mcp__media-mcp__*` for transcript / Twitter / YouTube research
  during ideation.
- `gh` CLI for everything GitHub-side.
- `rtk` (Rust Token Killer) wrapping every `git`, `cargo`, `gh`, and
  `npm` call to compress shell output in agent context. See `~/.claude/RTK.md`.

## Where to go when stuck

- Read the matching vault doc first.
- For unanswered questions, check `~/Documents/GitHub/obsidian.md/
  projects/attune/notes/`.
- For absolute-last-resort questions, file a GitHub issue and let
  the maintainer answer publicly so the answer is searchable.
