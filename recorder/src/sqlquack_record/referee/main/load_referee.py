"""Load the pending referee cases and environments the referee pending script wrote."""

from __future__ import annotations

from pathlib import Path

from sqlquack_record.referee._helpers.parsing import is_hash_file, parse_case, parse_environment
from sqlquack_record.referee.constants import (
    CASES_DIRECTORY,
    CATALOGUE_REQUEST,
    ENVIRONMENTS_DIRECTORY,
)
from sqlquack_record.referee.models import RefereePending


def load_referee(directory: Path) -> RefereePending | None:
    """Every pending referee case and environment in `directory`; absent when it has none."""
    if not directory.is_dir():
        return None
    cases_directory: Path = directory / CASES_DIRECTORY
    environments_directory: Path = directory / ENVIRONMENTS_DIRECTORY
    return RefereePending(
        cases=tuple(
            sorted(
                (parse_case(path) for path in _hash_files(cases_directory)),
                key=lambda case: case.case_reference,
            )
        ),
        environments=tuple(parse_environment(path) for path in _hash_files(environments_directory)),
        catalogue=(directory / CATALOGUE_REQUEST).is_file(),
    )


def _hash_files(directory: Path) -> list[Path]:
    if not directory.is_dir():
        return []
    return [path for path in sorted(directory.glob("*.json")) if is_hash_file(path)]
