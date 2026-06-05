# Folio Rust Code Guidelines

A detailed, checkable rule set distilled from ~50 authoritative Rust sources
(the official Rust API Guidelines, Style Guide, Rustonomicon, Performance Book,
Tokio/async docs, Effective Rust, Rust Design Patterns, and senior practitioners:
matklad, BurntSushi, Alice Ryhl, Niko Matsakis, pretzelhammer, Luca Palmieri,
Sabrina Jewson, Jon Gjengset, David Drysdale).

These documents are the _reference library_. The repo's enforced, opinionated
house rules live in [`../CODE_STYLE.md`](../CODE_STYLE.md) and the subsystem docs
under [`../guidelines/`](../guidelines/). Where this library and `CODE_STYLE.md`
overlap, **`CODE_STYLE.md` wins** — it is the project's canon. This library
exists to (a) explain the _why_ behind those rules with citations, and (b) cover
ground `CODE_STYLE.md` leaves implicit.

## Contents

| File                                                               | Topic                                                                                                            |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| [01-style-naming-api-design.md](01-style-naming-api-design.md)     | Naming (RFC 430), rustfmt formatting, API design, trait design, generics, docs, public-API hygiene               |
| [02-error-handling.md](02-error-handling.md)                       | Result vs panic, `unwrap`/`expect` discipline, `?`/`From`, thiserror/anyhow, locks, FFI, validation              |
| [03-ownership-types-idioms.md](03-ownership-types-idioms.md)       | Ownership/borrowing, lifetimes, newtype/typestate/builder, conversions, iterators, smart pointers, anti-patterns |
| [04-async-concurrency.md](04-async-concurrency.md)                 | Blocking the executor, locks across `.await`, lock choice, channels/actors, deadlocks, `select!`, shutdown       |
| [05-project-structure-modules.md](05-project-structure-modules.md) | Workspace/crate layout, module/file layout, visibility, IO/logic separation, when to abstract                    |
| [06-performance.md](06-performance.md)                             | Allocations, iterators, `#[inline]`, release profile, clippy perf gate                                           |
| [07-testing.md](07-testing.md)                                     | Test layout, assertions, matklad test design, determinism, fixtures/snapshots                                    |
| [08-unsafe.md](08-unsafe.md)                                       | When to use `unsafe`, validity vs safety invariants, `// SAFETY:` + `# Safety` docs                              |
| [09-tauri.md](09-tauri.md)                                         | Command design, managed state, capabilities/permissions, security                                                |

## How to use

- **Writing code:** skim the relevant file; the rules are imperative and short.
- **Reviewing code:** each rule is checkable. The audit that produced the
  initial Linear task backlog (see `GUIDELINE-` issues) checked the codebase
  against these.
- **Disagreeing:** if a rule conflicts with a real project constraint, document
  the deviation in `CODE_STYLE.md` rather than silently ignoring it.

> Scope note: the project is Tauri (Rust) + React/TypeScript. These guidelines
> target the **Rust** code (`crates/folio-core`, `crates/folio-cli`,
> `src-tauri`). TypeScript/React conventions live in `CODE_STYLE.md` §2/§9.2 and
> `docs/guidelines/frontend-architecture.md`.
