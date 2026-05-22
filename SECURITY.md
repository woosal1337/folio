# Security policy

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security reports. Two private channels are available:

- **GitHub private vulnerability advisory** (preferred): <https://github.com/woosal1337/attune/security/advisories/new>
- Email: the maintainer's contact listed in the repository profile.

Include:

- A clear description of the vulnerability and its impact.
- Steps to reproduce, or a minimal proof of concept.
- The affected version (release tag or commit SHA).
- Your name / handle if you would like to be credited.

## Scope

In scope:

- The Attune desktop binary (Rust, Tauri, JS/TS).
- Build and release tooling under `.github/workflows/`.
- The Tauri command boundary and capability files in `src-tauri/capabilities/`.

Out of scope:

- The OpenAI API itself, or any third-party service Attune is configured to talk to.
- Third-party plugins or extensions distributed outside this repository.
- Issues that require physical access to an unlocked machine.
- Issues that require running an attacker-supplied binary.

## Response targets

- Acknowledgement within 72 hours.
- Triage and severity assessment within 7 days.
- Fix or mitigation plan within 14 days for high / critical issues.
- Public disclosure coordinated with the reporter; CVE assigned via GitHub when applicable.

## Supported versions

While Attune is in `0.x`, only the latest minor release receives security fixes. Once `1.0` ships, this policy will be updated with a longer support window.

## Privacy and data

Attune is local-first by design.

- No telemetry, analytics, or crash reporting is bundled.
- The only outbound network connection is to `https://api.openai.com`, and only when the user has pasted an OpenAI key in Settings and explicitly requested a transcription.
- API keys are stored in the user's settings file on the local machine. They are never logged.

## Hall of fame

Reporters are credited in `CHANGELOG.md` and the relevant GitHub advisory, unless they opt out.
