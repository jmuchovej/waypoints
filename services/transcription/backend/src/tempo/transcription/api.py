from contextlib import asynccontextmanager
from uuid import UUID

from fastapi import APIRouter, FastAPI, HTTPException, Query, Request, UploadFile

from sqlmodel import select
from tempo.transcription.tools import ensure_buckets
from tempo.transcription.models import Recording

from tempo.common.clients import BucketDep, SessionDep
from tempo.common.tools import limiter

@asynccontextmanager
async def lifespan(app: FastAPI):
    ensure_buckets()
    yield

app = FastAPI(
    title="Transcription",
    lifespan=lifespan,
    openapi_tags=[
        dict(name="transcription", description="Transcriptions"),
        dict(name="recording", description="Recordings"),
        dict(name="transcript", description="Transcripts"),
    ],
    responses={
        403: { "description": "Not authorized." },
        404: { "description": "Not found." },
    },
)

@app.get("/", tags=["transcription"])
async def root():
    return {
        "description": "For API details, head to ...",
    }


recordings_router = APIRouter(
    prefix="/recordings",
    tags=["recordings"],
    redirect_slashes=True,
    dependencies=[
        # Depends(get_session)
    ]
)

@recordings_router.get("/", response_model=list[Recording])
@limiter.limit("100/minute")
async def get_recordings(
    request: Request, session: SessionDep,
    offset: int = 0, limit: int = Query(default=100, le=100),
) -> list[Recording] | None:
    recordings = session.exec(select(Recording).offset(offset).limit(limit)).all()
    return recordings

@recordings_router.get("/{id}", response_model=list[Recording])
@limiter.limit("10/minute")
async def get_recording(
    request: Request,
    id: UUID,
    session: SessionDep,
) -> Recording | None:
    recordings = session.exec(select(Recording)).where(Recording.id == id).one()
    return recordings

@recordings_router.post("/", response_model=Recording)
@limiter.limit("10/minute")
async def new_recording(
    request: Request,
    file: UploadFile,
    session: SessionDep, bucket: BucketDep,
) -> Recording:
    from tempo.transcription._api import _recording as recordings
    recording = await recordings.gather_facts(file)
    recording = recordings.get_or_create(session, recording)
    recording = await recordings.upload(bucket, session, recording, file)

    return recording.model_dump_json()

@recordings_router.patch("/{uuid}", response_model=Recording)
@limiter.limit("10/minute")
async def update_recording(
    request: Request,
    uuid: UUID, file: UploadFile,
    session: SessionDep, bucket: BucketDep,
) -> Recording:
    from tempo.transcription._api import _recording as recordings

    recording = await recordings.gather_facts(file, uuid=uuid)
    recording = recordings.get_or_create(session, recording)
    recording = await recordings.upload(bucket, session, recording, file)

    return recording.model_dump_json()

app.include_router(recordings_router)

# app.include_router(transcripts)
