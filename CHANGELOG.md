# Changelog

All notable changes to Folio are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Folio follows
[Semantic Versioning](https://semver.org).

<!--
  ───────────────────────────────────────────────────────────────────────────
  RELEASE TEMPLATE — copy the block below under ## [Unreleased] for a new
  version, fill in the date, and keep only the headings you actually use.

  Write for the person installing the release, not for the diff. Each bullet
  leads with a bolded human-readable claim, then one sentence of why it
  matters. Past tense, no issue numbers in the line itself.

  Heading order is fixed: Added, Changed, Deprecated, Removed, Fixed, Security.

  ## [X.Y.Z] — YYYY-MM-DD

  One-sentence theme for the release.

  ### Added
  - **Short claim.** What it does and why you'd want it.

  ### Changed
  - **Short claim.** What changed and what you'll notice.

  ### Deprecated
  - **Short claim.** What is going away, and what replaces it.

  ### Removed
  - **Short claim.** What was taken out and why.

  ### Fixed
  - **Short claim.** The symptom that is now gone.

  ### Security
  - **Short claim.** The exposure that is now closed.

  Then add a link reference at the bottom of the file:
  [X.Y.Z]: https://github.com/woosal1337/folio/releases/tag/vX.Y.Z
  ───────────────────────────────────────────────────────────────────────────
-->

## [Unreleased]

_Nothing yet._

## [2.0.0] — 2026-08-01

A deliberate stripping-back: Folio keeps the parts that record, transcribe, and
read well, and drops everything that never worked.

### Added

- **Large recordings upload again.** Audio now uploads to your server in 8 MiB
  chunks instead of one enormous request, so files past ~100 MB no longer bounce
  off Cloudflare's request-size limit. An interrupted upload resumes from where
  the server left off rather than starting over.

### Changed

- **Home is the only place your notes live.** Home and My Notes showed the same
  list on two screens; they are now one. Home keeps its Today / Yesterday /
  Earlier grouping and gains search, transcript and sort filters, and per-row
  actions. Every row shows both its transcription state and its sync state.
- **Settings is smaller and honest.** Every remaining toggle now does something.
  Roughly thirty controls that saved a value nothing ever read have been removed.
- **Preferences opens in the app.** ⌘, now opens the in-app Settings dialog
  instead of a second, separate window that drifted out of sync with it.

### Removed

- **Tasks and Memory tabs.** Extracted tasks and memories are still captured and
  still feed Analytics and the editor's participant cards; the standalone
  browsing screens are gone.
- **Spaces / folders.** The sidebar section, the editor's folder chip, folder
  filtering, the chat folder scope, and the `notes_by_folder` MCP tool.
- **Meeting auto-detection.** Folio no longer watches for conferencing apps or
  offers a "take notes?" popup when a call starts. Start recordings yourself.
- **Notifications settings.** Folio never sent a system notification; the tab
  configured behaviour that did not exist.
- **Integrations and Webhooks.** The connector cards were placeholders, and no
  webhook was ever fired for any event.
- **Anthropic and DeepSeek providers.** Both returned "support arrives in phase
  2" on every call. OpenAI is the supported provider.
- **Background sync loop.** Recordings still upload automatically when you stop
  them; the extra timer that re-synced every thirty seconds is gone.

### Fixed

- **Uploads no longer race themselves.** The background sync timer could start a
  second upload of a recording while the first was still running, so the same
  file was sent two or three times over. Only one sync per recording now runs at
  a time.
- **Sidebar footer type size.** Account, Settings, and the theme toggle render at
  the same size as the rest of the sidebar instead of Account being larger.

## [1.2.0] — 2026-07-23

Remote GPU transcription: point Folio at a server you own and let it do the
heavy lifting while your Mac sleeps. Local transcription stays the default.

### Added

- **Remote server transcription (opt-in).** Pick "Remote server" as the
  provider and recordings upload to a self-hosted Folio Server, transcribe on
  its GPU with faster-whisper, and the transcript syncs straight back into
  your vault. Uploads are resumable, size-capped, and integrity-checked, and
  everything is scoped to your account on that server.
- **A self-hostable backend lives in `server/`.** Deploy it with one click on
  Coolify (Docker Compose resource, base directory `server/`) or with
  `./deploy.sh` on any Docker host — GPU by default, CPU stack included. See
  `server/README.md`.
- **Account tab.** A new sidebar destination for your server connection:
  endpoint with a live connection test (engine, model, GPU), sign in / create
  account, auto-upload, and a one-click way to make the server your default
  transcriber.
- **Sync status everywhere.** Uploading / Queued / On GPU / Synced / Sync
  failed badges on Home, My Notes, and the note page, with stage-aware
  progress and a Try again action when a sync fails.

### Changed

- **Stopping a recording lands you on the note.** Folio navigates to the
  finished note and the page updates live through upload → transcription →
  synced, instead of leaving a stale empty view behind.
- **Status chips use sentence case** (Transcribed, Synced, On GPU, …) across
  the app.

### Fixed

- **Share / export no longer crashes the app.** The macOS share sheet was
  being presented off the main thread, which took the whole app down on
  current macOS.
- **Remote sessions no longer expire mid-day.** Access tokens refresh
  automatically instead of failing with "invalid token" after 30 minutes.
- **Silence no longer yields phantom captions.** The server runs
  voice-activity detection before transcribing, so a silent track produces an
  empty transcript instead of hallucinated text.

### Security

- **The server refuses to boot in production with the default JWT secret**,
  and registration can be locked once your accounts exist
  (`FOLIO_ALLOW_REGISTRATION=false`).

## [1.1.0] — 2026-06-09

Reliability pass for macOS 26 (Tahoe). Removes the macOS Calendar integration
and hardens the remaining system-permission paths so a changed or denied macOS
API can never crash the app.

### Removed

- **macOS Calendar integration.** Folio no longer reads your calendar through
  EventKit. Removed with it: the Settings → Calendar pane, the onboarding
  calendar step, the Home "Coming up" panel, the meeting-HUD attendee brief,
  the Calendar permission, the "scheduled meeting" notification, and the Apple
  Calendar connector card. macOS 26 changed the EventKit access API in a way
  that made the feature unreliable; the internal plumbing is kept dormant so it
  can return cleanly in a later release.

### Changed

- **Menu-bar status is a single dot.** Gray when idle, red while recording,
  amber when paused — identical size and position in every state, replacing the
  earlier mix of an audio-bars glyph and a duplicate marker in the title.

### Fixed

- **The "Whisper model not downloaded" toast now gets you there.** It carries
  an Open Settings action that jumps straight to Settings → Transcription so you
  can start the download in one click.
- **Onboarding system-audio permission opens the right pane.** It now checks,
  prompts, and opens macOS Screen Recording consistently, instead of sometimes
  opening the System Audio pane on macOS 26.
- **Apple Reminders access can no longer crash Folio.** EventKit requests run
  through a modern Objective-C bridge and the current-macOS access API, and any
  Objective-C exception is caught instead of aborting the process.

## [1.0.0] — 2026-06-08

First public release. Local-first meeting transcription for macOS: captures
system audio and microphone as independent streams, transcribes on-device, and
writes a markdown file per meeting to your vault. Audio never leaves your
machine on the default path.

### Added

- **Homebrew install.** `Casks/folio.rb` (this repo doubles as its own tap)
  plus an `update-homebrew-cask` workflow that pins the version and DMG checksum
  on every published release. `brew tap woosal1337/folio https://github.com/woosal1337/folio && brew install --cask folio`.
- **`folio-mcp` crate.** A local MCP stdio server exposing read-only access to
  transcripts, tasks, and memories to MCP-aware clients (Claude Desktop, Cursor,
  Claude Code), bundled into release builds as a sidecar so Settings →
  Connectors works out of the box.
- **On-device speaker diarization** on the system-audio track
  (pyannote-segmentation-3.0 + WeSpeaker embedding via sherpa-onnx); the
  microphone is always labelled **You**.
- **Settings → Analytics** computes real on-device activity totals (meetings,
  minutes, action items, decisions, memories) over a selectable date range.
- **Settings → Calendar** lists upcoming events from macOS Calendar (EventKit)
  when access is granted. _(Removed in 1.1.0.)_
- **Silent-microphone detection** — the recording bar warns when a backend
  captures near-silent audio.

### Changed

- AI auto-agents, diarization, and live-transcription toggles stay disabled
  (with a hint) until their model is downloaded or an API key is configured, so
  a toggle is never "on" while its dependency is missing.
- Error toasts strip the internal `ipc … failed:` plumbing and surface
  plain-language messages.

### Security

- Capability split per window class, strict CSP, narrowed asset-protocol scope,
  and the OpenAI key moved from on-disk settings to the macOS Keychain.

## [0.0.1] — 2026-05-20

Project initialized. See [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) for
the architecture.

[Unreleased]: https://github.com/woosal1337/folio/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/woosal1337/folio/releases/tag/v2.0.0
[1.2.0]: https://github.com/woosal1337/folio/releases/tag/v1.2.0
[1.1.0]: https://github.com/woosal1337/folio/releases/tag/v1.1.0
[1.0.0]: https://github.com/woosal1337/folio/releases/tag/v1.0.0
[0.0.1]: https://github.com/woosal1337/folio/releases/tag/v0.0.1
