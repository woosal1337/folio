# 04 — Async & Concurrency

From Tokio docs/tutorial, the async-book, Alice Ryhl, parking_lot docs, the Rust
Book "Fearless Concurrency", and senior posts on deadlocks. `CODE_STYLE.md` §6
is canon (`parking_lot::Mutex` for short sync sections; `tokio::sync::Mutex` only
across `.await`; the canonical Tauri command shape clones state before `.await`).

## 1. Never block the executor

- Async code must reach an `.await` quickly — rule of thumb ≤10–100µs between
  await points. Never `std::thread::sleep` in async; use `tokio::time::sleep`.
- Route blocking/CPU work off the executor:
  - **Blocking IO** → `tokio::task::spawn_blocking`.
  - **Heavy CPU** → `rayon` (cores-sized pool) + a `oneshot` to return.
  - **Long-running / indefinite loops** → a dedicated `std::thread`, **not**
    `spawn_blocking` (which would permanently occupy a pool thread).
- `spawn_blocking` tasks **cannot be aborted** once started; runtime shutdown
  waits for them (use `shutdown_timeout`).

## 2. Never hold a sync lock guard across `.await`

- Never hold a `std`/`parking_lot` `MutexGuard`/`RwLockGuard` across `.await`.
  `std::sync::MutexGuard` is `!Send` so it won't compile when spawned; **parking_lot/
  dashmap guards ARE `Send`, so they silently compile and then deadlock** — be
  extra careful.
- **Drop the guard with an explicit scope** before awaiting; `drop(guard)` does
  *not* fix the `Send` analysis (it's scope-based).
- Wrap `Arc<Mutex<T>>` in a struct exposing only **non-async** methods that lock
  and return, structurally guaranteeing the guard never crosses `.await`.

## 3. Choosing the lock

- **Default to a sync mutex** (`parking_lot::Mutex` in Attune) for short critical
  sections not held across `.await`.
- **`tokio::sync::Mutex` only when you must hold the lock across `.await`** (it's
  slower); first try to restructure so the section has no `.await`.
- `tokio::sync::Mutex::blocking_lock()` panics in async context.
- `RwLock` only for clearly read-heavy data (watch for writer starvation).
- **parking_lot vs std:** parking_lot is smaller/faster, no poisoning (`.lock()`
  returns the guard directly), guards are `!Send`; std guards are `Send` and
  poison on panic.
- **Atomics for a single flag/counter** (`AtomicBool`/`AtomicUsize`): `Relaxed`
  for independent flags, `Acquire`/`Release` to publish/consume guarded data,
  `SeqCst` when unsure.

## 4. Message passing / actors

- When a resource is IO-heavy or you'd share it via a lock held across `.await`,
  give one task exclusive ownership and talk to it over a channel (actor).
- Use **bounded** channels for backpressure; the actor shuts down when all
  senders drop (`recv()` → `None`).
- Channels: `mpsc` (many→one), `oneshot` (single response), `broadcast` (fan-out),
  `watch` (latest value / config).

## 5. Deadlock avoidance

- Never acquire a second lock while holding the first; if unavoidable, enforce one
  global lock ordering everywhere.
- Never re-acquire a `tokio::sync::Mutex` you already hold on the same task.
- Don't `block_on(...)` while holding a lock.
- Keep critical sections minimal (lock, mutate, drop; IO/awaits outside). Shard
  hot maps to cut contention.

## 6. `select!` and cancellation

- Assume every non-winning `select!` branch's future is **dropped** mid-flight;
  only put cancellation-safe ops in looping branches.
  - Safe: `recv()`, `accept()`, `read`/`read_buf`, `write`/`write_buf`, `next()`.
  - **Not** safe (data loss): `read_exact`, `read_to_end`, `write_all`, lock/
    semaphore acquisition.
- To resume a future across iterations, `tokio::pin!` it and select on `&mut`.
- Use `biased;` only when you deliberately want top-to-bottom poll order.

## 7. Don't silently detach `JoinHandle`s

- Dropping a `JoinHandle` **detaches** the task (keeps running, unobservable);
  it does **not** cancel it — call `handle.abort()` to stop.
- Await the handle and inspect `JoinError` (`is_panic()`) to surface panics in
  spawned tasks.

## 8. `Send + Sync`

- Anything `tokio::spawn`'d on the multi-thread runtime must be `Send + 'static`;
  reference-shared data must be `Sync`.
- Share owned data cross-thread with `Arc`, never `Rc`; use `move` closures to
  transfer ownership into tasks.

## 9. Runtime config & shutdown

- Name runtime/worker threads (and any dedicated thread) for legible panics/
  profiles; `enable_all()` for IO+timer drivers.
- Pick the flavor: `current_thread` for non-`Send`/one-at-a-time work,
  `multi_thread` when background tasks must keep progressing.
- **Graceful shutdown = detect (signal) + notify (`CancellationToken`) + wait
  (`TaskTracker`).** For a single dedicated thread, an `AtomicBool` checked each
  iteration suffices.

## 10. Attune / Tauri-FFI notes

- Long-lived native work (audio capture, transcription pumps, EventKit polling)
  runs on a **dedicated `std::thread`**, not `spawn_blocking`.
- Bridge that thread to async via an `mpsc` channel (`blocking_send` from the FFI
  side, `recv().await` on the async side) — keeps the FFI boundary off the
  executor.
- Never `tokio::sync::Mutex::blocking_lock()` inside an async command; never hold
  a sync guard across a command's `.await`.
- Give worker/FFI threads a name + a clean stop signal (`AtomicBool`/token) and
  `join()` them on teardown so native resources release deterministically.

## Sources

Tokio (bridging, shared-state, shutdown, `spawn_blocking`, `select!`, `JoinHandle`,
`Mutex`, `RwLock`, `runtime::Builder`) · async-book (concurrency primitives) ·
Alice Ryhl (what-is-blocking, actors-with-tokio, shared-mutable-state) · Rust Book
"Fearless Concurrency" · parking_lot docs · `std::sync::atomic` · Turso/Medium
deadlock posts.
