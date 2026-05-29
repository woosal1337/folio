# 05 — Project Structure & Modules

From matklad (Large Rust Workspaces, How to Test, Fast Rust Builds, Inline in
Rust, Caches in Rust), the Rust Book ch.7, Effective Rust, Luca Palmieri, Jon
Gjengset. `CODE_STYLE.md` §9 is canon (layer dependency rule; module org).

## 1. Workspace & crates

- Use a **flat `crates/`** layout, not a nested tree (the Cargo namespace is flat).
- Make the workspace root a **virtual manifest** (`[workspace]` only, no root pkg).
- Name each crate like its folder; set `version = "0.0.0"` for internal crates.
- **Keep binaries thin; put logic in a library crate** the binary calls
  (`main.rs` = args + IO + exit codes only). *(Attune already does this:
  `attune-core` lib, `attune-cli`/`src-tauri` thin.)*
- Shape the dep graph wide (diamond), not a linear chain, for parallel compiles.
- A crate is a compilation + encapsulation boundary; split a growing crate when it
  becomes a recompile or coupling bottleneck.

## 2. Module & file layout

- Prefer the **file-module style** (`foo.rs` + `foo/`) over `mod.rs` files.
  *(Attune uses `mod.rs` re-export modules — acceptable; keep them logic-free.)*
- Declare each module with `mod` exactly once; directory structure mirrors the
  module hierarchy.
- Re-export the public surface with `pub use` to flatten internal nesting.
- One module = one concept; keep functions and files cohesive.

## 3. Visibility discipline

- Default to private; add `pub` only to the deliberate API.
- Use `pub(crate)` for cross-module-but-not-exported items.
- Keep struct fields private; expose behavior through methods.
- Avoid wildcard (`use foo::*`) imports; seal traits not meant for downstream impl.
- Re-export any dependency type that appears in your public API.

## 4. IO vs pure logic (testability)

- **Push IO to the caller; keep core logic as pure fns** (data in → data out).
  IO, not code volume, dominates test time.
- Never fire-and-forget background work; return a handle tests can join.
- Test at a stable boundary / public API ("the neural-network test"), not internal
  fns; funnel calls through one `check()` helper.
- Prefer real in-memory deps over mocks; mock only true external IO.
- Make adding a test trivial (data-driven inputs, snapshot/`expect-test`).
- Gate slow tests behind an env var (e.g. Attune's `ATTUNE_WHISPER_MODEL` /
  `--ignored`), not `cfg`.

## 5. When to abstract vs inline

- Don't reach for generics/traits/lifetimes until a concrete need appears.
- **Confine generic code to a thin public wrapper that delegates to a non-generic
  inner fn** — generics monomorphize per crate; outlining the body prevents
  duplicated machine code and slow builds.
- Use `&dyn Fn()` instead of `impl Fn()` for closure params when you don't need
  inlining.
- Choose `dyn Trait` vs generics deliberately (flexibility/size vs speed/bloat).
- Prefer interior mutability over a viral `&mut self` getter; encapsulate `unsafe`
  interior mutability in vetted crates (`once_cell`), don't hand-roll.

## 6. Dependencies

- Audit deps from `Cargo.lock`, justify each for *your* context.
- Minimize proc-macro deps; keep `syn`-pulling crates away from the dep root.
- Keep features off by default and additive; manage the graph + semver actively.
- Public deps of a stable crate must themselves be stable; prefer permissive
  licenses. *(Attune denies `openssl`; uses rustls-tls; replaced archived
  `serde_yaml` with `serde_norway`.)*
- Detect monomorphization bloat with `cargo llvm-lines`; profile builds with
  `-Z timings`.

## 7. Comments, consts, logging

- Comments explain **why**, not **what**; encode the "what" in names/types.
- Document every public item (rustdoc + `# Errors`/`# Panics`/`# Safety` + `?`
  examples).
- Replace magic numbers/strings with named `const`s (and newtypes for units).
- `tracing` spans/events; **log an error once at the handling boundary**, never
  re-log on propagation; keep logging out of pure logic.
- Run clippy + rustfmt in CI with `-D warnings`; gate format/lint/doc checks at
  merge.

## 8. Recurring "how seniors structure real Rust apps"

- Thin binary, fat library. Layered crates, tested per layer.
- Boundaries stable, internals free to churn. Pure core, imperative shell.
- Add/keep an `ARCHITECTURE.md`. Optimize build time early. Reach for abstraction
  last.

## Sources

matklad (large-rust-workspaces, how-to-test, why-not-rust, caches-in-rust,
inline-in-rust, fast-rust-builds, Rust100k) · Rust Book ch.7 · Effective Rust ·
BurntSushi error-handling · Luca Palmieri "Error Handling in Rust – A Deep Dive"
· Niko Matsakis · Jon Gjengset *Rust for Rustaceans*.
