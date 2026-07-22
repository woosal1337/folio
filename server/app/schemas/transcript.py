from pydantic import BaseModel


class Segment(BaseModel):
    start_seconds: float
    end_seconds: float
    text: str
    speaker: int | None = None
    language: str | None = None


class ChannelTranscript(BaseModel):
    channel: str
    language: str | None = None
    segments: list[Segment]


class SessionTranscript(BaseModel):
    channels: list[ChannelTranscript]
