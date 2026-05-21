# Commits, Branches, PRs

Small, focused commits with messages that survive a year. Branch names
that describe the work. PRs that someone else can review without a Zoom
call.

## Commit messages

Format:

```
<scope>: <imperative summary in 60 chars or fewer>

<body explaining WHY this change is needed.
Wrapped at 72 chars. Optional but encouraged.>

<optional footer: refs, links, co-authors>
```

`scope` is the area of the codebase: `core`, `app`, `cli`, `ui`,
`build`, `ci`, `docs`, `deps`, `standards`. Multi-scope commits are
fine to keep small commits possible; if you find yourself listing four
scopes, the commit is too big.

Examples:

```
core: optimize WAV writer with batched quantization

The per-sample clamp + cast in the inner loop produced one branch
per sample. Hoisting to a single pass over the slice halves the
callback cost on Apple Silicon and removes the per-sample mutex
contention path.

Measured with samply on M2 Pro, 48 kHz mic capture, 480-frame
buffers: 18 µs → 9 µs median per buffer.
```

```
ui: replace JSX section comments with semantic regions

Section labels were inline JSX comments that didn't survive
production builds. Promoted to `<section aria-labelledby="...">`
where the boundary is real; deleted where the section was a
visual layout boundary only.
```

```
ci: enforce frontend type-check and lint

Existing CI only covered the Rust workspace. Adds tsc + eslint
gates so refactor breakage is caught before merge.
```

## Rules

- Imperative mood. "add", "fix", "remove" — not "added", "adds".
- No trailing period in the subject line.
- The body explains *why*. The diff explains *what*.
- "Misc cleanup" / "address PR comments" are red flags. Prefer one
  commit per logical change; rebase before pushing if the local
  history is messy.
- Co-authors get `Co-authored-by:` trailers.
- Don't include automation-specific trailers (model identifiers,
  session URLs, agent IDs) in the public history. Those belong in PR
  descriptions for context, not in the commit log.

## Branch names

- `feature/<short-name>` for new functionality.
- `fix/<short-name>` for bug fixes.
- `chore/<short-name>` for tooling, dependency bumps, refactors.
- `docs/<short-name>` for documentation-only changes.
- `release/v0.x.y` for release branches.
- Short and kebab-cased. The branch name is part of the PR header.

## Pull requests

Each PR has:

- A title that reads like a commit subject. Repeats the work in one
  line: "core: optimize WAV writer with batched quantization".
- A description that covers:
  - **What** changed (the high-level diff in plain English).
  - **Why** it changed (the problem being solved).
  - **How** to verify (steps to test manually, or "covered by tests").
  - **Risk** (anything reviewers should pay attention to).
- A link to the related issue or design doc if applicable.
- Screenshots for UI changes (before/after when relevant).

## PR size

- Aim for diffs that one reviewer can hold in their head. ~300 lines
  changed is comfortable; ~1000 lines starts to overflow.
- Big mechanical changes (renames, lint-driven reformatting) ship in
  their own PR, separately from logic changes. Reviewers should not
  have to disentangle "this rename" from "this bug fix".
- Stack PRs when one logical change is too large for a single review.
  The base PR lays groundwork; subsequent PRs target the base branch.

## Review

- Reviewers respond within one working day or pass the review on.
- Authors respond to feedback within one working day or comment on
  why not.
- "LGTM" alone is not a review. At least one substantive line of
  feedback per reviewer, even if it's "I checked the locking
  behaviour and it's correct."
- Reviewers leave their disagreement in code comments; if the
  disagreement persists, escalate to async chat — the PR thread is
  not the place to argue at length.

## Merging

- Squash and merge for short branches with one logical change.
  Rebase and merge for stacked work. No merge commits to `main`.
- Final commit message on `main` matches the format above. The PR
  description body becomes the commit body when squash-merging.
- Delete the branch after merge. Old branches accumulate and confuse
  navigation.

## Reverts

- Reverting is a normal operation. "Revert <subject>" with a short
  body explaining the reason. Followups (`Revert "Revert ..."`) are
  fine; the history is the record.

## Tags and releases

- Semantic versioning per `CHANGELOG.md`. Tag format is `v0.1.2`
  (lowercase `v`). The tag points at the merge commit on `main`.
- Releases are created from tags via the `release.yml` workflow.
  Manual releases are last-resort.
