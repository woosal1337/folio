# Folio remote transcription server

A self-hostable, GPU-accelerated transcription backend for
[Folio](https://folio.chele.bi). Folio uploads a recording, this server
transcribes it (faster-whisper / CTranslate2 on the GPU), and the transcript
syncs back to the desktop app — so your Mac can sleep while the GPU does the work.

Local transcription stays the default in Folio; this server is an opt-in remote
backend you point the app at. See [`../docs/REMOTE_SERVER.md`](../docs/REMOTE_SERVER.md)
for the architecture and API contract.

## Deploy on Coolify (one-click)

The stack is built for a one-click Coolify deploy, the same way CompanyOS is
deployed:

1. In Coolify, **＋ New Resource → Docker Compose**, point it at this repo and set
   the **Base Directory** to `server/`.
2. Coolify reads `docker-compose.yml`. The magic variables make it self-configuring:
   - `SERVICE_FQDN_API_8080` — Coolify assigns a domain and routes it to the API.
     Attach your **custom domain** to the `api` service in the Coolify UI.
   - `SERVICE_BASE64_64_JWT` — Coolify generates the JWT signing secret (shared by
     api + worker); nothing to paste.
3. Enable **GPU** for the resource (GPU-capable Coolify server + NVIDIA Container
   Toolkit). The worker reserves an NVIDIA device by default.
4. **Deploy.** In Folio open **Account** in the sidebar, set the endpoint to
   `https://your-domain`, hit **Test** (it should report `faster_whisper` and
   GPU), create your account, then pick **Remote server** as the provider under
   _Settings → Transcription_ — or use **Make default** on the Account page.

Optional overrides (Coolify → Environment Variables): `FOLIO_WHISPER_MODEL`
(default `base`; use `small`/`medium`/`large-v3` on a bigger GPU),
`FOLIO_ALLOW_REGISTRATION=false` to lock a personal server after you sign up.

The GPU worker image stays on `python:3.12-slim`: CUDA + cuDNN runtimes come
from the `nvidia-*-cu12` pip wheels (`requirements-gpu.txt`), so the host only
needs the NVIDIA driver and the NVIDIA Container Toolkit — no CUDA base image.

## Run with Docker locally

GPU host:

```bash
cd server
cp .env.example .env          # set FOLIO_JWT_SECRET
./deploy.sh                   # GPU stack (docker-compose.yml)
```

No GPU (CPU-only):

```bash
cd server
cp .env.example .env
./deploy.sh cpu               # docker-compose.cpu.yml, API on :8080
```

## Run without Docker (dev)

```bash
cd server
python -m venv .venv && source .venv/bin/activate
pip install -r requirements-dev.txt        # API + stub engine + test tools
pip install faster-whisper                 # optional: real ASR on CPU
cp .env.example .env
# single process (API + worker), stub engine, no models needed:
FOLIO_RUN_WORKER_IN_PROCESS=true uvicorn app.main:app --port 8080
```

For real transcription set `FOLIO_WHISPER_ENGINE=faster_whisper` and a model
(`FOLIO_WHISPER_MODEL=tiny` for a fast local check). Prove the whole flow with a
real audio file:

```bash
python scripts/smoke_e2e.py http://127.0.0.1:8080 /path/to/mic.wav
```

## Configuration

Everything is driven by `FOLIO_*` environment variables — see
[`.env.example`](.env.example). Highlights:

| Var                        | Default                               | Notes                                       |
| -------------------------- | ------------------------------------- | ------------------------------------------- |
| `FOLIO_JWT_SECRET`         | `change-me-in-production`             | Coolify generates this; set it otherwise    |
| `FOLIO_DATABASE_URL`       | `sqlite+aiosqlite:///./data/folio.db` | Postgres: `postgresql+asyncpg://…`          |
| `FOLIO_STORAGE_DIR`        | `./data/blobs`                        | uploaded audio + transcripts                |
| `FOLIO_WHISPER_MODEL`      | `base`                                | `tiny`…`large-v3`                           |
| `FOLIO_WHISPER_DEVICE`     | `auto`                                | compose sets `cuda` on the GPU worker       |
| `FOLIO_WHISPER_ENGINE`     | `auto`                                | `auto` / `faster_whisper` / `stub`          |
| `FOLIO_ALLOW_REGISTRATION` | `true`                                | disable to lock down a personal server      |
| `FOLIO_ENVIRONMENT`        | `development`                         | `production` refuses the default JWT secret |

Registration is open by default so you can create your first account; on an
internet-facing server set `FOLIO_ALLOW_REGISTRATION=false` afterwards.
`/v1/capabilities` reports whether registration is open. With
`FOLIO_ENVIRONMENT=production` (the compose default) the API refuses to boot
while `FOLIO_JWT_SECRET` is still `change-me-in-production`.

## Health & capabilities

```bash
curl https://your-domain/health
curl https://your-domain/v1/capabilities   # engine, model, gpu, diarization, version
```

## Tests

```bash
cd server && pip install -r requirements-dev.txt && pytest
```

Tests use the built-in **stub** engine, so no GPU or model download is required.

## Layout

```
server/
├── app/
│   ├── main.py                 # FastAPI app factory
│   ├── core/                   # config, security (argon2 + JWT), logging
│   ├── db/                     # SQLAlchemy engine (+ SQLite WAL) + models
│   ├── schemas/                # request/response + SessionTranscript models
│   ├── storage/                # object store (local FS)
│   ├── transcription/          # engine: stub + faster-whisper
│   ├── api/routes/             # health, auth, recordings, jobs
│   └── workers/                # GPU transcription worker (DB-polling)
├── scripts/smoke_e2e.py        # real upload→transcribe→fetch check
├── tests/
├── Dockerfile                  # API
├── Dockerfile.worker           # GPU worker (default)
├── Dockerfile.worker.cpu       # CPU worker
├── docker-compose.yml          # GPU stack, Coolify one-click
└── docker-compose.cpu.yml      # CPU stack
```
