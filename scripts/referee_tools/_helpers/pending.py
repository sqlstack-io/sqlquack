"""Writing pending referee files for the recorder, and removing stale ones."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.referee_tools.constants import (
    CASES_DIRECTORY,
    CATALOGUE_REQUEST,
    ENVIRONMENTS_DIRECTORY,
    HASH_LENGTH,
    HEX_DIGITS,
    JSON_SUFFIX,
    REFEREE_FORMAT,
    SOURCE_DIALECT,
)
from scripts.referee_tools.models import EnvironmentFile, RefereeCase


def write_pending(
    *, directory: Path, cases: list[RefereeCase], catalogue: bool
) -> tuple[int, int, int]:
    """Pending files for `cases` and their environments; written, removed and environments."""
    environments: dict[str, EnvironmentFile] = {
        case.environment.environment_sha256: case.environment for case in cases
    }
    wanted: dict[str, dict[str, object]] = {
        f"{CASES_DIRECTORY}/{case.case_hash}{JSON_SUFFIX}": _case_document(case) for case in cases
    }
    wanted.update(
        (f"{ENVIRONMENTS_DIRECTORY}/{digest}{JSON_SUFFIX}", _environment_document(environment))
        for digest, environment in environments.items()
    )
    removed: int = _remove_stale(directory=directory, wanted=frozenset(wanted))
    for name, document in wanted.items():
        path: Path = directory / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    request: Path = directory / CATALOGUE_REQUEST
    if catalogue:
        request.write_text(json.dumps({"format": REFEREE_FORMAT}) + "\n", encoding="utf-8")
    else:
        request.unlink(missing_ok=True)
    return len({case.case_hash for case in cases}), removed, len(environments)


def _case_document(case: RefereeCase) -> dict[str, object]:
    return {
        "format": REFEREE_FORMAT,
        "case_hash": case.case_hash,
        "case_reference": case.case_reference,
        "source_dialect": SOURCE_DIALECT,
        "environment_sha256": case.environment.environment_sha256,
        "statement": case.statement,
        "syntax_only": case.syntax_only,
    }


def _environment_document(environment: EnvironmentFile) -> dict[str, object]:
    return {
        "format": REFEREE_FORMAT,
        "environment_sha256": environment.environment_sha256,
        "session_parameters": environment.session_parameters,
        "setup_sql": list(environment.setup_sql),
    }


def _remove_stale(*, directory: Path, wanted: frozenset[str]) -> int:
    removed: int = 0
    for folder in (CASES_DIRECTORY, ENVIRONMENTS_DIRECTORY):
        for path in sorted((directory / folder).glob(f"*{JSON_SUFFIX}")):
            name: str = f"{folder}/{path.name}"
            hashed: bool = len(path.stem) == HASH_LENGTH and set(path.stem) <= HEX_DIGITS
            if hashed and name not in wanted:
                path.unlink()
                removed += 1
    return removed
