# Remote transcription server

Folio is local-first: by default audio never leaves the machine and Whisper
runs on-device. This document describes the **optional** remote backend that
lets a user offload transcription to a self-hosted, GPU-accelerated server so
the Mac can sleep or be closed while a meeting is transcribed elsewhere.

Remote mode is **opt-in**. Local and OpenAI transcription remain available and
local stays the default.

## Goals

- Upload a finished recording to a server the user controls; transcribe it on a
  GPU; sync the transcript back into the local vault.
- The Mac does not stay hot or awake for the length of the job — it uploads,
  then the work happens remotely and is pulled back later.
- Trivial to self-host: `docker compose up` on a GPU box.
- Accounts (login / register) and a user-configurable endpoint, laying the
  groundwork for sharing transcriptions with others.

## Topology

```
┌───────────────┐        HTTPS / bearer JWT        ┌─────────────────────────┐
│  Folio (Mac)  │ ───────────────────────────────▶ │  FastAPI API            │
│               │  create → upload → enqueue        │  (auth, uploads, jobs)  │
│  sync engine  │ ◀─────────────────────────────── │                         │
│  sync.json    │   poll job → pull transcript      └───────────┬─────────────┘
└───────────────┘                                               │ claims jobs
                                                                ▼
                                          ┌──────────────┐  ┌──────────────────┐
                                          │  DB          │  │  GPU worker       │
                                          │ (SQLite/PG)  │  │  faster-whisper   │
                                          └──────────────┘  └──────────────────┘
                                          ┌──────────────────────────────────┐
                                          │  object store (local FS / S3)     │
                                          └──────────────────────────────────┘
```

The API and the worker are separate processes that communicate only through the
database (job queue) and the object store. There is no Redis or external broker
— a single GPU node is served by one worker polling the `jobs` table. Postgres
or multiple workers scale this out later without changing the contract.

## Transcript compatibility (hard requirement)

The server must return transcripts that deserialize, unchanged, into the Rust
`SessionTranscript` written to disk by the client
(`crates/folio-core/src/transcription/mod.rs`):

```jsonc
{
  "channels": [
    {
      "channel": "mic", // or "system"
      "language": "en", // nullable
      "segments": [
        {
          "start_seconds": 0.0,
          "end_seconds": 1.24,
          "text": "hello",
          "speaker": null, // Option<i32>, filled by diarization
          "language": "en", // nullable
        },
      ],
    },
  ],
}
```

The client persists this via `SessionTranscript::write_json` (zstd
`transcript.json.zst`). The server produces per-channel segments; the client
assembles and stores them. VAD timestamp remapping and local diarization happen
client-side after the pull, exactly as they do for the local path.

## REST contract (`/v1`)

| Method | Path                                  | Purpose                                    |
| ------ | ------------------------------------- | ------------------------------------------ |
| GET    | `/health`                             | liveness                                   |
| GET    | `/v1/capabilities`                    | version, engine, model, GPU, diarization   |
| POST   | `/v1/auth/register`                   | create account → tokens                    |
| POST   | `/v1/auth/login`                      | password → access + refresh tokens         |
| POST   | `/v1/auth/refresh`                    | refresh → new access token                 |
| GET    | `/v1/auth/me`                         | current user                               |
| POST   | `/v1/recordings`                      | create/upsert a recording by client UUID   |
| GET    | `/v1/recordings`                      | list (supports `updated_since` for delta)  |
| GET    | `/v1/recordings/{id}`                 | one recording + channel/job state          |
| DELETE | `/v1/recordings/{id}`                 | delete recording + artifacts               |
| PUT    | `/v1/recordings/{id}/channels/{name}` | resumable chunked upload of `mic`/`system` |
| POST   | `/v1/recordings/{id}/transcribe`      | enqueue a transcription job                |
| GET    | `/v1/jobs/{id}`                       | poll status / progress / error             |
| GET    | `/v1/recordings/{id}/transcript`      | fetch the `SessionTranscript`              |

### Resumable channel upload

`PUT /v1/recordings/{id}/channels/{name}` uses simple offset semantics so an
interrupted upload (Mac sleep, flaky link) resumes rather than restarts:

- `Upload-Offset: <n>` — byte offset this chunk begins at; must equal the
  server's current stored size, else `409` with the true offset.
- body — raw bytes for this chunk.
- `Upload-Complete: true` on the final chunk, with `X-Content-Sha256: <hex>` for
  integrity; the server verifies and marks the channel complete.
- response — `{ "offset": <new size>, "complete": <bool> }`.

This mirrors the client's existing chunk state machine in
`crates/folio-core/src/transcription/upload_state.rs`.

## Sync lifecycle (client side)

Each recording session directory gets a durable UUID and a `sync.json` sidecar
(distinct from the existing `upload-state.json`), written atomically:

```jsonc
{
  "schema_version": 1,
  "recording_id": "<client uuid>",
  "remote_recording_id": "<server id | null>",
  "remote_job_id": "<server job id | null>",
  "upload_state": "pending | uploading | complete",
  "remote_status": "none | queued | running | succeeded | failed",
  "last_synced_at": "<rfc3339 | null>",
  "error": "<string | null>",
}
```

The reconcile loop is idempotent and restart-safe (all state lives in
`sync.json`):

1. `pending` → create recording, upload each channel (resumable), mark
   `complete`.
2. enqueue transcribe → store `remote_job_id`, `remote_status = queued`.
3. poll job until `succeeded`/`failed`.
4. on `succeeded` → pull transcript, write `transcript.json.zst`, set
   `last_synced_at`.

Conflict rule: the server provides the initial transcript; subsequent local
edits win. Retention (`storage/retention.rs`) must not delete source WAVs until
`upload_state == complete`.

## Auth & credentials

- Passwords hashed with Argon2id server-side; short-lived access JWT + rotating
  refresh token.
- The client stores the token pair in the **OS Keychain** via
  `llm/keystore.rs::KeyStore` — never in `Settings` (plaintext JSON).
- The non-secret endpoint URL lives in `Settings.remote_endpoint`.

## Privacy posture

- Remote is strictly opt-in and clearly labeled as leaving the device.
- `privacy_mode` / airgap fully disables upload and sync.
- Every outbound call passes `cloud_guard::ensure_allowed(host_of(url))`; the
  configured endpoint host is auto-allowlisted when settings are saved.
- HTTPS is enforced for non-localhost endpoints.
- Optional client-side encryption of uploads (AES-256-GCM via `encryption.rs`)
  is a follow-up for zero-trust self-hosting.

## Deployment

The default target is **Coolify**, deployed one-click as a Docker Compose
resource (base directory `server/`), the same pattern as CompanyOS. GPU is the
default: the worker reserves an NVIDIA device and runs faster-whisper on CUDA.

- `server/docker-compose.yml` — the GPU stack Coolify reads. Coolify magic
  variables make it self-configuring: `SERVICE_FQDN_API_8080` assigns/routes a
  domain (attach a custom domain in the UI), `SERVICE_BASE64_64_JWT` generates
  the JWT secret shared by api + worker.
- `server/Dockerfile.worker` — GPU worker; CUDA + cuDNN runtime come from the
  `nvidia-*-cu12` pip wheels (`requirements-gpu.txt`), so it stays on the slim
  Python base and needs only the host NVIDIA driver + Container Toolkit.
- `server/docker-compose.cpu.yml` + `Dockerfile.worker.cpu` — a CPU-only stack
  for boxes without a GPU (`./deploy.sh cpu`).
- Persistence defaults to SQLite on a Docker volume (WAL mode, shared by api +
  worker); set `FOLIO_DATABASE_URL` to a Postgres DSN for heavier / multi-worker
  use.
- TLS is terminated by Coolify's proxy in front of the API; the app points at
  `https://your-domain`.

See `server/README.md` for the concrete Coolify and local instructions.

### Verified end-to-end (2026-07-22, local)

The full flow was exercised against a live server with the real faster-whisper
engine (CPU, `tiny`): register → resumable two-chunk upload of a WAV → worker
job `succeeded` → transcript fetched in the exact `SessionTranscript` shape,
e.g. `{"channels":[{"channel":"mic","language":"en","segments":[{"start_seconds":0.0,
"end_seconds":4.64,"text":"Hello, this is a folio remote transcription test …"}]}]}`.
Reproduce with `server/scripts/smoke_e2e.py`.
