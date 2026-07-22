import json
from datetime import datetime

from fastapi import APIRouter, Depends, Header, HTTPException, Request, Response, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import selectinload

from app.api.deps import get_current_user
from app.core.config import get_settings
from app.db.base import get_session
from app.db.models import Channel, Job, Recording, Transcript, User
from app.schemas.recording import (
    JobOut,
    RecordingCreate,
    RecordingOut,
    TranscribeRequest,
    UploadResult,
)
from app.storage import get_storage

router = APIRouter(prefix="/v1/recordings", tags=["recordings"])

VALID_CHANNELS = {"mic", "system"}


async def _load(session: AsyncSession, rec_id: str, user_id: str) -> Recording | None:
    result = await session.execute(
        select(Recording)
        .where(Recording.id == rec_id, Recording.user_id == user_id)
        .options(selectinload(Recording.channels))
    )
    return result.scalar_one_or_none()


@router.post("", response_model=RecordingOut)
async def create_recording(
    body: RecordingCreate,
    user: User = Depends(get_current_user),
    session: AsyncSession = Depends(get_session),
) -> Recording:
    result = await session.execute(
        select(Recording)
        .where(Recording.user_id == user.id, Recording.client_id == body.client_id)
        .options(selectinload(Recording.channels))
    )
    rec = result.scalar_one_or_none()
    if rec is None:
        rec = Recording(
            user_id=user.id,
            client_id=body.client_id,
            label=body.label,
            duration_seconds=body.duration_seconds,
        )
        session.add(rec)
    else:
        if body.label:
            rec.label = body.label
        if body.duration_seconds:
            rec.duration_seconds = body.duration_seconds
    await session.commit()
    return await _load(session, rec.id, user.id)


@router.get("", response_model=list[RecordingOut])
async def list_recordings(
    updated_since: str | None = None,
    user: User = Depends(get_current_user),
    session: AsyncSession = Depends(get_session),
) -> list[Recording]:
    stmt = (
        select(Recording)
        .where(Recording.user_id == user.id)
        .options(selectinload(Recording.channels))
    )
    if updated_since:
        try:
            since = datetime.fromisoformat(updated_since)
        except ValueError as exc:
            raise HTTPException(
                status.HTTP_422_UNPROCESSABLE_ENTITY, "invalid updated_since"
            ) from exc
        stmt = stmt.where(Recording.updated_at >= since)
    result = await session.execute(stmt.order_by(Recording.updated_at.desc()))
    return list(result.scalars().all())


@router.get("/{rec_id}", response_model=RecordingOut)
async def get_recording(
    rec_id: str,
    user: User = Depends(get_current_user),
    session: AsyncSession = Depends(get_session),
) -> Recording:
    rec = await _load(session, rec_id, user.id)
    if rec is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "recording not found")
    return rec


@router.delete("/{rec_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_recording(
    rec_id: str,
    user: User = Depends(get_current_user),
    session: AsyncSession = Depends(get_session),
) -> Response:
    rec = await _load(session, rec_id, user.id)
    if rec is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "recording not found")
    storage = get_storage()
    for channel in rec.channels:
        storage.delete(channel.storage_key)
    await session.delete(rec)
    await session.commit()
    return Response(status_code=status.HTTP_204_NO_CONTENT)


@router.put("/{rec_id}/channels/{name}", response_model=UploadResult)
async def upload_channel(
    rec_id: str,
    name: str,
    request: Request,
    upload_offset: int = Header(0, alias="Upload-Offset"),
    upload_complete: str = Header("false", alias="Upload-Complete"),
    content_sha256: str | None = Header(None, alias="X-Content-Sha256"),
    user: User = Depends(get_current_user),
    session: AsyncSession = Depends(get_session),
) -> UploadResult:
    if name not in VALID_CHANNELS:
        raise HTTPException(status.HTTP_400_BAD_REQUEST, "channel must be mic or system")
    rec = await _load(session, rec_id, user.id)
    if rec is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "recording not found")

    settings = get_settings()
    data = await request.body()
    if upload_offset + len(data) > settings.max_upload_bytes:
        raise HTTPException(status.HTTP_413_REQUEST_ENTITY_TOO_LARGE, "upload too large")

    storage = get_storage()
    key = f"{user.id}/{rec.id}/{name}.wav"
    try:
        new_size = storage.append(key, upload_offset, data)
    except ValueError:
        current = storage.current_size(key)
        raise HTTPException(
            status.HTTP_409_CONFLICT,
            detail={"message": "offset mismatch", "offset": current},
        ) from None

    channel = next((c for c in rec.channels if c.name == name), None)
    if channel is None:
        channel = Channel(recording_id=rec.id, name=name, storage_key=key)
        session.add(channel)
    channel.storage_key = key
    channel.size_bytes = new_size

    complete = upload_complete.lower() == "true"
    if complete:
        size, actual = storage.finalize(key, content_sha256)
        channel.size_bytes = size
        channel.sha256 = actual
        channel.upload_complete = True
    else:
        rec.status = "uploading"

    await session.commit()
    return UploadResult(offset=channel.size_bytes, complete=complete)


@router.post("/{rec_id}/transcribe", response_model=JobOut)
async def transcribe_recording(
    rec_id: str,
    body: TranscribeRequest,
    user: User = Depends(get_current_user),
    session: AsyncSession = Depends(get_session),
) -> Job:
    rec = await _load(session, rec_id, user.id)
    if rec is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "recording not found")
    if not any(c.upload_complete for c in rec.channels):
        raise HTTPException(
            status.HTTP_400_BAD_REQUEST, "no fully-uploaded channels to transcribe"
        )
    job = Job(
        recording_id=rec.id,
        user_id=user.id,
        status="queued",
        language=body.language,
        diarize=body.diarize,
    )
    session.add(job)
    rec.status = "queued"
    await session.commit()
    await session.refresh(job)
    return job


@router.get("/{rec_id}/transcript")
async def get_transcript(
    rec_id: str,
    user: User = Depends(get_current_user),
    session: AsyncSession = Depends(get_session),
) -> dict:
    rec = await _load(session, rec_id, user.id)
    if rec is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "recording not found")
    result = await session.execute(
        select(Transcript).where(Transcript.recording_id == rec.id)
    )
    transcript = result.scalar_one_or_none()
    if transcript is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "no transcript yet")
    return json.loads(transcript.payload)
