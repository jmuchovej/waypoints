from datetime import datetime, timezone
from hashlib import sha256
from pathlib import Path
from tempfile import NamedTemporaryFile
from uuid import UUID, uuid4
from fastapi import UploadFile, HTTPException
from minio import Minio, S3Error
from soundfile import SoundFile
from sqlmodel import Session, select

from tempo.common.tools import logger
from tempo.transcription.models import Recording


CHUNK_SIZE = (1024) ^ 2 * 128


async def gather_facts(file: UploadFile, uuid: UUID = uuid4()) -> Recording:
    await file.seek(0)
    try:
        with NamedTemporaryFile() as tmp:
            checksum = sha256()
            filesize = 0
            while chunk := await file.read(CHUNK_SIZE):
                checksum.update(chunk)
                tmp.write(chunk)
                filesize += len(chunk)
            tmp.flush()

            with SoundFile(tmp.name) as audio:
                sample_rate = audio.samplerate
                duration = len(audio) / sample_rate
                channels = audio.channels
                bit_depth_map = {
                    "PCM_16": 16,
                    "PCM_24": 24,
                    "PCM_32": 32,
                    "FLOAT": 32,
                    "DOUBLE": 64,
                }
                bit_depth = bit_depth_map[audio.subtype]
                bit_rate = sample_rate * channels * bit_depth
                logger.info(f"{duration=} {sample_rate=} {channels=} {bit_rate=}")

        ext = Path(file.filename).suffix.replace(".", "")
        ext = ext if ext != "" else "bin"
        filename = f"{uuid}.{ext}"
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to ingest audio: {str(e)}")

    return Recording(
        id=uuid,
        filename=filename,
        duration=duration,
        sample_rate=sample_rate,
        channels=channels,
        bit_rate=bit_rate,
        checksum=checksum.hexdigest(),
        filesize=filesize,
    )

def get_or_create(session: Session, recording: Recording) -> Recording:
    stmt = select(Recording).where(Recording.checksum == recording.checksum)
    entry = session.exec(stmt).one_or_none()

    if entry:
        return entry

    try:
        session.add(recording)
        session.commit()
        session.refresh(recording)
    except Exception as e:
        session.rollback()
        raise HTTPException(status_code=500, detail=f"Failed initial database insertion: {str(e)}")

    return recording


async def upload(bucket: Minio, session: Session, recording: Recording, upload: UploadFile) -> None:
    from tempo.transcription.tools import Buckets
    bucket_name = Buckets.RECORDINGS.value

    try:
        await upload.seek(0)  # Reset for uploading to bucket
        bucket.fput_object(bucket_name, recording.filename, upload.file.fileno())
    except Exception as e:
        try:
            bucket.remove_object(bucket_name, recording.filename)
        except S3Error:
            pass

        stmt = select(Recording).where(Recording.id == recording.id)
        entry = session.exec(stmt).one_or_none()
        session.delete(entry)
        session.commit()

        raise HTTPException(500, detail=f"Failed to upload: {str(e)}")

    try:
        recording.uploaded = True
        recording.updated_at = datetime.now(timezone.utc)

        session.add(recording)
        session.commit()
    except Exception as e:
        session.rollback()
        raise HTTPException(500, detail=f"Failed to update Recording({recording.id}): {str(e)}")

    return recording
