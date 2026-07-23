from __future__ import annotations

import json
import logging
from pathlib import Path

from app.core.config import Settings
from app.schemas.transcript import Segment

logger = logging.getLogger(__name__)


class TranscriptionEngine:
    name = "base"

    def transcribe(
        self, wav_path: str, language: str | None = None
    ) -> tuple[str | None, list[Segment]]:
        raise NotImplementedError


class StubEngine(TranscriptionEngine):
    name = "stub"

    def transcribe(
        self, wav_path: str, language: str | None = None
    ) -> tuple[str | None, list[Segment]]:
        lang = language if language and language != "auto" else "en"
        return lang, [
            Segment(
                start_seconds=0.0,
                end_seconds=0.0,
                text="[stub transcription]",
                speaker=None,
                language=lang,
            )
        ]


class FasterWhisperEngine(TranscriptionEngine):
    name = "faster_whisper"

    def __init__(self, settings: Settings) -> None:
        self._settings = settings
        self._model = None

    def _resolve_device(self) -> str:
        if self._settings.whisper_device != "auto":
            return self._settings.whisper_device
        return "cuda" if gpu_available() else "cpu"

    def _resolve_compute_type(self, device: str) -> str:
        if self._settings.whisper_compute_type != "auto":
            return self._settings.whisper_compute_type
        return "float16" if device == "cuda" else "int8"

    def _load(self):
        if self._model is not None:
            return self._model
        from faster_whisper import WhisperModel

        device = self._resolve_device()
        compute_type = self._resolve_compute_type(device)
        logger.info(
            "loading faster-whisper model=%s device=%s compute_type=%s",
            self._settings.whisper_model,
            device,
            compute_type,
        )
        self._model = WhisperModel(
            self._settings.whisper_model, device=device, compute_type=compute_type
        )
        return self._model

    def transcribe(
        self, wav_path: str, language: str | None = None
    ) -> tuple[str | None, list[Segment]]:
        model = self._load()
        hint = language if language and language != "auto" else None
        segments, info = model.transcribe(
            wav_path,
            language=hint,
            vad_filter=True,
            condition_on_previous_text=False,
        )
        out: list[Segment] = []
        detected = getattr(info, "language", None)
        for seg in segments:
            out.append(
                Segment(
                    start_seconds=float(seg.start),
                    end_seconds=float(seg.end),
                    text=seg.text.strip(),
                    speaker=None,
                    language=detected,
                )
            )
        return detected, out


def gpu_available() -> bool:
    try:
        import ctranslate2

        return ctranslate2.get_cuda_device_count() > 0
    except Exception:
        return False


def faster_whisper_available() -> bool:
    try:
        import faster_whisper  # noqa: F401

        return True
    except Exception:
        return False


def resolve_engine_name(settings: Settings) -> str:
    if settings.whisper_engine == "stub":
        return "stub"
    if settings.whisper_engine == "faster_whisper":
        return "faster_whisper"
    return "faster_whisper" if faster_whisper_available() else "stub"


def get_engine(settings: Settings) -> TranscriptionEngine:
    if resolve_engine_name(settings) == "faster_whisper":
        return FasterWhisperEngine(settings)
    return StubEngine()


def wav_exists(path: str) -> bool:
    return Path(path).exists()


def worker_caps_path(settings: Settings) -> Path:
    return Path(settings.storage_dir) / "_worker_capabilities.json"


def write_worker_capabilities(settings: Settings) -> None:
    engine = get_engine(settings)
    caps = {
        "engine": engine.name,
        "model": settings.whisper_model,
        "gpu": gpu_available(),
        "diarization": settings.diarization_enabled,
    }
    path = worker_caps_path(settings)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(caps))


def read_worker_capabilities(settings: Settings) -> dict | None:
    try:
        return json.loads(worker_caps_path(settings).read_text())
    except Exception:
        return None
