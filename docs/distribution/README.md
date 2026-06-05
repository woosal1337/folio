# Distribution channels

Folio ships through three channels. The direct DMG is the canonical
release; the Mac App Store and Setapp builds are reduced-feature SKUs
of the same Rust core..

| Channel                 | Status           | Audience                                       | Feature delta                                                                                                                                             |
| ----------------------- | ---------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Direct DMG with Sparkle | Primary, day one | Power users, devs, BYO-key crowd               | Everything. Local Whisper, OpenAI Whisper, agents, webhooks, MCP, deep links, file associations, accessibility services.                                  |
| Mac App Store (MAS)     | Targeted v1.1    | Mainstream macOS users finding us on the Store | Sandboxed. No deep links to other apps. No MCP server (network listen). No Accessibility-services hooks. Same audio capture + transcription + AI surface. |
| Setapp                  | Targeted Q4      | Already-paying Setapp subscribers              | Same as direct DMG. Setapp handles licensing; we ship the same notarised binary with a Setapp entitlement.                                                |

## Build variants

The Cargo workspace exposes three feature flags on `folio-app`:

| Cargo feature | Channel       | Effect                                                                                                                    |
| ------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `direct`      | Direct DMG    | Default. Every native integration available.                                                                              |
| `mas`         | Mac App Store | Disables MCP server module, deep-link write capability, Accessibility-services hooks. Forces the App Sandbox entitlement. |
| `setapp`      | Setapp        | Switches the licence-check call site to Setapp's framework instead of the in-app keystore. No other behavioural change.   |

These flags are exclusive — CI builds one binary per channel from the
same workspace.

## Release pipeline

1. `gh workflow run release.yml -f channel=direct` builds the
   notarised, stapled DMG and uploads it to the Sparkle appcast on
   `folio.app/releases/`.
2. `gh workflow run release.yml -f channel=mas` builds the
   sandboxed `.pkg` and uploads it via `xcrun altool` to App Store
   Connect.
3. `gh workflow run release.yml -f channel=setapp` builds the same
   notarised binary signed with the Setapp distribution profile and
   uploads it via Setapp's vendor portal.

Every channel pipeline runs the same gates: `cargo deny check`,
`cargo audit`, `npm audit --omit=dev`, CodeQL, `cargo nextest run`,
`bun run typecheck`, `bun run lint`, `bun run test`. Channel-specific
gates layer on top: notarisation for direct + Setapp, App Store
Review validation for MAS.

## Updater + signing

- Direct DMG: Sparkle 2 with EdDSA signatures. The pubkey lives in
  the bundle's `Info.plist` and is rotated via the standard Sparkle
  key-rotation flow. The appcast feed URL is hard-coded into the
  build.
- MAS: macOS auto-update via App Store; the Sparkle path is
  disabled at compile time when the `mas` feature is on.
- Setapp: Setapp handles updates; the Sparkle path is also disabled
  on Setapp builds because parallel updaters create UX confusion.

## Feature gating

`#[cfg(feature = "mas")]` gates the MAS deltas in Rust. On the
TypeScript side a runtime flag `__FOLIO_CHANNEL__` (injected by Vite
during the build) lets the UI hide the MCP server / Accessibility
panels in the MAS SKU. The single source of truth for which channels
hide which features is `docs/distribution/feature-matrix.md` (a
follow-up doc that pairs with the actual SKU launch).

## What this PR ships

This PR ships the policy doc and the canonical channel table. Wiring
the Cargo features into `folio-app/Cargo.toml`, splitting the Sparkle
appcast, and registering the MAS bundle identifier are individual
follow-up PRs as each channel approaches launch. The Linear backlog
will gain `-mas`, `-setapp`, and `-feature-matrix`
issues at that point.
