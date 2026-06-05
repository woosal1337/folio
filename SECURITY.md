# Security policy

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security reports. Two private channels are available:

- **GitHub private vulnerability advisory** (preferred): <https://github.com/woosal1337/folio/security/advisories/new>
- Email: the maintainer's contact listed in the repository profile.

Include:

- A clear description of the vulnerability and its impact.
- Steps to reproduce, or a minimal proof of concept.
- The affected version (release tag or commit SHA).
- Your name / handle if you would like to be credited.

## Scope

In scope:

- The Folio desktop binary (Rust, Tauri, JS/TS).
- Build and release tooling under `.github/workflows/`.
- The Tauri command boundary and capability files in `src-tauri/capabilities/`.

Out of scope:

- The OpenAI API itself, or any third-party service Folio is configured to talk to.
- Third-party plugins or extensions distributed outside this repository.
- Issues that require physical access to an unlocked machine.
- Issues that require running an attacker-supplied binary.

## Response targets

- Acknowledgement within 72 hours.
- Triage and severity assessment within 7 days.
- Fix or mitigation plan within 14 days for high / critical issues.
- Public disclosure coordinated with the reporter; CVE assigned via GitHub when applicable.

## Supported versions

For the `1.0.x` line, the latest patch release receives security fixes. When a new minor (`1.x.0`) ships, the previous minor receives security fixes for 90 days.

## Privacy and data

Folio is local-first by design.

- No telemetry, analytics, or crash reporting is bundled.
- Audio, transcripts, and notes never leave the machine on the default path.
- Outbound network connections happen only when the user opts in, and only to:
  - `https://api.openai.com` / `https://api.anthropic.com` — cloud transcription or note generation, only after the user configures that provider's key.
  - `https://huggingface.co` and `https://github.com` (k2-fsa/sherpa-onnx releases) — one-time download of the local Whisper and diarization models. The voice-activity-detection model is compiled into the binary and is never downloaded.
  - Any webhook URL the user configures in Settings.
- Privacy Mode (Settings → Privacy) blocks every outbound request except `localhost`, including the model downloads above.
- API keys are stored in the macOS Keychain, never in a plaintext settings file, and are never logged.

## Hall of fame

Reporters are credited in `CHANGELOG.md` and the relevant GitHub advisory, unless they opt out.
