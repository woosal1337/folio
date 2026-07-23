from __future__ import annotations

import asyncio
import logging

from sqlalchemy import select

from app.core.config import get_settings
from app.core.logging import configure_logging
from app.db.base import SessionLocal, init_db
from app.db.models import Channel, Job, Recording, Transcript
from app.schemas.transcript import ChannelTranscript, SessionTranscript
from app.storage import get_storage
from app.transcription.engine import get_engine, write_worker_capabilities

logger = logging.getLogger(__name__)


async def _claim_next_job_id() -> str | None:
    async with SessionLocal() as session:
        result = await session.execute(
            select(Job)
            .where(Job.status == "queued")
            .order_by(Job.created_at)
            .limit(1)
        )
        job = result.scalar_one_or_none()
        if job is None:
            return None
        job.status = "running"
        job.progress = 0.0
        await session.commit()
        return job.id


async def _process_job(job_id: str) -> None:
    settings = get_settings()
    engine = get_engine(settings)
    storage = get_storage()

    async with SessionLocal() as session:
        job = await session.get(Job, job_id)
        if job is None:
            return
        try:
            result = await session.execute(
                select(Channel).where(
                    Channel.recording_id == job.recording_id,
                    Channel.upload_complete.is_(True),
                )
            )
            channels = list(result.scalars().all())
            if not channels:
                raise RuntimeError("no uploaded channels to transcribe")

            channels_out: list[ChannelTranscript] = []
            for index, channel in enumerate(channels):
                path = str(storage.local_path(channel.storage_key))
                language, segments = await asyncio.to_thread(
                    engine.transcribe, path, job.language
                )
                channels_out.append(
                    ChannelTranscript(
                        channel=channel.name, language=language, segments=segments
                    )
                )
                job.progress = (index + 1) / len(channels)
                await session.commit()

            payload = SessionTranscript(channels=channels_out).model_dump_json()
            existing = await session.execute(
                select(Transcript).where(Transcript.recording_id == job.recording_id)
            )
            transcript = existing.scalar_one_or_none()
            if transcript is None:
                session.add(
                    Transcript(recording_id=job.recording_id, payload=payload)
                )
            else:
                transcript.payload = payload

            job.status = "succeeded"
            job.progress = 1.0
            recording = await session.get(Recording, job.recording_id)
            if recording is not None:
                recording.status = "transcribed"
            await session.commit()
            logger.info("job %s succeeded (%d channels)", job_id, len(channels_out))
        except Exception as exc:  # noqa: BLE001
            await session.rollback()
            failed = await session.get(Job, job_id)
            if failed is not None:
                failed.status = "failed"
                failed.error = str(exc)
                await session.commit()
            logger.exception("job %s failed", job_id)


async def process_next() -> bool:
    job_id = await _claim_next_job_id()
    if job_id is None:
        return False
    await _process_job(job_id)
    return True


async def run_worker() -> None:
    settings = get_settings()
    logger.info("worker started (engine=%s)", get_engine(settings).name)
    try:
        write_worker_capabilities(settings)
    except Exception as exc:  # noqa: BLE001
        logger.warning("could not publish worker capabilities: %s", exc)
    while True:
        if not await process_next():
            await asyncio.sleep(settings.worker_poll_interval_seconds)


def main() -> None:
    configure_logging()

    async def _bootstrap() -> None:
        await init_db()
        await run_worker()

    asyncio.run(_bootstrap())


if __name__ == "__main__":
    main()
