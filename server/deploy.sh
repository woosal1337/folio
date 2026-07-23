#!/usr/bin/env bash
set -euo pipefail

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
