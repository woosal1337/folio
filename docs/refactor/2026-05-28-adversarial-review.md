# Adversarial codebase review — 2026-05-28

Whole-codebase adversarial pass to reduce layers, remove complexity,
and increase reliability while keeping repo policies (docs/CODE_STYLE.md,
AGENTS.md) and original intent intact. Two parallel review agents
(architecture + reliability) plus a mechanical policy scan.

## Fixed in this pass (verified)

### Reliability — durable writes are now crash/power-loss safe

The `atomic_write` helper (temp + fsync + rename) existed but four of
the most important writers bypassed it with a bare `fs::write` or a
non-fsynced temp+rename. A crash or power loss mid-write could
truncate them. Routed all four through `atomic_write`:

- `transcription/mod.rs::write_json` — the transcript is the single
  most valuable artifact; its doc-comment _claimed_ atomicity but used
  `fs::write`. A corrupt transcript also let WAV-purge delete the
  source audio ("a transcript exists"), losing both. **Now atomic.**
- `storage/settings.rs::save` — a truncated settings.json silently
  resets the user's whole config to defaults on next load. **Now atomic.**
- `storage/tasks.rs::save` — **now atomic.**
- `memory/page.rs::write_page` — **now atomic.**
- `commands/transcription.rs` edited-transcript save — **now atomic.**

### Security — Privacy Mode (airgap) is now actually enforced on egress

§8.3 mandates every outbound request pass `cloud_guard::ensure_allowed`.
Two egress points bypassed it, so flipping Privacy Mode left the
AIRGAP badge lying while audio/text still shipped to OpenAI:

- `transcription/openai.rs` — guards the host before building the
  client; airgap returns a Privacy-Mode error instead of sending.
- `memory/embed.rs` — same guard, **plus** the embedder client had no
  timeout (a hung connection would wedge a blocking-pool thread
  forever); added 10s connect / 60s total timeouts mirroring the
  OpenAI transcriber.
- Added a synchronous egress regression test proving the embeddings
  URL resolves to `api.openai.com`, paired with the existing
  `airgap_blocks_external_hosts` host test (no process-global mutation,
  so no test-parallelism race).

### Reliability — flaky cloud_guard tests serialised

The cloud_guard tests mutate the process-global `AIRGAP` flag without
serialization, so under cargo's parallel runner they raced each other
(and surfaced as intermittent failures). Added a module-local
`AIRGAP_LOCK` mutex around the airgap-mutating tests. **49/49 pass in
parallel now.**

### Architecture — dead code + a §2.4 violation removed (from my own recent backend work)

- Deleted the unused `Envelope<T>` type in `backend/types.rs` (the
  client unwraps via a hand-rolled `serde_json::Value` pass; the
  struct was imported but never used) and its dead import.
- Deleted the unused `current_access` local + its `let _ =` discard in
  `backend/client.rs::refresh` (the convergence guarantee comes from
  the lock + refresh-token re-read, not the access token).
- ts-rs-derived the backend wire types (`UserDoc`, `DeviceDoc`,
  `ReferralStats`, `ReferralTokenResponse`) and deleted the
  hand-written TS interfaces in `ipc.ts` that duplicated them
  (§2.4 violation). Fixed a latent bug along the way: `UserDoc.id`
  serialized as `_id` (Mongo rename) but the hand-written type
  declared `id`, so `user.id` was undefined on the wire. Flipped the
  serde rename to serialize `id` while still accepting the server's
  `_id` via alias.
- Replaced the §1.1/§1.4 comment violations I introduced (an ASCII-art
  separator, a `//` item comment that should be `///`, a "Future:"
  comment that should be `TODO(owner)`).

## Deferred — higher blast radius, needs owner sign-off

These are real findings but touch real-time audio threads or change
13+ command signatures, so they warrant a deliberate decision rather
than an opportunistic edit.

### Reliability (high value, careful)

1. **Resampler allocates on every audio callback** (`audio/resampler.rs`
   `process`, plus the i16/u16 paths in `mic.rs`). Violates §7.1
   (no allocation on the hot path); the module doc even claims it
   doesn't allocate. Risk: audible dropouts under load. Fix: reusable
   scratch buffers allocated in `new()`. Needs perf validation on the
   real-time thread.
2. **Recording start/stop check-then-act race** (`commands/recording.rs`).
   The session lock isn't held across the `spawn_blocking(start)`
   await, so a double-fired hotkey can spawn two capture sessions and
   leak the first one's audio stream + stuck recording indicator. Fix:
   an atomic "starting" guard or a single tokio::Mutex held across the
   await.
3. **Egress log is O(n²)** (`storage/egress_log.rs`) — re-reads the
   whole file on every append to recompute the prev-line hash, and the
   log is uncapped. Fix: cache the running hash + add rotation.
4. **Model download has no overall timeout and no resume**
   (`transcription/models.rs`) — a mid-stream stall hangs forever; a
   dropped connection at 95% restarts from zero despite the module doc
   advertising Range-resume.
5. **Resampler tail not flushed on stop** (`mic.rs`/`system.rs` `stop`)
   — drops the final <1s of buffered audio; `flush()` exists and is
   used by the WAV decoder but not at capture teardown.

### Architecture (medium value)

6. **`BackendClient::new()` reconstructed per call** (13 command sites)
   — defeats reqwest connection pooling and gives each call its own
   refresh-lock (so the "don't double-refresh" guarantee is partly
   defeated). Fix: one shared `BackendClient` in `AppState` (it's
   `Clone` over an `Arc`). Internal-only — not a wire-contract change.
7. **`settings_sync` slice has no frontend consumer yet** — intentional
   future work (recordings-sync, task GET-140), not accidental dead
   code. Its command-layer `From<SettingsSnapshot>` mirror is pointless
   indirection once wired; collapse when the consumer lands.
8. **Missing IPC registry test** (§5.5/§9.4) — no test asserts every
   `ipc.ts` wrapper points at a real `generate_handler!` command.
   Add before executing #6 so command renames are caught mechanically.

## Policy reconciliation — needs a decision

CODE_STYLE §1.1 says "inline `//` comments inside function bodies are
not permitted" except `SAFETY` / `TODO(owner)` / `FIXME(owner)` /
`NOTE:`. **The codebase does not follow this** — there are hundreds of
untagged WHY comments inside bodies (e.g. `transcription/local.rs` has
~85). They are high-quality WHY comments — exactly what the rule's
_intent_ (kill WHAT comments, keep WHY) wants — they just lack the
literal `NOTE:` prefix.

Two honest options:

- **(A) Relax §1.1** to "WHY comments in bodies are permitted; WHAT
  comments are forbidden" — matches actual (good) practice, zero churn.
- **(B) Mechanical sweep** prefixing every body WHY comment with
  `// NOTE:` — ~500 edits of pure noise for literal compliance.

Recommendation: **(A)**. The intent is already honored everywhere; the
mechanism is stricter than the codebase (or the author) actually wants.
Awaiting owner decision before editing CODE_STYLE.md.
