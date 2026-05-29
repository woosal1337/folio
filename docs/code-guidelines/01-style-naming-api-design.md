# 01 — Style, Naming, API Design

Checkable rules from the Rust API Guidelines (`C-*` items), the Rust Style Guide
(rustfmt defaults), RFC 430, and senior authors. `CODE_STYLE.md` §1–§2 is canon;
this expands it.

## 1. Naming (RFC 430 / C-CASE)

- **Casing:** `UpperCamelCase` for types/traits/enum-variants/type-params;
  `snake_case` for modules/functions/methods/vars/macros; `SCREAMING_SNAKE_CASE`
  for statics/consts; short lowercase lifetimes (`'a`, `'src`).
- Treat acronyms as one word: `Uuid` not `UUID`, `HttpClient` not `HTTPClient`.
- Type params are concise (`T`, `K`, `V`); crate names carry no `-rs`/`-rust` suffix.
- **Conversions encode cost + ownership (C-CONV):** `as_` = free borrow→borrow view;
  `to_` = expensive, borrow→owned; `into_` = consumes owned→owned. e.g.
  `as_bytes`, `to_owned`, `into_bytes`. Keep `mut` ordering: `as_mut_slice`.
- **Getters omit `get_` (C-GETTER):** `fn first(&self) -> &T`, not `get_first`.
  Reserve `get`/`get_mut` for fallible indexed access returning `Option`.
- **Iterators (C-ITER / C-ITER-TY):** `iter`/`iter_mut`/`into_iter`; the produced
  type name matches the method (`Iter`, `IterMut`, `IntoIter`, `Keys`).
- **Cargo features have no placeholder words (C-FEATURE):** `serde`, not `with-serde`.
- **Consistent word order (C-WORD-ORDER):** verb-object-error like `ParseIntError`.
- **Constructors (C-CTOR):** primary is `Type::new()`; evocative verbs for
  domain ctors (`File::open`, `TcpStream::connect`); secondary config via
  `_with_*`; `from_*` for encoding/disambiguating ctors.

## 2. Formatting (rustfmt defaults — Rust Style Guide)

- 4-space indent, never tabs; **max line width 100**; no trailing whitespace.
- Block indent over visual indent; trailing commas on multi-line lists.
- ≤1 blank line between items; K&R braces (open brace on the same line).
- `use`/`mod` before other items; version-sort names within an import group,
  don't merge groups; `self`/`super` sort first, globs last.
- Multi-line fn signatures break after `(`, one arg per line, trailing comma.
- Method chains break *before* `.`; binary operators break *before* the operator;
  both get surrounding spaces. Ranges have no inner spaces (`0..10`).
- Always specify ABI: `extern "C" fn`, never bare `extern fn`.
- Just run `cargo fmt`; do not hand-format.

## 3. API design

- **Conversions use `From`/`TryFrom`/`AsRef`/`AsMut`; never impl `Into` directly
  (C-CONV-TRAITS)** — the blanket impl gives `Into` for free.
- **Put conversions on the most specific type (C-CONV-SPECIFIC).**
- **Operations with a clear receiver are methods, not free functions (C-METHOD).**
- **No out-parameters (C-NO-OUT):** return tuples/structs, not writes through
  `&mut` params (except reusable caller buffers like `Read::read`).
- **Expose intermediate results (C-INTERMEDIATE):** e.g. return the insertion
  index on a failed binary search.
- **Let the caller control allocation (C-CALLER-CONTROL):** borrow unless you
  consume; don't borrow-then-clone internally.
- **Use builders for complex construction (C-BUILDER);** prefer non-consuming
  (`&mut self` setters) so conditional config reads cleanly.
- **Smart pointers add no inherent methods (C-SMART-PTR)** — `Box::into_raw(b)`,
  not `b.into_raw()`. **Only smart pointers impl `Deref` (C-DEREF).**
- **Operator overloads must be unsurprising (C-OVERLOAD).**
- **Provide both `new()` and `Default`** with identical behavior when applicable.
- **Prefer `Result` over `Option`** when you can describe the failure.

## 4. Trait design

- **Eagerly impl common traits (C-COMMON-TRAITS):** `Copy, Clone, Eq, PartialEq,
  Ord, PartialOrd, Hash, Debug, Display, Default` — the orphan rule blocks
  downstream from adding them later.
- Impl `FromIterator`/`Extend` on collections (C-COLLECT); `Serialize`/
  `Deserialize` behind an optional `serde` feature (C-SERDE).
- Make types `Send`/`Sync` where possible (C-SEND-SYNC); verify with
  `fn assert_send<T: Send>()` helpers for raw-pointer types.
- Design for object-safety when useful as `dyn Trait` (C-OBJECT); exclude
  generic methods with `where Self: Sized`.
- Generic reader/writer params take `R: Read`/`W: Write` by value (C-RW-VALUE).

## 5. Generics & bounds

- **Minimize assumptions (C-GENERIC):** accept `impl IntoIterator<Item = T>` /
  `impl AsRef<Path>` over concrete `&[T]`/`&Path` when you only need the capability
  (trade-off: more codegen — see §05 on outlining).
- **Accept borrowed slice/str types:** `&str` over `&String`, `&[T]` over `&Vec<T>`.
- Add `T: ?Sized` to generic params used behind a pointer.
- **Validate, preferring static over dynamic (C-VALIDATE):** newtype invariant >
  runtime check > `debug_assert!` > unchecked opt-out.
- Don't duplicate derived bounds on the struct (C-STRUCT-BOUNDS): write
  `#[derive(Clone)] struct S<T>`, not `struct S<T: Clone>`.

## 6. Documentation

- Thorough crate-root docs with runnable examples (C-CRATE-DOC); every public
  item has an example (C-EXAMPLE); examples use `?`, never `unwrap` (C-QUESTION-MARK).
- **Document `# Errors`, `# Panics`, `# Safety` (C-FAILURE):**
  - `# Errors` on every fallible fn: which variants, under which conditions.
  - `# Panics` on any fn that can panic (e.g. out-of-bounds).
  - `# Safety` on every `unsafe fn`: the invariants the caller must uphold.
- Hyperlink prose with intra-doc links (C-LINK); fill Cargo.toml metadata
  (C-METADATA); keep a CHANGELOG and tag releases (C-RELNOTES).
- Hide noise with `#[doc(hidden)]` (C-HIDDEN); write/keep an `ARCHITECTURE.md`.

## 7. Public-API hygiene

- **Seal traits not meant for downstream impl (C-SEALED).**
- **Keep struct fields private (C-STRUCT-PRIVATE)** except passive C-style data.
- Wrap complex return types in a newtype (C-NEWTYPE-HIDE) so internals can change.
- Mark future-extensible structs/enums `#[non_exhaustive]`.
- Annotate `#[must_use]` on `Result`-like returns, pure computations, builders.
- All public types impl `Debug`, and `Debug` output is never empty (C-DEBUG-NONEMPTY).
- 1.0 crates' public deps must themselves be stable (C-STABLE).
- **Error hygiene:** error types impl `std::error::Error`, are `Send + Sync +
  'static`; never `()`/`String` as the error; `Display` is lowercase, no trailing
  punctuation; destructors never fail/block (provide explicit `close()`).
- **Type-safety:** newtypes for unit distinctions (`Miles(f64)`); meaning via
  types not `bool`/`Option` args (C-CUSTOM-TYPE); `bitflags` for flag sets.

## Sources

Rust API Guidelines (checklist, naming, interoperability, predictability,
type-safety, future-proofing, documentation, flexibility, dependability,
debuggability, necessities, macros) · Rust Style Guide (index, items, expressions)
· RFC 430 · Rust Design Patterns (coercion-arguments) · matklad ARCHITECTURE.md ·
BurntSushi "Error Handling in Rust" · pretzelhammer (lifetime misconceptions,
sizedness) · blessed.rs · `#[must_use]` / `#[non_exhaustive]` reference.
