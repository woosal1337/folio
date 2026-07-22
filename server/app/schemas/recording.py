from datetime import datetime

from pydantic import BaseModel


class RecordingCreate(BaseModel):
    client_id: str
    label: str = ""
    duration_seconds: int = 0


class ChannelOut(BaseModel):
    name: str
    size_bytes: int
    upload_complete: bool

    model_config = {"from_attributes": True}


class RecordingOut(BaseModel):
    id: str
    client_id: str
    label: str
    duration_seconds: int
    status: str
    created_at: datetime
    updated_at: datetime
    channels: list[ChannelOut] = []

    model_config = {"from_attributes": True}


class UploadResult(BaseModel):
    offset: int
    complete: bool


class TranscribeRequest(BaseModel):
    language: str | None = None
    diarize: bool = False


class JobOut(BaseModel):
    id: str
    recording_id: str
    status: str
    progress: float
    error: str | None = None
    created_at: datetime
    updated_at: datetime

    model_config = {"from_attributes": True}
