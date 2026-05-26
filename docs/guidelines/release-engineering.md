# Release engineering

How the signed, notarised, auto-updating Attune builds get produced.
Cited from `docs/CODE_STYLE.md` §9.5 and `docs/refactor/PHASE-3-PUNCH-LIST.md` D1/D3.

## One-line summary

`git tag v1.0.0 && git push --tags` triggers `.github/workflows/release.yml`. The workflow builds, signs, and notarises a macOS arm64 DMG, a macOS x86_64 DMG, and a Windows x86_64 MSI, then uploads them as a GitHub draft release alongside a Tauri-updater manifest (`latest.json`). The manifest's binaries are signed by the Tauri updater key, and the embedded pubkey in `tauri.conf.json` verifies them on the user's machine before applying the update.

## Required GitHub Actions secrets

Configure these in `Settings → Secrets and variables → Actions`. Names must match exactly.

| Secret | Purpose |
|---|---|
| `APPLE_CERTIFICATE` | Base64-encoded `Developer ID Application` `.p12` file. |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12`. |
| `APPLE_SIGNING_IDENTITY` | The signing identity name, e.g. `Developer ID Application: Ege Çelebi (TEAMID)`. |
| `APPLE_ID` | Apple ID for notarisation. |
| `APPLE_PASSWORD` | App-specific password for `notarytool`. |
| `APPLE_TEAM_ID` | Apple developer team id. |
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` certificate. |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the `.pfx`. |
| `TAURI_SIGNING_PRIVATE_KEY` | The Tauri updater private key (generated below). |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the updater key. |

## Generating the Tauri updater keypair

Run this once per major rotation. The private key stays in GitHub Secrets; the public key gets committed in `tauri.conf.json`.

```sh
# 1. Generate the keypair (writes to ~/.tauri/<name>.key and .key.pub)
bunx @tauri-apps/cli signer generate -w ~/.tauri/attune-updater

# 2. Copy the public key into tauri.conf.json
cat ~/.tauri/attune-updater.pub
# → replace the "pubkey" value under plugins.updater in src-tauri/tauri.conf.json

# 3. Copy the private key into GitHub Secrets as TAURI_SIGNING_PRIVATE_KEY
cat ~/.tauri/attune-updater
# → paste into the secret, including the password used during generate

# 4. Commit the pubkey change in src-tauri/tauri.conf.json. Push.
```

The pubkey is checked into source on purpose — it must travel with the binary so the update verifier can validate signatures offline. Treat the private key like an SSH host key: rotate every release cadence (annually at minimum), or on any suspicion of leak.

## Cutting a release

1. **Update `CHANGELOG.md`.** Move the `[Unreleased]` bullets under a new dated header (e.g. `## [1.0.1] — 2026-06-15`).
2. **Bump the version triplet** in `Cargo.toml` (workspace), `package.json`, `src-tauri/tauri.conf.json`. All three must match.
3. **Commit + push to `main`.**
4. **Tag.** `git tag v1.0.1 -m "v1.0.1: <one-line summary>" && git push --tags`. The `release.yml` workflow fires on the tag push.
5. **Wait for the draft release.** Three artifacts (macOS arm64 DMG, macOS x86_64 DMG, Windows x86_64 MSI) plus the `latest.json` manifest will appear on the GitHub Releases page as a draft.
6. **Smoke-test the artifacts** locally before publishing. Open the DMG, drag to Applications, launch, hit Cmd-R to record a test session, hit Stop, verify transcription completes.
7. **Publish.** Flip the draft to public. The Sparkle/Tauri-updater path picks up `latest.json` within an hour on every running install.

## Updater manifest format

The workflow writes `latest.json` to the GitHub release with the shape Tauri-updater expects:

```json
{
  "version": "1.0.1",
  "notes": "See CHANGELOG.md for the entries.",
  "pub_date": "2026-06-15T12:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<base64 ed25519 signature>",
      "url": "https://github.com/woosal1337/attune/releases/download/v1.0.1/Attune.app.tar.gz"
    },
    "darwin-x86_64": { … },
    "windows-x86_64": { … }
  }
}
```

The updater plugin downloads the URL, validates the signature against the pubkey embedded in `tauri.conf.json`, and applies the bundle.

## Key rotation procedure

1. Generate a fresh keypair (see the `signer generate` step above) with a new filename.
2. Open a PR that updates `pubkey` in `tauri.conf.json`. **Keep the old pubkey in the source history** for one full release cycle so users on the previous build can still verify the update that ships the rotation.
3. Update the GitHub Secret `TAURI_SIGNING_PRIVATE_KEY` with the new key after the PR merges to main.
4. Cut a release using the new key. Users update once; their next install uses the new key end-to-end.
5. After one release of overlap, retire the old key.

## Sparkle / non-Tauri-updater fallback

Not used. The `docs/distribution/README.md` doc mentioned Sparkle by analogy; the actual auto-update plumbing is the Tauri Updater plugin. Sparkle is reserved for a hypothetical native-Swift shell, not the current Tauri build.

## Mac App Store + Setapp builds

`docs/distribution/README.md` describes the dual-distribution plan. Both the MAS and Setapp SKUs build from the same workspace via Cargo feature flags (`mas`, `setapp`). Each channel has its own release pipeline; this doc covers the **direct DMG** channel only.

## Related

- `docs/CODE_STYLE.md` §11.1 — public-release hygiene checklist that this pipeline must satisfy.
- `docs/refactor/PHASE-3-PUNCH-LIST.md` D1, D2, D3, D11 — the audit items this doc closes.
- `docs/distribution/README.md` — the three-channel distribution strategy.
- `NOTICE` — third-party attribution file that ships in the bundle.
