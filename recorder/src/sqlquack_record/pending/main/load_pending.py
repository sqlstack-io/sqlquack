"""Load the pending case files `sqlquack pending` wrote."""

from __future__ import annotations

import json
from pathlib import Path

from sqlquack_record.constants import EVIDENCE_FORMAT, HASH_NAME_LENGTH
from sqlquack_record.errors import RecorderError
from sqlquack_record.models import PendingCase, SessionValue

_KEYS: frozenset[str] = frozenset(
    {
        "format",
        "case_hash",
        "case_reference",
        "source_dialect",
        "session_parameters",
        "source_sql",
        "duckdb_version",
    }
)


def load_pending(directory: Path) -> list[PendingCase]:
    """Every pending case in `directory`, sorted by case reference."""
    if not directory.is_dir():
        return []
    cases: list[PendingCase] = [
        _parse(path) for path in sorted(directory.glob("*.json")) if _is_case_file(path)
    ]
    return sorted(cases, key=lambda case: case.case_reference)


def _is_case_file(path: Path) -> bool:
    stem: str = path.stem
    return len(stem) == HASH_NAME_LENGTH and all(
        character in "0123456789abcdef" for character in stem
    )


def _parse(path: Path) -> PendingCase:
    document: object = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(document, dict) or set(document) != _KEYS:
        raise RecorderError(f"{path.name}: not a pending case file")
    if document["format"] != EVIDENCE_FORMAT or document["case_hash"] != path.stem:
        raise RecorderError(f"{path.name}: unexpected format or case hash")
    parameters: object = document["session_parameters"]
    if not isinstance(parameters, dict):
        raise RecorderError(f"{path.name}: session_parameters is not an object")
    return PendingCase(
        case_hash=str(document["case_hash"]),
        case_reference=str(document["case_reference"]),
        source_dialect=str(document["source_dialect"]),
        session_parameters=tuple(
            sorted((str(key), _value(path, value)) for key, value in parameters.items())
        ),
        source_sql=str(document["source_sql"]),
        duckdb_version=str(document["duckdb_version"]),
    )


def _value(path: Path, value: object) -> SessionValue:
    if isinstance(value, bool | int | str):
        return value
    raise RecorderError(f"{path.name}: unsupported session parameter value {value!r}")
