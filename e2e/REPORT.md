# Attune e2e test report

**Generated:** 2026-05-28 (expanded coverage pass)
**Outcome:** **101 / 101 passing** (Playwright) · **104 / 104 passing** (Vitest)
**Wall-clock:** 47 s for the full Playwright run · 2 s for Vitest

---

## What's covered

The e2e suite drives the real React UI Tauri ships, in a real
Chromium against the Vite dev server (`http://localhost:5173`). The
Tauri IPC bridge is stubbed via `window.__TAURI_INTERNALS__.invoke`
so every backend call lands on an in-page handler with realistic
return shapes (see `e2e/fixtures/scenario.ts`).

| Spec file | Tests | Surface under test |
|---|---|---|
| `onboarding.spec.ts` | 3 | Permissions → signup → OTP → EventKit → workspace setup → I'm ready, returning-user shortcut, signed-out sidebar gate |
| `auth.spec.ts` | 3 | Sign out from Profile, re-sign-in skips workspace setup, auth_status hydrates at boot |
| `navigation.spec.ts` | 5 | Every sidebar route (Record / Inbox / Library / Tasks / Memory) renders |
| `sidebar.spec.ts` | 4 | Collapse button shrinks the sidebar, active route highlight via `aria-current`, Settings button opens modal, theme toggle button is in the footer |
| `command-palette.spec.ts` | 3 | Cmd-K opens palette, Cmd-Shift-/ opens cheatsheet, Escape closes |
| `settings-personal.spec.ts` | 6 | Preferences live-meeting toggle, 90-day GDPR default, Profile display name + email, Calendar toggle, Notifications toggle |
| `settings-recording.spec.ts` | 7 | Input device list, Transcription provider selection, Storage paths, Privacy section, Appearance, Save round-trip |
| `settings-workspace.spec.ts` | 10 | Workspace General name + discoverable toggle, Team empty state, Analytics no-scoring red line + range chips, Billing tier matrix, MCP copy-URL, Referrals copy, Webhooks / Usage tabs, IPC save fires |
| `settings-exhaustive.spec.ts` | 22 | Every persisted toggle / select / input across Preferences, Calendar, Notifications, Audio, Transcription, AI master + briefing language, Privacy, Storage, Workspace; round-trip drift check; freshSettings invariants |
| `recording.spec.ts` | 4 | Start affordance renders, `start_recording` IPC fires, `recording_status` boot probe, `list_recordings` populates the history strip |
| `library-tasks-memory.spec.ts` | 4 | Library row from `list_recordings`, Tasks create via inline composer, Memory seeded entries, Inbox empty state |
| `webhooks.spec.ts` | 2 | `list_webhooks` fires on tab open, seeded webhook renders with `label` + `url` |
| `agents.spec.ts` | 3 | `list_providers` fires on AI tab open, Inbox route mounts without IPC errors, provider stub shape |
| `referrals-flow.spec.ts` | 5 | Personal link renders, Copy writes share URL to clipboard, Email button generates `mailto:`, rules / steps render, no premature `referrals_me` call |
| `privacy-airgap.spec.ts` | 5 | `privacy_mode` toggle persists `true`, defaults `false`, `share_aggregate_stats` opt-in OFF, auto-delete defaults 90 days, link sharing defaults `workspace_only` |
| `cloud-cost.spec.ts` | 1 | No cloud-cost alert dialog on a clean boot |
| `integration-contract.spec.ts` | 5 | Boot probes (`auth_status`, `get_settings`), `recording_status` + `list_recordings` on Record mount, `save_settings` is canonical (not `settings_sync_push`), `auth_logout` fires exactly once |
| `backend-sync.spec.ts` | 5 | `save_settings` fires + carries patched payload, `auth_logout` flips the gate, boot probes (`auth_status` + `get_settings`), workspace-name round-trip |
| `audio-fixtures.spec.ts` | 3 | Voice manifest is well-formed, every MP3 fixture is non-empty + has valid magic, Chromium decodes the English business clip |
| **Total** | **101** | — |

---

## ElevenLabs voice fixtures

Generated **once** via `bun run e2e:fixtures`. Re-running the script
skips every file that already exists on disk — no extra ElevenLabs
spend on subsequent runs. The catalogue lives at the top of
`scripts/generate-voice-fixtures.mjs`; add new ids freely.

| File | Language | Context | Bytes | Voice |
|---|---|---|---|---|
| `en-business-1min.mp3` | en | business | 336 KB | Rachel |
| `en-action-items.mp3` | en | action_items | 265 KB | Domi |
| `en-clinical-consult.mp3` | en | clinical | 214 KB | Bella |
| `en-product-review.mp3` | en | product | 215 KB | Adam |
| `en-decision-record.mp3` | en | decision | 233 KB | Antoni |
| `tr-meeting.mp3` | tr | business | 160 KB | Rachel |
| `de-standup.mp3` | de | standup | 120 KB | Domi |
| `fr-product-pitch.mp3` | fr | pitch | 114 KB | Bella |
| `es-clinical-followup.mp3` | es | clinical | 156 KB | Adam |
| `ja-greeting.mp3` | ja | greeting | 91 KB | Antoni |

**Total cache footprint:** ~ 2 MB. Files are gitignored (per-dev
cache); the `manifest.json` companion describes each entry for
downstream tests to enumerate.

---

## Issues found while writing the suite (and what was done)

The first full run reported **13 failing tests / 47**. Each failure
turned into either a selector fix, a fixture-shape correction, or
— in one case — a real product bug.

### 1. Selector pattern — `.locator("..").getByRole("switch")` is fragile
**Symptom:** several toggle tests timed out (Preferences live-meeting
indicator, Calendar show-upcoming-meetings, Notifications scheduled,
Workspace discoverable).
**Cause:** the React `ToggleRow` component renders the title inside a
`<Label htmlFor={id}>` paired with a `<Switch id={id}>`, but my
locator climbed the DOM via `.locator("..")` to a different parent
than the switch sat under.
**Fix:** switch every toggle locator to
`getByRole("switch", { name: /title/i })`, which uses the
accessibility-name pairing the Label/Switch already establish.
**Files:** `e2e/settings-personal.spec.ts`, `e2e/settings-workspace.spec.ts`,
`e2e/backend-sync.spec.ts`.

### 2. Settings → Profile display-name input
**Symptom:** `getByLabel(/display name/i)` failed to find the input.
**Cause:** the Profile section's `FieldRow` renders the title as a
plain `<p>` (not as a `<Label htmlFor=…>`), so the input has no
accessible label association.
**Fix:** locate the input by its `placeholder` attribute
("Your name") instead.

### 3. Preferences → auto-delete GDPR default
**Symptom:** an XPath `ancestor::div` walk couldn't find the
`<combobox>` and the test timed out.
**Cause:** the `SelectRow` component uses `Label htmlFor=id` paired
with `<select id=id>` — `getByLabel` would have found it directly.
**Fix:** `expect(page.getByLabel(/auto-delete transcripts/i)).toHaveValue("90")`.

### 4. Settings → Transcription provider switch
**Symptom:** clicking `/openai whisper/i` text and then asserting an
"API key" label was visible failed — the API-key control is in a
different section.
**Cause:** I assumed an inline "API key" sub-control; in reality the
provider tile uses `aria-pressed` to indicate selection and the API
key lives under a different setting.
**Fix:** assert on `aria-pressed === "true"` after clicking the
"OpenAI Whisper API" tile.

### 5. Settings → General input-device picker
**Symptom:** asserting `getByText(/macbook pro microphone/i)` on the
General section came back empty.
**Cause:** the picker is a native `<select>` and the option text
doesn't render into the visible DOM until the dropdown is opened.
**Fix:** read the option text off the `<select>` element directly
via `locator.allTextContents()`.

### 6. Library row schema
**Symptom:** `expect(getByText("Product review")).toBeVisible()` failed
because the seeded `recordings` payload used field names like `title`
and `recorded_at` that don't exist on `RecordingSummary`.
**Cause:** I'd guessed at the shape. The real `RecordingSummary`
exposes `label`, `created_at`, `suggested_title`, etc., generated
from the Rust struct via ts-rs.
**Fix:** widen the stub interface to `Record<string, unknown>` in
`scenario.ts` and pass the real ts-rs-aligned shape per spec.

### 7. Memory list schema
**Symptom:** Memory page rendered but the seeded memory didn't appear,
which prevented the heading visibility check from settling in time.
**Cause:** seeded `kind: "Decision"` is not a valid `MemoryKind`. The
ts-rs enum is `"observe" | "claim" | "pref" | "person"`.
**Fix:** use `kind: "observe"` plus the full `Memory` shape (with
`valid_from`, `valid_until`, `confidence`, etc.).

### 8. Tasks composer is collapsed by default
**Symptom:** `getByPlaceholder(/what needs doing/i).fill(...)` timed
out because the textarea wasn't on screen.
**Cause:** the inline composer is hidden behind a `+ Add task` dashed
button per column; clicking the button reveals the textarea.
**Fix:** click `getByRole("button", { name: /^add task$/i }).first()`
to reveal the composer, then fill + press Enter.

### 9. Tasks IPC payload shape
**Symptom:** `calls[0].args.title` was `undefined`.
**Cause:** `createTask` ships its arg as `{ task: NewTask }` (Tauri
convention for the rich payload), not as the raw NewTask.
**Fix:** read `calls[0].args.task.title`. Also updated the scenario
stub to drain from the same wrapped shape.

### 10. **Real product bug** — Settings modal stays open after sign-out
**Symptom:** the "re-sign-in after sign out lands directly on main
app" test failed: after the user signed out, signed back in, the
Record heading was hidden behind the still-open Settings modal.
**Cause:** the Settings modal's open/close state lives in a global
Zustand store. The sign-out flow cleared the auth token + auth
identity but did **not** close the modal. App.tsx re-rendered into
the conductor branch (which unmounts the modal), but the
`useSettingsUiStore.open` flag remained `true`. After OTP verify
the auth-gate flipped back to the signed-in branch, the modal
re-mounted, and `open === true` made it pop up on top of the new
main app session.
**Fix (real code change):**
`section-profile.tsx::handleSignOut` now calls
`useSettingsUiStore.close()` after clearing the auth store.
**Why this matters:** without the e2e suite, a user signing out and
back in would see the Settings modal appear inexplicably over their
Record page on every re-sign. That's a sharp papercut that
unit-only testing would not have caught.

---

## How to run

```bash
# One-time setup (cached locally, ~2 MB):
bun run e2e:fixtures              # ElevenLabs voice clips

# Day-to-day:
bun run e2e                       # full suite, list reporter
bun run e2e:headed                # watch Chromium walk through
bun run e2e:ui                    # debugger
bun run e2e:report                # HTML report from the last run
```

The Vite dev server boots automatically via the `webServer` config
in `playwright.config.ts`. CI uses `--reporter=github` (already
wired). To force-regenerate a single fixture, delete it from
`e2e/fixtures/audio/` and re-run `bun run e2e:fixtures` — only the
deleted entry will hit ElevenLabs again.

---

## Coverage matrix (where each layer is tested)

| Layer | Suite | Tests |
|---|---|---|
| attune-api endpoints (Mongo + Redis live) | `attune-api/tests/integration/` | 18 |
| Rust core (VAD, calendar, attendee derivation, etc.) | `cargo test` | 440+ |
| **Real whisper.cpp transcription on real audio** | `cargo test --test transcription_fixtures` | 8 |
| React render tree in jsdom | Vitest + Testing Library | 104 |
| **Real React in Chromium** | **Playwright** | **101** |
| Real Tauri shell driving | blocked on `tauri-driver` macOS support | — |
| Real mic / system-audio capture | needs TCC + live device — manual | — |

## Real transcription tests (no mocking)

`crates/attune-core/tests/transcription_fixtures.rs` runs the
**actual** local Whisper pipeline — whisper-rs → whisper.cpp → the
cached `ggml-large-v3.bin` model — against the ElevenLabs voice
fixtures (transcoded to 16 kHz mono WAV by the fixture generator via
ffmpeg). This is genuine end-to-end transcription, the same code path
the app runs when you stop a recording. No IPC stub, no fake.

- The fast default test (`english_business_clip…`) runs on every
  `cargo test` — transcribes the English business clip and asserts
  ≥2 of {launch, marketing, migration, referral} survive. **Verified
  passing in 5.8 s (release).**
- The multilingual sweep (en-clinical, tr, de, fr, es, action-items)
  is `#[ignore]` because large-v3 is ~15-30 s per clip. Run with:
  ```bash
  bun run test:transcription
  # = cargo test -p attune-core --test transcription_fixtures -- --ignored --nocapture
  ```
- Tests **skip gracefully** (early return + eprintln) when the model
  or fixtures are absent, so CI without the 3 GB model stays green.
  Override the model path with `ATTUNE_WHISPER_MODEL`.

This closes the "can you actually test transcription?" gap: yes —
the engine is tested headlessly against real generated speech in 6
languages. What still can't be automated is *GUI-driving the native
window* (`tauri-driver` macOS gap) and *live mic capture* (needs a
real audio device + TCC grant).

## Expanded-coverage notes (round 2)

The expansion pass added **54 new tests** across 9 new spec files
(`settings-exhaustive`, `recording`, `agents`, `command-palette`,
`sidebar`, `privacy-airgap`, `referrals-flow`, `webhooks`,
`cloud-cost`, `integration-contract`). Key invariants the suite now
enforces:

- **Every persisted setting field round-trips** through
  `save_settings` with the exact value the user toggled.
- **Privacy defaults can't drift**: privacy_mode OFF, share_stats
  OFF, auto-delete 90 days, link sharing workspace_only.
- **No premature backend calls**: `referrals_me` doesn't fire
  before the section needs it; `settings_sync_push` doesn't fire
  on every Save (only `save_settings` does).
- **Boot probes are guaranteed**: `auth_status`, `get_settings`,
  `recording_status`, `list_recordings` all fire on cold load.
- **Shortcuts** (Cmd-K / Cmd-Shift-/) drive the actual chrome
  overlays the user reaches with keyboard.

Issues uncovered this round + fixed:
1. **Cheatsheet shortcut is Cmd-Shift-/**, not Cmd-/. Test corrected.
2. **`HomeRedirect`** sends users with recordings to `/library`,
   not `/record`. Tests must navigate explicitly to Record when
   asserting on that route.
3. **Sidebar collapse** has a 300ms CSS transition; tests must
   poll for the bounding box to settle.
4. **Webhook record shape** uses `label`, not `name`. Fixed in
   fixture.
5. **AI master toggle** label is "AI on every recording", not
   "Run AI agents after". Fixed.

When `tauri-driver` adds macOS support (Tauri 2 open issue), the
existing `e2e/` suite re-points at the real binary with a one-line
`playwright.config.ts` change.
