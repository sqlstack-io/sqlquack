"""Test types for recording pending cases."""

from __future__ import annotations

from dataclasses import dataclass

from sqlquack_record.recording.models import ScratchFunctions
from sqlquack_record.recording.types import SessionParameters

# What was stored for one case: the outcome, whether only a hash of the query ID is stored,
# whether the Arrow file matches the metadata hash, whether the stored rows equal the
# warehouse's Arrow table, and the recorded row count as JSON.
type RecordedFile = tuple[str, bool, bool, bool, str]


@dataclass(frozen=True)
class RecordTestCase:
    description: str
    statements: tuple[str, ...]
    failing: frozenset[str]
    batch_fails: bool
    parameters: tuple[tuple[tuple[str, bool | int | str], ...], ...]
    preexisting: tuple[str, ...]
    without_values: frozenset[str]
    expected_recorded: int
    expected_failures: int
    expected_log: tuple[str, ...]
    expected_files: tuple[RecordedFile, ...]
    with_functions: frozenset[str] = frozenset()


@dataclass(frozen=True)
class BrokenCaseTestCase:
    description: str
    statements: tuple[str, ...]
    broken: str
    expected_mention: str


@dataclass(frozen=True)
class ScratchTestCase:
    description: str
    setup_sql: tuple[str, ...]
    source_sql: str
    expected_functions: ScratchFunctions


@dataclass(frozen=True)
class ParametersTestCase:
    description: str
    parameters: SessionParameters
    expected_effective: dict[str, str]


@dataclass(frozen=True)
class DefaultsTestCase:
    description: str
    names: frozenset[str]
    expected_parameters: dict[str, str]
    expected_error: str
