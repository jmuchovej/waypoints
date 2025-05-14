from uuid import UUID, uuid4
from enum import Enum
from datetime import datetime, timezone

from sqlmodel import Field, SQLModel

class Recording(SQLModel, table=True):
    id: UUID = Field(default_factory=uuid4, primary_key=True)
    filename: str
    duration: int
    bit_rate: int
    sample_rate: int
    channels: int
    filesize: int
    uploaded: bool = Field(default=False)
    checksum: str = Field(index=True, unique=True)

    #! TODO: can't implement this until something like this comment is advanced:
    #!   https://github.com/fastapi/sqlmodel/issues/539#issuecomment-2439862367
    #!   https://github.com/fastapi/sqlmodel/issues/594#issuecomment-1575344153
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))

    transcript_id: UUID | None = Field(default=None, foreign_key="transcript.id")


class TranscriptStatus(Enum):
    WAITING     = "w"
    PENDING     = "p"
    COMPLETE    = "c"

class Transcript(SQLModel, table=True):
    id: UUID = Field(default_factory=uuid4, primary_key=True)
    duration: int
    status: TranscriptStatus = Field(default=TranscriptStatus.WAITING)

    #! TODO: can't implement this until something like this comment is advanced:
    #!   https://github.com/fastapi/sqlmodel/issues/539#issuecomment-2439862367
    #!   https://github.com/fastapi/sqlmodel/issues/594#issuecomment-1575344153
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))

    recording_id: UUID | None = Field(default=None, foreign_key="recording.id")

Models = [Recording, Transcript]
