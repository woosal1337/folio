#!/usr/bin/env bash
set -euo pipefail

# One-command bring-up for the Folio transcription server.
# Usage:
#   ./deploy.sh          # GPU stack (default; requires NVIDIA runtime)
#   ./deploy.sh cpu      # CPU-only stack
#
# Production deploys usually go through Coolify (Docker Compose resource) which
# reads docker-compose.yml directly — see README.md.

cd "$(dirname "$0")"

if [ ! -f .env ]; then
  cp .env.example .env
  echo "Created .env from .env.example — set FOLIO_JWT_SECRET before exposing this server."
fi

MODE="${1:-gpu}"
if [ "$MODE" = "cpu" ]; then
  exec docker compose -f docker-compose.cpu.yml up --build -d
else
  exec docker compose up --build -d
fi
