"""Reading evidence.toml."""

from __future__ import annotations

import re
import tomllib

from scripts.evidence_pin.constants import (
    COMMIT_PATTERN,
    CONTENT_SHA256_PATTERN,
    PIN_FILE,
    PIN_KEYS,
)
from scripts.evidence_pin.exceptions import PinFileError
from scripts.evidence_pin.models import EvidencePin


def parsed_pin(text: str) -> EvidencePin:
    """The pin `text` holds, or PinFileError saying why it is not a well-formed pin."""
    try:
        document: dict[str, object] = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise PinFileError(f"{PIN_FILE} is not TOML: {error}") from error
    if set(document) != PIN_KEYS:
        raise PinFileError(f"{PIN_FILE} holds exactly the keys {', '.join(sorted(PIN_KEYS))}")
    repository: object = document["repository"]
    commit: object = document["commit"]
    content_sha256: object = document["content_sha256"]
    if not isinstance(repository, str) or not repository:
        raise PinFileError(f"{PIN_FILE}'s repository is not a URL")
    if not isinstance(commit, str) or not re.fullmatch(COMMIT_PATTERN, commit):
        raise PinFileError(f"{PIN_FILE}'s commit is not a 40-digit lower-case hex SHA")
    if not isinstance(content_sha256, str) or not re.fullmatch(
        CONTENT_SHA256_PATTERN, content_sha256
    ):
        raise PinFileError(f"{PIN_FILE}'s content_sha256 is not a 64-digit lower-case hex SHA-256")
    return EvidencePin(repository=repository, commit=commit, content_sha256=content_sha256)
