"""Parsing one pending case file."""

from __future__ import annotations

import json
from pathlib import Path

from sqlquack_record.recording.constants import (
    EVIDENCE_FORMAT,
    HASH_NAME_LENGTH,
    HEX_DIGITS,
    OPTIONAL_PENDING_KEYS,
    PENDING_KEYS,
    PINNED_KEY,
    SETUP_KEY,
    VALUES_KEY,
)
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import PendingCase
from sqlquack_record.recording.types import SessionValue


def is_case_file(path: Path) -> bool:
    """Whether `path` is named `<case hash>.json`."""
    return len(path.stem) == HASH_NAME_LENGTH and set(path.stem) <= HEX_DIGITS


def parse_pending(path: Path) -> PendingCase:
    """The pending case in `path`; its keys and hash must match the format exactly."""
    document: object = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(document, dict) or set(document) - OPTIONAL_PENDING_KEYS != PENDING_KEYS:
        raise RecorderError(f"{path.name}: not a pending case file")
    values: object = document.get(VALUES_KEY, True)
    if not isinstance(values, bool):
        raise RecorderError(f"{path.name}: values must be true or false")
    setup: object = document.get(SETUP_KEY, [])
    if not isinstance(setup, list) or not all(isinstance(statement, str) for statement in setup):
        raise RecorderError(f"{path.name}: setup_sql must be a list of statements")
    if document["format"] != EVIDENCE_FORMAT or document["case_hash"] != path.stem:
        raise RecorderError(f"{path.name}: unexpected format or case hash")
    parameters: object = document["session_parameters"]
    if not isinstance(parameters, dict):
        raise RecorderError(f"{path.name}: session_parameters is not an object")
    pinned: object = document.get(PINNED_KEY, {})
    if not isinstance(pinned, dict):
        raise RecorderError(f"{path.name}: pinned_parameters is not an object")
    return PendingCase(
        case_hash=str(document["case_hash"]),
        case_reference=str(document["case_reference"]),
        source_dialect=str(document["source_dialect"]),
        session_parameters=tuple(
            sorted((str(key), _value(path=path, value=value)) for key, value in parameters.items())
        ),
        source_sql=str(document["source_sql"]),
        duckdb_version=str(document["duckdb_version"]),
        values=values,
        setup_sql=tuple(setup),
        pinned_parameters=tuple(
            sorted((str(key), _value(path=path, value=value)) for key, value in pinned.items())
        ),
    )


def _value(*, path: Path, value: object) -> SessionValue:
    if isinstance(value, bool | int | str):
        return value
    raise RecorderError(f"{path.name}: unsupported session parameter value {value!r}")
