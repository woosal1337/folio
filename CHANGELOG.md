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

[Unreleased]: https://github.com/woosal1337/folio/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/woosal1337/folio/releases/tag/v1.1.0
[1.0.0]: https://github.com/woosal1337/folio/releases/tag/v1.0.0
[0.0.1]: https://github.com/woosal1337/folio/releases/tag/v0.0.1
