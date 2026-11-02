"""Parsing pending referee case and environment files."""

from __future__ import annotations

import json
from pathlib import Path

from sqlquack_record.recording.constants import HASH_NAME_LENGTH, HEX_DIGITS
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.types import SessionValue
from sqlquack_record.referee._helpers.hashing import case_sha256, environment_sha256
from sqlquack_record.referee.constants import CASE_KEYS, ENVIRONMENT_KEYS, REFEREE_FORMAT
from sqlquack_record.referee.models import RefereeCase, RefereeEnvironment


def parse_case(path: Path) -> RefereeCase:
    """The referee case in `path`; its keys must match and its hash must match its content."""
    document: dict[str, object] = _document(path=path, keys=CASE_KEYS)
    syntax_only: object = document["syntax_only"]
    if not isinstance(syntax_only, bool):
        raise RecorderError(f"{path.name}: syntax_only must be true or false")
    case: RefereeCase = RefereeCase(
        case_hash=str(document["case_hash"]),
        case_reference=str(document["case_reference"]),
        source_dialect=str(document["source_dialect"]),
        environment_sha256=str(document["environment_sha256"]),
        statement=str(document["statement"]),
        syntax_only=syntax_only,
    )
    expected: str = case_sha256(
        source_dialect=case.source_dialect,
        environment=case.environment_sha256,
        statement=case.statement,
        syntax_only=case.syntax_only,
    )
    if case.case_hash != path.stem or expected != path.stem:
        raise RecorderError(f"{path.name}: the case hash does not match its content")
    return case


def parse_environment(path: Path) -> RefereeEnvironment:
    """The environment in `path`; its hash must match its parameters and setup statements."""
    document: dict[str, object] = _document(path=path, keys=ENVIRONMENT_KEYS)
    parameters: object = document["session_parameters"]
    setup: object = document["setup_sql"]
    if not isinstance(parameters, dict):
        raise RecorderError(f"{path.name}: session_parameters is not an object")
    if not isinstance(setup, list) or not all(isinstance(statement, str) for statement in setup):
        raise RecorderError(f"{path.name}: setup_sql must be a list of statements")
    values: dict[str, SessionValue] = {
        str(name): _value(path=path, value=value) for name, value in parameters.items()
    }
    expected: str = environment_sha256(session_parameters=values, setup_sql=setup)
    if document["environment_sha256"] != path.stem or expected != path.stem:
        raise RecorderError(f"{path.name}: the environment hash does not match its content")
    return RefereeEnvironment(
        environment_sha256=path.stem,
        session_parameters=tuple(sorted(values.items())),
        setup_sql=tuple(setup),
    )


def is_hash_file(path: Path) -> bool:
    """Whether `path` is named `<SHA-256 hash>.json`."""
    return len(path.stem) == HASH_NAME_LENGTH and set(path.stem) <= HEX_DIGITS


def _document(*, path: Path, keys: frozenset[str]) -> dict[str, object]:
    if not is_hash_file(path):
        raise RecorderError(f"{path.name}: not named by a SHA-256 hash")
    document: object = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(document, dict) or set(document) != keys:
        raise RecorderError(f"{path.name}: not a referee file")
    if document["format"] != REFEREE_FORMAT:
        raise RecorderError(f"{path.name}: unexpected format")
    return document


def _value(*, path: Path, value: object) -> SessionValue:
    if isinstance(value, bool | int | str):
        return value
    raise RecorderError(f"{path.name}: unsupported session parameter value {value!r}")
