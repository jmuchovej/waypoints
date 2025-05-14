import logging

from rich.logging import RichHandler

from slowapi import Limiter
from slowapi.util import get_remote_address

logging.basicConfig(
    level="INFO",
    format="%(message)s",
    datefmt="[%X]",
    handlers=[
        RichHandler(rich_tracebacks=True),
    ],
)
logger = logging.getLogger("tempo")

limiter = Limiter(key_func=get_remote_address)
