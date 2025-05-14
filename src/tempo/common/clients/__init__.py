import os
from typing import Annotated

from fastapi import Depends
from minio import Minio
from sqlmodel import Session, create_engine

bucket = Minio(
    os.getenv("MINIO_CLIENT"),
    access_key=os.getenv("MINIO_ACCESS_KEY"),
    secret_key=os.getenv("MINIO_SECRET_KEY"),
    # TODO ensure that, in prod, `secure=True`
    secure=False,
)

def get_bucket() -> Minio:
    return bucket

db = create_engine(os.getenv("DATABASE_URL"))

def get_session():
    with Session(db) as session:
        yield session

BucketDep = Annotated[Minio, Depends(get_bucket)]
SessionDep = Annotated[Session, Depends(get_session)]

__all__ = [
  "get_bucket",
  "get_session",
  "BucketDep",
  "SessionDep",
]
