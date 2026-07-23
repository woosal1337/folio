from fastapi import APIRouter

from app.core.config import get_settings
from app.transcription.engine import (
    gpu_available,
    read_worker_capabilities,
    resolve_engine_name,
)

router = APIRouter()


@router.get("/health")
async def health() -> dict:
    return {"status": "ok"}


@router.get("/v1/capabilities")
async def capabilities() -> dict:
    settings = get_settings()
    worker = read_worker_capabilities(settings) or {}
    return {
        "name": settings.app_name,
        "version": settings.version,
        "engine": worker.get("engine", resolve_engine_name(settings)),
        "model": worker.get("model", settings.whisper_model),
        "gpu": worker.get("gpu", gpu_available()),
        "diarization": worker.get("diarization", settings.diarization_enabled),
        "max_upload_bytes": settings.max_upload_bytes,
        "registration_open": settings.allow_registration,
    }
