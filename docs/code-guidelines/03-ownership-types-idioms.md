# 03 — Ownership, Types, Idioms

From the Rust Book (ownership/lifetimes/interior mutability), Rust Design
Patterns, Effective Rust, pretzelhammer, Cliffle (typestate), matklad.

## 1. Ownership & moves

- Assignment of a non-`Copy` type is a **move**; the source becomes invalid.
- Rely on scope-based `Drop` (RAII); don't manually free.
- Return owned values to transfer ownership out; accept owned only when you must
  consume.
- Use `mem::take`/`mem::replace` to move a value out from behind `&mut` instead
  of cloning.

## 2. Borrow over clone

- **Borrow by default; `.clone` only when you need an independent copy.**
- **Don't `.clone` to silence the borrow checker** — restructure ownership, or
  use `Rc`/`Arc` if sharing is real.
- Clone is fine for prototypes/small `Copy`-like values where perf is irrelevant.
- Hold `&str`/`&[T]` and materialize owned data only at the ownership boundary;
  each `.to_string` is an allocation.

## 3. Function argument types

- Accept `&str` not `&String`; `&[T]` not `&Vec<T>`; `&T` not `&Box<T>`.
- Accept `impl AsRef<str>` / `impl AsRef<Path>` for max caller flexibility at zero
  cost.
- Use `impl Trait` in arg position for ergonomics, but **delegate to a non-generic
  inner fn** to limit monomorphization bloat (see §05).
- Add `T: ?Sized` when a generic should also accept slices/trait objects.

## 4. Returning: owned vs borrowed, `Cow`

- Return owned when you produce new data; return a borrow only when it lives in
  `self` and the lifetime allows.
- Return `Cow<'a, str>` when output is usually borrowed but sometimes owned.
- Prefer `impl Trait` return over boxing for a single unnameable type (iterator/
  closure); use `Box<dyn Trait>` only for heterogeneous returns.

## 5. Make invalid states unrepresentable

- Replace `bool`/int flags with enums (`print(Sides::Both, Color::Bw)`).
- Encode invariants in the type so illegal combinations don't compile.
- `Option<T>` for absence, `Result<T, E>` for fallibility; never sentinels
  (`-1`, `""`, null).
- Prefer enums-with-data over a struct of optional fields where only some combos
  are valid.

## 6. Newtype pattern

- Wrap in a single-field tuple struct for a _distinct_ type (not a `type` alias).
- Distinguish units/semantics (`Miles` vs `Kilometres`); provide explicit `From`.
- Use a newtype to impl a foreign trait on a foreign type (orphan-rule workaround).
- Validate invariants in the ctor and keep the field private.
- Accept the forwarding-boilerplate cost; add `#[repr(transparent)]` when layout
  matters.

## 7. Typestate & builder

- **Typestate:** encode runtime state as distinct types; transitions consume
  `self` and return the next state type, so invalid operations don't compile.
- **Builder:** for many optional fields / multi-step construction; each setter
  takes `mut self`, returns `Self`, with a terminal `build`.

## 8. Conversions

- Impl `From<T>`, never `Into<T>` manually; use `Into` in generic bounds, `From`
  in impls.
- Impl `TryFrom<T>` (→ `Result`) for fallible conversions; `TryInto` comes free.
- **Prefer `From`/`Into`/`TryFrom` over `as` casts** — `as` allows silent lossy
  conversions; use `as` only when you specifically want that cast.

## 9. Iterators & combinators

- Express transforms with `map`/`filter`/`fold`/`sum`/`collect` over C-style
  index loops (clearer, often faster — elided bounds checks).
- Keep explicit loops for large bodies, mid-loop early exits, or measured hot
  paths; `collect::<Result<_,_>>` handles fallible cases.
- `Option` is a 0-or-1 iterator: `.extend(Some(x))`/`.chain(...)`.
- Chain `.map`/`.and_then`/`.ok_or`/`.unwrap_or` instead of verbose
  `match`; `.as_ref` on `&Option`/`&Result` to transform without moving out.

## 10. Smart pointers & interior mutability

- Default to `&T`/`&mut T`; reach for owning pointers only when references can't
  express the ownership graph.
- `Box<T>` for single-owner heap / recursive types; `Rc<T>` single-threaded
  shared ownership; `Arc<T>` cross-thread (beware ref cycles).
- `&dyn`/`Box<dyn>` for runtime-determined types; generics for static dispatch.
- Interior mutability only when you can guarantee borrow rules the compiler can't
  prove: `Cell<T>` for `Copy` whole-swaps, `RefCell<T>` to borrow inner (panics
  on violation), `Rc<RefCell>` single-threaded shared-mut, `Arc<Mutex>`/`RwLock`
  cross-thread.

## 11. Lifetimes

- Annotate lifetimes on struct fields holding references; rely on the three
  elision rules, add explicit lifetimes only when they don't apply.
- `T: 'static` means "can live that long", not "lives for the whole program";
  owned types (`String`) satisfy it.
- `T: 'a` is broader than `&'a T`; a generic `T` already includes `&T`/`&mut T`.
- Compiling ≠ ideal lifetimes; don't blindly follow error-message fixes.
- Avoid gratuitously reborrowing `&mut T` as `&T`.

## 12. Derive hygiene & std traits

- Derive `Copy` only for small bit-copy-safe types (no owned resources); `Copy`
  requires `Clone`.
- Impl `IntoIterator`/`Iterator`/`FromIterator` for a coherent collection.
- Impl `Borrow<str>`/`Borrow<[T]>` to enable borrowed-key lookups.
- Rely on auto-derived `Send`/`Sync`; write `unsafe impl` only with proven
  reasoning (a `// SAFETY:` comment).

## 13. Anti-patterns

- **Don't use `Deref`/`DerefMut` to fake inheritance** — use composition +
  explicit delegation.
- **Don't `.clone` to satisfy the borrow checker.**
- **Don't reach for `Rc<RefCell<T>>` prematurely** — prefer plain ownership or
  `&`/`&mut` first; runtime borrow panics and ref-cycle leaks are the cost.
- **Don't micro-optimize speculatively** — measure first.

## Sources

Rust Book (ch.04/10/15) · Rust Design Patterns (idioms, newtype, builder, deref
& borrow_clone anti-patterns, option-iter, mem-replace) · pretzelhammer (lifetime
misconceptions, std-library-traits tour) · Effective Rust (use-types, transform,
casts, newtype, iterators, references) · Cliffle "Typestate" · matklad (why-not-
rust, fast-rust-builds).
