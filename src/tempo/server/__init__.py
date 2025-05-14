from contextlib import asynccontextmanager

from fastapi import FastAPI

try:
    from tempo import transcription as _transcription
except ImportError:
    print("Attempted to import `tempo.transcription`, but couldn't find it!")


@asynccontextmanager
async def lifespan(app: FastAPI):
    if _transcription:
        _transcription.tools.ensure_buckets()

    yield

app = FastAPI(lifespan=lifespan, redirect_slashes=False)

if _transcription:
    app.mount("/transcription", _transcription.api.app)


