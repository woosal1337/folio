# Performance Standards

The fastest code is the code you don't write, the second-fastest is the
code that does its work once. Attune's perf model is straightforward:
allocate at startup, run at steady state, profile before optimising.

## Budget by layer

| Layer | Budget | Tolerated cost |
| --- | --- | --- |
| Audio callback (cpal / SCK) | < 1 ms per buffer | Zero allocation, zero syscalls |
| Tauri command handler | < 50 ms wall-clock | Some allocation, IO permitted |
| UI render | < 16 ms per frame | Whatever React does; avoid layout thrash |
| Startup (cold) | < 1 s to first paint | App + plugin init + theme apply |
| Settings open | < 100 ms | Devices enumerated, settings loaded |

These are guidelines. Hit them by default; investigate when something
overshoots.

## Profiling tools

- **CPU**: `samply` (sampling, low-overhead, works on macOS). `cargo
  flamegraph` for narrower investigations.
- **Allocation**: `dhat-rs` enabled behind a feature flag. Take
  measurements with the audio thread running, not idle.
- **Frontend**: WebKit's built-in profiler. Tauri exposes it via the
  webview inspector (Cmd+Opt+I in dev).
- **Startup**: timed by adding `tracing::info!("ready", ms = elapsed)`
  at the end of `run()`. Compare across commits.

Don't ship a performance fix you didn't profile. The cost-benefit
without measurement is guesswork.

## Allocation discipline

- Audio threads: zero. Pre-allocate at `start()`; reuse in callback.
- Per-buffer transient allocations move to a reused `Vec<f32>` carried
  by the callback closure (the i16/u16 paths in `mic.rs` are
  candidates).
- The `pending` buffer in `StreamingResampler` and `samples_written`
  counter use the right patterns. Future capture sources should
  follow them.
- React renders: don't allocate inside render functions unless the
  allocation depends on render. `useMemo` for arrays/objects fed to
  children, not for every literal object.

## Hot paths

- The mic capture path is hot at the cpal callback rate (typically
  ~100 callbacks/sec at 48 kHz with 480-frame buffers). Audit any
  change to `mic.rs::handle_samples`, `resampler.rs::process`, and
  `wav_writer.rs::append`.
- The system audio path is hot at SCK's audio thread rate (similar
  cadence). `system.rs::did_output_sample_buffer` and its helpers
  are equivalent hot paths.
- The recording status polling (UI side) runs every 250 ms; cheap by
  design — it just reads atomics under a mutex.

## Lazy work

- Device enumeration runs only when the Settings modal opens. It is
  not called on startup or per render.
- The recordings list is rebuilt on the explicit `refresh` action and
  when `lastSavedDir` changes, not continuously.
- `convertFileSrc` is memoised inside `<AudioPlayer>` with `useMemo`
  keyed on `filePath`. Avoids recomputing on every parent render.

## Render perf

- Lists are keyed by stable IDs (`session_dir`). Re-renders only
  reconcile the changed items.
- Heavy components (the future TipTap editor, the kanban) will be
  route-split with `React.lazy` + `Suspense`. Today the bundle is
  small enough that splitting is unnecessary; revisit when total JS
  > 250 KB gzipped.
- shadcn primitives are unmemoised. They are cheap; memoising every
  one would obscure the few we genuinely need.

## Build perf

- `[profile.dev.package."*"]` sets `opt-level = 1` so dependencies build
  with light optimisation in dev. This trades a one-time cold build for
  faster incremental compiles in audio code.
- `Swatinem/rust-cache@v2` caches build artefacts in CI; rebuilds
  should be sub-minute.
- The frontend bundle is small; Vite cold builds are fast. No
  optimisation needed.

## Startup

- Dock icon set at `setup` — cheap (a few KB image load).
- `tracing_subscriber` initialised in `init_tracing`. Compact formatter,
  env filter, no JSON.
- Settings load from disk does not block the UI; it runs in a Tauri
  command invoked on Settings modal open. Future: pre-load on app
  startup behind an idle callback.

## Size

- Release builds use `lto = "thin"`, `codegen-units = 1`,
  `strip = "symbols"`. Universal binary is ~30 MB unsigned (Rust core
  + Tauri runtime + bundled WebKit content).
- Frontend bundles tree-shake unused shadcn primitives because each
  is its own file.
- Audio model files (`models/*.bin`, `*.gguf`) are gitignored and
  downloaded at first run, not bundled.

## Anti-patterns

- Premature optimisation in cold paths (settings load, UI helpers).
  Spend the budget where the code runs often.
- "Just to be safe" caching. Caches add memory pressure and
  correctness bugs.
- Reaching for SIMD or AVX intrinsics in audio paths before the
  scalar code is profiled. `rubato` already SIMDs internally; we
  don't have to.

## Performance review checklist

Before merging a change that touches a hot path:

- [ ] No new heap allocations in the audio callback.
- [ ] No new lock contention. New mutexes are justified or replaced
      with atomics.
- [ ] No new syscalls in the callback path (`std::fs::*`, `println!`,
      `tracing` log emission inside the inner loop).
- [ ] If the change is "optimisation", attach before/after numbers.
- [ ] If the change adds an allocation in a hot path, attach the
      reason and the measured impact.
