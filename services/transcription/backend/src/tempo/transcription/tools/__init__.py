from enum import Enum

from tempo.common.clients import get_bucket


class Buckets(Enum):
    BASE = "transcription"
    RECORDINGS = "recordings"
    TRANSCRIPTS = "transcripts"


def ensure_buckets() -> None:
    client = get_bucket()
    for bucket in Buckets:
        try:
            assert client.bucket_exists(bucket.value)
        except AssertionError:
            client.make_bucket(bucket_name=bucket.value)
