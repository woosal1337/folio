<!--
Thanks for the PR. Please fill in the sections below.
Keep the title in conventional-commit style: type(scope): subject.
-->

## Summary

<!-- One paragraph: what changed and WHY. The diff already shows WHAT. -->

## Type of change

- [ ] feat — new user-facing capability
- [ ] fix — bug fix
- [ ] refactor — internal restructuring, no behavior change
- [ ] perf — performance improvement
- [ ] docs — documentation only
- [ ] test — tests only
- [ ] build / ci / chore — tooling, dependencies, repo hygiene

## Test plan

<!-- How was this verified? Include commands run, manual steps, screenshots. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `bun run typecheck`
- [ ] `bun run lint`
- [ ] Manually exercised the feature in `bun tauri dev`

## Linked issues

<!-- Closes #123, refs #456. -->

## Notes for reviewers

<!-- Anything reviewers should know first: structural choices, follow-ups, areas of concern. -->
