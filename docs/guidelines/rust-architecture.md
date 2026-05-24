# Rust workspace & module architecture — Attune guidelines

Source-cited synthesis for organising the Attune workspace, drawing module boundaries, and shaping public APIs. Targets the multi-crate workspace (`attune-core`, `attune-cli`, `attune-app`) and is intended to be re-read whenever a new module, crate, or public type is added.

## TL;DR — the rules

1. **Flat workspace, virtual root.** All crates under `crates/`, root `Cargo.toml` is `[workspace]`-only (no `[package]`). ([matklad](https://matklad.github.io/2021/08/22/large-rust-workspaces.html))
2. **Inherit metadata via `[workspace.package]` and deps via `[workspace.dependencies]`.** Single source of truth for version/edition/license; one build of each external dep. ([Cargo Book](https://doc.rust-lang.org/cargo/reference/workspaces.html))
3. **Internal crates use `version = "0.0.0"`.** Signal that they're not publishable; promote to a real semver only if extracted to `libs/`.
4. **`<name>.rs + <name>/` over `<name>/mod.rs`.** Compiler enforces no mixing. Editor tabs read `capture.rs`, not seventeen `mod.rs`. ([Rust Reference](https://doc.rust-lang.org/reference/items/modules.html))
5. **Modules are units of privacy, not filesystem mirrors.** Each module = one cohesive responsibility + a narrow public face. ([Aaron Turon](http://aturon.github.io/tech/2017/07/26/revisiting-rusts-modules/))
6. **Visibility ladder defaults to most-restrictive that works.** Private > `pub(super)` > `pub(crate)` > `pub`. Visibility expansion is forever; restriction is impossible without breaking callers. ([Effective Rust Item 22](https://effective-rust.com/visibility.html))
7. **Run the [Rust API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html) against every public surface** before tagging a release. The list is short; bookmark it.

## Workspace layout

Current Attune layout (good):

```
attune/
  Cargo.toml                # virtual manifest: [workspace] + [workspace.dependencies] + [workspace.package]
  crates/
    attune-core/            # framework-agnostic library
    attune-cli/             # CLI test harness, depends on attune-core
  src-tauri/                # Tauri shell, package name "attune-app"
  src/                      # React frontend
```

Improvements to consider:

- Promote `src-tauri/` into `crates/attune-shell/` so the workspace is fully flat. Tauri tooling reads `tauri.conf.json` and doesn't care about the directory name — only `bun tauri dev`/`build` looks at the convention. Test on a branch before adopting.
- Add an `xtask/` crate for repository automation (model downloads, codegen, release packaging). Replaces shell scripts with type-checked Rust everyone on the team can already read. ([matklad](https://matklad.github.io/2021/08/22/large-rust-workspaces.html))

## When to split a crate vs add a module

Crate-splitting is not free. It slows `cargo check`, complicates visibility, and `mv` is no longer enough to reorganise. Only split when **one** of these is true:

1. You need **a binary target** (CLI, daemon, xtask, plugin).
2. The boundary has a **stable, narrow API** that the compiler should enforce across the line.
3. Two pieces have **disjoint dependency closures** and splitting cuts compile time meaningfully for the smaller side.
4. You want **independent feature flags** (e.g., a future `attune-transcribe` with `metal`, `cuda`, `cpu` variants).
5. The boundary is **testable in isolation** with mocks; the seam is real.

Otherwise: add a module. Modules are the cheap-to-revert tool.

## Module organisation

### File naming

- Always `<name>.rs + <name>/`. Never `<name>/mod.rs`.
- The `<name>.rs` file is the module's table of contents: `mod` declarations and `pub use` re-exports for the child modules. Real code lives in the children.

### Boundary heuristics

A sub-module appears when:

- A single file passes ~400 lines _and_ contains more than one cohesive concept; or
- A private state / lifecycle emerges that the parent doesn't need to see; or
- A type and its impl block + helpers form a self-contained mini-API.

A new module does _not_ appear because a file is "long but coherent" — `hallucination_filter.rs` is 405 lines and is one concept (filter rules + a single `is_hallucination` function); splitting would just disperse the data.

### Visibility ladder

| Modifier       | Use when                                                                                             |
| -------------- | ---------------------------------------------------------------------------------------------------- |
| (private)      | Item is module-local. Majority of declarations.                                                      |
| `pub(super)`   | Item is shared with parent's sibling modules only. Useful in deep hierarchies.                       |
| `pub(crate)`   | Item is internal but used across modules in this crate. **The default for "I need this elsewhere."** |
| `pub(in path)` | Surgical exposure to one specific subtree. Rare but exists.                                          |
| `pub`          | Item is part of the crate's external API. Justify every one.                                         |

From [Effective Rust Item 22](https://effective-rust.com/visibility.html):

> "Visibility changes can be hard to undo. Once a crate item is public, it can't be made private again without breaking any code that uses the crate."

Practical Attune rule: Tauri command modules in `src-tauri/src/commands/` must be `pub` (Tauri's `generate_handler!` requires it); the `attune-core` helpers they call should be `pub(crate)` _unless_ `attune-cli` also needs them. Re-export the chosen API from `lib.rs`.

### `pub use` facade pattern

Hide implementation paths behind re-exports from the crate root:

```rust
// crates/attune-core/src/lib.rs
pub mod audio;          // public surface declared inline below
pub mod transcription;
pub mod storage;
pub mod llm;

pub use audio::{CaptureSession, CaptureArtifacts, CaptureConfig, Channel};
pub use error::{AttuneError, Result};
```

Consumers `use attune_core::CaptureSession`, not `attune_core::audio::capture::CaptureSession`. You can rename, move, or split `audio::capture` later without breaking anyone.

## The Rust API Guidelines — rules most likely to bite

Full checklist at <https://rust-lang.github.io/api-guidelines/checklist.html>. The ones that surface most often in real code:

### Naming

- **C-CASE** — `snake_case` fns/modules, `UpperCamelCase` types, `SCREAMING_SNAKE_CASE` consts.
- **C-CONV** — Ad-hoc conversions follow `as_`/`to_`/`into_`. `as_` is free/borrowing, `to_` is expensive, `into_` consumes.
- **C-GETTER** — Getter names omit `get_`. `fn sample_rate(&self) -> u32`, not `get_sample_rate`.
- **C-ITER** — Iterators are `iter`/`iter_mut`/`into_iter`.
- **C-WORD-ORDER** — Pick one verb-noun ordering and stick to it across the API.

### Interoperability

- **C-COMMON-TRAITS** — Derive `Copy/Clone/Eq/PartialEq/Ord/PartialOrd/Hash/Debug/Display/Default` where applicable. Users get angry when `Config` lacks `Debug`.
- **C-SEND-SYNC** — Types are `Send + Sync` where possible. Critical for Tauri `State` and `tokio::spawn`.
- **C-GOOD-ERR** — Error types are meaningful and well-behaved. See [`rust-error-handling.md`](./rust-error-handling.md).
- **C-SERDE** — Data structures implement `Serialize`/`Deserialize` _behind a feature gate_ so non-serializing consumers don't pay.

### Predictability

- **C-CTOR** — Constructors are inherent methods (`Foo::new(...)`), not free functions (`fn new_foo()`).
- **C-NO-OUT** — No out-parameters. Return a tuple/struct.

### Type safety

- **C-NEWTYPE** — Newtypes for static distinctions. `pub struct SampleRate(u32)` beats raw `u32` at API boundaries.
- **C-CUSTOM-TYPE** — Arguments convey meaning through types, not `bool` or `Option`. `fn save(force: bool, dry_run: bool)` is a bug waiting to happen — extract `SaveMode { Force, DryRun, Normal }`.
- **C-BUILDER** — Builders for complex values.

### Future-proofing

- **C-SEALED** — Sealed traits when downstream impls aren't intended. (See [sealed traits](#sealed-traits) below.)
- **C-STRUCT-PRIVATE** — Public structs have private fields. Adding a field later is otherwise a breaking change.
- **C-NEWTYPE-HIDE** — `pub struct SampleRate(u32)`, not `pub struct SampleRate(pub u32)`. Inner is implementation detail.
- **C-STRUCT-BOUNDS** — Bounds on impls, not struct defs. `struct Foo<T>` + `impl<T: Debug> Foo<T>`, never `struct Foo<T: Debug>`.

### Documentation & metadata

- **C-CRATE-DOC** — Crate-level docs with examples. The `//!` block at the top of `lib.rs`.
- **C-EXAMPLE** — Every public item has at least one rustdoc example.
- **C-FAILURE** — `# Errors` and `# Panics` sections under every `Result`-returning or panic-prone fn.
- **C-METADATA** — `Cargo.toml` has description, license, repository, keywords, categories. Inherit via `[workspace.package]`.
- **C-HIDDEN** — `#[doc(hidden)]` on macro-generated impls and items technically public but conceptually internal.

### Dependability

- **C-VALIDATE** — Functions validate their arguments at the boundary, not in private helpers.
- **C-DTOR-FAIL** — Destructors never fail. Always call `.finalize()` / `.close()` explicitly and propagate the error.
- **C-DTOR-BLOCK** — Destructors that may block have an alternative explicit path (relevant for `AudioWavWriter`, `CaptureSession`).

## Hexagonal / ports & adapters, Rust-flavored

Reference: [Master Hexagonal Architecture in Rust (howtocodeit.com)](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust). The shape:

- `domain/` — pure types, trait _ports_, domain errors. Zero infra deps.
- `inbound/` — anything that _drives_ the domain (Tauri commands, CLI subcommands, HTTP routes).
- `outbound/` — anything the domain _uses_ (whisper.cpp, cpal/CoreAudio, ScreenCaptureKit, the filesystem).

Ports are traits with concrete adapters as structs:

```rust
pub trait Transcriber: Send + Sync + 'static {
    fn transcribe(&self, audio: &Path, lang: Option<&str>) -> Result<Transcript, TranscribeError>;
}

pub struct LocalWhisper { /* whisper_rs handle, model path, … */ }
impl Transcriber for LocalWhisper { /* … */ }

pub struct OpenAiWhisper { /* reqwest client, API key, … */ }
impl Transcriber for OpenAiWhisper { /* … */ }
```

The `Send + Sync + 'static` bounds aren't optional — they're what makes the trait usable from `tokio::spawn` and `tauri::State`.

**Wrap external libraries, never let them leak.** If `whisper_rs::WhisperContext` appears in a `pub fn` signature in `attune-core`, you've lost. The whole point of the adapter is to absorb upstream churn.

**Domain entities ≠ request DTOs.** `CreateTranscriptRequest` (what Tauri commands take), `Transcript` (the domain value), `TranscriptRow` (the SQLite/JSON shape) should be three distinct types, even if they currently have the same fields. Coupling them all into one struct is the single biggest source of churn in Rust apps at scale.

**`main` (and Tauri's `setup`) is dependency wiring only.** Construct adapters, hand them to `tauri::Builder::manage(...)`, mount commands. No business logic.

## Sealed traits

From [the definitive guide](https://predr.ag/blog/definitive-guide-to-sealed-traits-in-rust/) — a trait is sealed when it cannot be implemented outside its own crate. The standard recipe:

```rust
mod private {
    pub trait Sealed {}
}

pub trait Backend: private::Sealed {
    fn name(&self) -> &str;
}

impl private::Sealed for LocalWhisper {}
impl Backend for LocalWhisper { /* … */ }

impl private::Sealed for OpenAiWhisper {}
impl Backend for OpenAiWhisper { /* … */ }
```

**Critical:** never `pub use private::Sealed`. Doing so defeats the seal.

Use sealing when:

- You'll add methods later without bumping major version, or
- The trait enumerates a closed set of valid implementations (supported backends).

## Newtypes at API boundaries

Per **C-NEWTYPE-HIDE** and the [Rust patterns book](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html):

```rust
pub struct SampleRate(u32);     // good
pub struct SampleRate(pub u32); // bad — inner is part of public API
```

Expose access via methods (`fn hz(&self) -> u32`), conversion (`TryFrom<u32>`), or `Deref` only when the inner is a smart pointer.

Newtypes belong at every "untrusted in, trusted out" boundary: Tauri commands → domain, CLI args → domain, on-disk JSON → domain. Once the data is past the boundary, the type system carries the invariant.

## `#[non_exhaustive]`

- Apply to **public** enums and structs to prevent additive changes from being a SemVer break. ([RFC 2008](https://rust-lang.github.io/rfcs/2008-non-exhaustive.html))
- Skip on crate-private types — no SemVer concerns there.
- Most-common use: error enums.

```rust
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum AudioError { /* … */ }
```

Downside (per [Knoldus](https://medium.com/@knoldus/prevent-breaking-code-changes-in-future-releases-using-non-exhaustive-enums-in-rust-ace1ac4650d9)): "may lead to awkward code and code paths only executed in rare circumstances." Worth it for public APIs; overkill for internal.

## `#[doc(hidden)]`

For items that must be technically public (macro-generated impls, cross-crate workspace internals) but conceptually internal. Doesn't change visibility — only hides from rustdoc. Pair with module-level `//!` "this is internal" notes.

## Attune-specific recommendations (priority order)

1. **Adopt the `<name>.rs + <name>/` pattern** for all modules. Current code already does this except where files are short enough to live inline.
2. **Mark public enums `#[non_exhaustive]`.** Top candidate: `AttuneError` in `crates/attune-core/src/error.rs`.
3. **Tighten visibility** — sweep `attune-core` for `pub` items that only one other module uses, demote to `pub(crate)`. Each demotion frees you to refactor.
4. **Split `attune-cli/src/main.rs` (600 lines) into per-subcommand modules** under `crates/attune-cli/src/commands/`. The main fn becomes the dispatch table.
5. **Split `voice_processing_capture.rs` (470 lines)** — the smoke-test type used only by the `vpio-smoke` CLI subcommand and the production `VoiceProcessingMicCapture` are two different concerns sharing a file. Move them into a `voice_processing_capture/` directory.
6. **Consider hexagonal layering** (`domain/inbound/outbound`) as the next big refactor. Current layout (`audio/transcription/storage/llm`) is responsibility-based but mixes ports with adapters. Splitting trait definitions from concrete implementations is a 1-week project, not a single-PR change.
7. **Run the API Guidelines checklist** against `attune-core`'s public surface once before tagging a release. Track gaps as issues.

## Sources

- [Rust API Guidelines — Checklist](https://rust-lang.github.io/api-guidelines/checklist.html)
- [matklad — "Large Rust Workspaces"](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)
- [The Cargo Book — Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [The Rust Reference — Modules](https://doc.rust-lang.org/reference/items/modules.html)
- [Effective Rust — Item 22: Minimize Visibility](https://effective-rust.com/visibility.html)
- [Kobzol — Two ways of interpreting visibility in Rust](https://kobzol.github.io/rust/2025/04/23/two-ways-of-interpreting-visibility-in-rust.html)
- [Aaron Turon — Revisiting Rust's modules](http://aturon.github.io/tech/2017/07/26/revisiting-rusts-modules/)
- [Master Hexagonal Architecture in Rust (howtocodeit.com)](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust)
- [LukeMathWalker — Zero to Production in Rust (repo)](https://github.com/LukeMathWalker/zero-to-production)
- [Predrag Gruevski — A definitive guide to sealed traits in Rust](https://predr.ag/blog/definitive-guide-to-sealed-traits-in-rust/)
- [Refined Types in Rust: Parse, Don't Validate (Entropic Drift)](https://entropicdrift.com/blog/refined-types-parse-dont-validate/)
- [Rust Design Patterns — Newtype](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)
- [Tauri 2 — Project Structure](https://v2.tauri.app/start/project-structure/)
- [RFC 2008: non_exhaustive](https://rust-lang.github.io/rfcs/2008-non-exhaustive.html)
