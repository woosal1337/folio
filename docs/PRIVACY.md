# Privacy

Folio is local-first. This document states exactly what stays on your
machine, what can leave it, and what controls you have. It describes the
shipped behaviour of the open-source build in this repository.

## What stays on your machine

By default, everything:

- **Audio.** System-audio and microphone recordings are written as WAV to
  your chosen output directory. They are never uploaded on the default path.
- **Transcripts and notes.** Generated locally and written as Markdown to
  your vault. They never leave the machine on the default path.
- **Diarization.** Speaker separation runs entirely on-device through
  sherpa-onnx. No audio is sent anywhere to label speakers.
- **Search index, tasks, memories.** Stored in a local SQLite database and
  local files.

## What can leave your machine, and only when you opt in

Folio makes outbound network requests only in these cases:

| Destination                     | When                                                                                                                                                                                         | What is sent                                             |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `huggingface.co` / `github.com` | First use of local Whisper / diarization (Whisper + the pyannote segmentation model come from Hugging Face; the WeSpeaker embedding model comes from the k2-fsa/sherpa-onnx GitHub releases) | An HTTP GET to download the model weights. No user data. |
| `api.openai.com`                | Only if you set an OpenAI key **and** choose cloud transcription or AI notes                                                                                                                 | The audio or text you are transcribing / summarising     |
| `api.anthropic.com`             | Only if you set an Anthropic key **and** use AI notes                                                                                                                                        | The text you are summarising                             |
| Your Folio Server (self-hosted) | Only if you choose the Remote server provider in Account, set an endpoint you control, and sign in                                                                                           | The recording's audio tracks; the transcript syncs back  |
| A webhook URL                   | Only if you configure one in Settings                                                                                                                                                        | The event payload you configured                         |

If you never configure a cloud key, never point Folio at a Folio Server, and
never set a webhook, the only outbound request Folio ever makes is the
one-time model download — and that too can be blocked (see Privacy Mode).

The Folio Server case differs from the cloud rows in one important way: the
destination is a machine you deploy and administer yourself
([`server/`](../server/README.md)), authenticated with your own account, and
the audio never touches a third party.

## Privacy Mode (air-gap)

Settings → Privacy enables Privacy Mode. When on, the egress guard
(`cloud_guard`) blocks **every** outbound request except `localhost` —
model downloads and uploads to your own Folio Server included. The app keeps
working end-to-end with Wi-Fi off, provided the models you need are already
downloaded.

## No telemetry

- No analytics, no crash reporting, no usage tracking SDKs are bundled.
- This is enforced in CI: `scripts/check-no-telemetry.sh` fails the build
  if an analytics or crash-report dependency enters the lock files.

## Credentials

API keys are stored in the macOS Keychain, never in a plaintext settings
file, and are never written to logs.

## Data retention

Folio does not delete your recordings or notes on its own unless you ask it
to. Settings → Preferences offers an auto-delete window for transcripts
(7 / 30 / 90 days / 1 year / off); 90 days is the recommended
data-minimisation default. Deleting a note removes the Markdown file and its
index entry. You can also delete the recording and vault directories
yourself at any time.

## You are the data controller

Folio runs on your machine and stores data you control. For any recording
that includes other people, **you** are responsible for compliance with the
recording-consent and data-protection laws that apply to you and the other
participants (for example GDPR for EU residents, and one-party vs all-party
consent rules that vary by US state and by country).

## Recording consent

Recording a conversation can be illegal without the consent of the people in
it. Laws differ widely:

- Many US states (for example CA, CT, FL, IL, MD, MA, MT, NH, OR, PA, WA)
  require **all-party** consent.
- Many countries require notifying or obtaining consent from participants.

When a call spans multiple jurisdictions, assume the strictest rule applies.
Folio gives you the tool; obtaining consent is your responsibility. Tell
participants before you record.

## Reporting a privacy or security issue

See [`SECURITY.md`](../SECURITY.md) for the private reporting channels.
