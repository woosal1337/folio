# 08 — Unsafe Code

From the Rustonomicon, the Unsafe Code Guidelines, clippy, and "Safety Comments
Matter". `CODE_STYLE.md` §1.1 requires a `// SAFETY:` comment on every unsafe
block. Attune's only `unsafe` is FFI (cocoa/objc, sqlite-vec, libc pthread QoS)
plus marker `unsafe impl Send`.

## 1. When & how much

- Default to safe Rust; `unsafe` only unlocks five things: FFI, raw-pointer deref,
  mutable statics, unions, and unsafe-trait impls. Everything else stays safe.
- **Minimize and encapsulate** `unsafe` behind a sound safe abstraction — a
  library is *sound* iff no safe caller can trigger UB through its public API.
- Keep each `unsafe` block minimal: ideally one unsafe operation per block.

## 2. Invariants & UB

- Always keep data **valid** (validity invariant), enforced at every access; only
  safe code may assume the **safety** invariant. Unsafe code may temporarily break
  the safety invariant but must restore it before returning to safe code.
- Never produce invalid values (bad `bool`, out-of-range `char`/discriminant, null
  `fn` ptr, dangling reference, uninitialized integer).
- Never deref dangling/unaligned raw pointers; never read uninitialized memory.
- Never break aliasing (no simultaneous live `&mut` and `&` to the same data);
  never cause data races.
- At FFI boundaries match the ABI exactly and never let a foreign function unwind
  across the boundary.

## 3. Documentation (enforceable)

- **`// SAFETY:` comment directly above every `unsafe` block** stating why it's
  sound and which invariants hold (clippy `undocumented_unsafe_blocks`).
- **`/// # Safety` doc section on every `unsafe fn`** listing caller preconditions
  (clippy `missing_safety_doc`).
- State soundness, not assumptions; address each precondition as a bullet and
  reference the checks that guarantee it.
- Enable `unsafe_op_in_unsafe_fn` (default-warn in edition 2024) so each unsafe op
  inside an `unsafe fn` still needs its own `unsafe` block.

## Sources

Rustonomicon (intro, what-unsafe-does) · Unsafe Code Guidelines glossary (validity
vs safety, soundness) · clippy (`undocumented_unsafe_blocks`, `missing_safety_doc`)
· TheBestTvarynka "Safety Comments Matter".
