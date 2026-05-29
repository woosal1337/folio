# 06 — Performance

From The Rust Performance Book, clippy lint groups, and matklad "Inline in Rust".
`CODE_STYLE.md` §7 is canon (hot/cold paths, product budgets). **Measure before
optimizing** — profile with DHAT/`--release`; ~10 allocs/M-instructions ≈ 1%.

## 1. Allocations & clones

- Pre-allocate with `with_capacity`/`reserve` when the size is known
  (`Vec`/`HashMap`/`HashSet`).
- Remove unnecessary `.clone()` on heap types; borrow instead.
- Prefer `&str` over `String` and `&[T]` over `Vec<T>` in params (clippy
  `ptr_arg`).
- Reuse a collection across loop iterations with `.clear()` (retains capacity).
- Read with `BufRead::read_line` into a reused `String`, not `lines()`.
- Use `Cow<'static, str>` for mixed static/dynamic strings; avoid `format!` when a
  literal or `write!` suffices.
- `SmallVec<[T; N]>` for many short-lived small vectors; box large/rare enum
  variants (clippy `large_enum_variant`).
- Use `Rc`/`Arc` only when values are actually shared often.

## 2. Iterators / zero-cost abstractions

- Don't `collect` then iterate; return `impl Iterator` / iterate lazily (clippy
  `needless_collect`).
- Use `extend(iter)` over collect-to-temp-then-append; `filter_map` over
  `filter().map()`; `chunks_exact` when length divides evenly.
- `.copied()`/`.cloned()` for small `Copy` values; impl `size_hint`/
  `ExactSizeIterator` on custom iterators so downstream preallocates.
- Avoid range-index loops where an iterator works (clippy `needless_range_loop`).

## 3. `#[inline]` (judiciously)

- Add `#[inline]` to small, **non-generic**, public library fns (and trivial
  `Deref`/`AsRef` impls) — downstream can't inline across the crate boundary
  otherwise.
- **Do NOT** add `#[inline]` to generic fns (already monomorphized downstream).
- In applications, add `#[inline]` reactively, after profiling; don't sprinkle it.
- Inlining is non-transitive; `#[inline(always)]`/`#[inline(never)]` only for
  measured hot/cold paths.

## 4. Build / release profile

- Always benchmark with `--release` (dev is 10–100× slower).
- For max runtime speed: `codegen-units = 1`, `lto = "fat"` (or `"thin"`),
  `panic = "abort"` if you don't use `catch_unwind`.
- `-C target-cpu=native` when you control the hardware (breaks portability).
- Speed up dev builds: `debug = "line-tables-only"` + a faster linker (`lld`/`mold`).
- Consider PGO for distributed binaries; `strip = "symbols"` when debuggability
  is non-critical.

## 5. Clippy as a gate

- Run `cargo clippy` in CI; the `correctness` group is deny-level. Treat `perf`
  warnings as actionable. Opt into `pedantic`/`nursery` selectively. *(Attune CI
  runs `clippy --workspace --all-targets -D warnings`.)*

## Sources

The Rust Performance Book (heap-allocations, iterators, inlining, build-
configuration) · Clippy lint index · matklad "Inline in Rust".
