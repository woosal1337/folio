# macOS code signing & notarization

How Folio is signed so that, when another user installs it, the app is a
notarized Developer ID build that opens with no Gatekeeper warning and keeps
its microphone / screen-recording permissions across updates.

## What the build already does

`release.yml` builds on macOS runners and hands signing + notarization to
`tauri-apps/tauri-action`. The bundler:

- signs the app **inside-out** (nested code first, then the outer `.app`) with
  the Developer ID identity — this is the supported replacement for the
  deprecated `codesign --deep`;
- enables **Hardened Runtime** (`bundle.macOS.hardenedRuntime` is `true`);
- applies `src-tauri/Entitlements.plist`;
- submits to Apple's notary service and staples the ticket.

The ONNX Runtime used by diarization (`sherpa-onnx`) and VAD (`ort`) is
**statically linked**, so there is no `libonnxruntime.dylib` to sign and no
`disable-library-validation` entitlement is needed. The only nested binary is
the `folio-mcp` sidecar (`bundle.externalBin`).

## Entitlements (`src-tauri/Entitlements.plist`)

Minimal and correct for a non-sandboxed Developer ID app that records audio and
runs a WKWebView:

- `com.apple.security.cs.allow-jit` — WKWebView JITs JavaScript.
- `com.apple.security.device.audio-input` — the microphone entitlement for a
  notarized, non-sandboxed, Hardened-Runtime app.

Deliberately **absent**: `com.apple.security.app-sandbox` (App Store only),
`com.apple.security.device.microphone` (sandbox variant), and
`disable-library-validation` (would weaken Gatekeeper and isn't needed).
ScreenCaptureKit is gated at runtime by TCC via the `NS*UsageDescription`
strings in `Info.plist`, not by an entitlement.

## One-time setup: certificate + GitHub secrets

1. **Apple Developer Program** membership ($99/yr), Account Holder role.
2. Certificates → **Developer ID Application** (not "Apple Distribution").
   Create the CSR from Keychain Access → Certificate Assistant, upload it,
   download the `.cer`, and import it into your **login** keychain.
3. Keychain Access → My Certificates → expand the cert → **Export** as `.p12`
   with a password.
4. Base64-encode it: `openssl base64 -A -in certificate.p12 -out cert.txt`.
5. appleid.apple.com → Sign-In and Security → **App-Specific Passwords** →
   generate one for notarization.

Populate these repository secrets (already wired into `release.yml`):

| Secret                       | Contents                                                                                         |
| ---------------------------- | ------------------------------------------------------------------------------------------------ |
| `APPLE_CERTIFICATE`          | base64 of the `.p12` (the `cert.txt` above)                                                      |
| `APPLE_CERTIFICATE_PASSWORD` | the `.p12` export password                                                                       |
| `APPLE_SIGNING_IDENTITY`     | `Developer ID Application: Your Name (TEAMID)` — from `security find-identity -v -p codesigning` |
| `APPLE_ID`                   | Apple Developer account email                                                                    |
| `APPLE_PASSWORD`             | the **app-specific** password (not your Apple ID password)                                       |
| `APPLE_TEAM_ID`              | 10-character Team ID                                                                             |

With those set, tag a release (`v1.0.0`) and the pipeline signs + notarizes.

## Verifying a build is correctly signed

Run against the built bundle / DMG:

```sh
APP="src-tauri/target/release/bundle/macos/Folio.app"
DMG="src-tauri/target/release/bundle/dmg/Folio_1.0.0_aarch64.dmg"

codesign --verify --deep --strict --verbose=2 "$APP"          # valid on disk
codesign --display --verbose=4 "$APP" 2>&1 | grep -E 'Authority|TeamIdentifier|flags'
#   Authority=Developer ID Application: <Name> (TEAMID)
#   flags=0x10000(runtime)

codesign --display --verbose=2 "$APP/Contents/MacOS/folio-mcp" 2>&1 | grep flags
#   flags=0x10000(runtime)   <-- sidecar must carry hardened runtime

spctl --assess --type execute --verbose=2 "$APP"              # accepted
#   source=Notarized Developer ID
xcrun stapler validate "$APP"                                 # ticket stapled
xcrun stapler validate "$DMG"
otool -L "$APP/Contents/MacOS/folio" | grep -i onnx || echo "no onnx dylib (static, as expected)"
```

The two phrases that prove success are **`accepted`** and
**`source=Notarized Developer ID`**.

## Sidecar caveat

Tauri issue [#11992](https://github.com/tauri-apps/tauri/issues/11992) reports
notarization occasionally failing when an `externalBin` sidecar is not signed
with `--options runtime`. Folio ships one sidecar (`folio-mcp`). After the first
release, confirm it with the `folio-mcp` verification line above. If it lacks
the runtime flag, pre-sign it in `release.yml` after the "Build folio-mcp
sidecar" step (requires importing the cert into the runner keychain first):

```sh
codesign --force --options runtime --timestamp \
  --sign "$APPLE_SIGNING_IDENTITY" \
  "src-tauri/binaries/folio-mcp-<target-triple>"
```

## Permissions on first launch

Because the app is signed with a stable identity (`dev.folio.app`) and ships the
`NS*UsageDescription` strings plus `CFBundleDisplayName = "Folio"`, the first-run
TCC prompts show **"Folio"** with the app icon, and the grants persist across
updates (TCC binds permissions to the code signature). Unsigned local builds get
a generic name/icon and may lose grants on rebuild — always test permission flows
against a signed build.
