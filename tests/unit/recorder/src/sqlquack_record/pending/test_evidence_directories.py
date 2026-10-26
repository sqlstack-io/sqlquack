"""Resolving the evidence directories from the working directory."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.recorder.src.sqlquack_record.pending._test_types import (
    EvidenceDirectoriesTestCase,
    MissingPendingTestCase,
)
from tests.unit.recorder.src.sqlquack_record.pending.helpers import (
    missing_directory_references,
    resolved_directories,
)


@pytest.mark.parametrize(
    "test_case",
    (
        EvidenceDirectoriesTestCase(
            description="from the repository root the defaults are its evidence directories",
            marker_at="repo",
            working_directory="repo",
            pending=None,
            results=None,
            expected_directories=("repo/evidence/pending", "repo/evidence/results"),
        ),
        EvidenceDirectoriesTestCase(
            description="from a subdirectory the defaults still resolve at the repository root",
            marker_at="repo",
            working_directory="repo/recorder",
            pending=None,
            results=None,
            expected_directories=("repo/evidence/pending", "repo/evidence/results"),
        ),
        EvidenceDirectoriesTestCase(
            description="outside a repository without explicit directories resolution fails",
            marker_at=None,
            working_directory="elsewhere",
            pending=None,
            results=None,
            expected_directories=None,
        ),
        EvidenceDirectoriesTestCase(
            description="explicit directories are used as given, even outside a repository",
            marker_at=None,
            working_directory="elsewhere",
            pending="queue",
            results="store",
            expected_directories=("queue", "store"),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_working_directory_when_resolving_evidence_then_defaults_follow_the_repository(
    test_case: EvidenceDirectoriesTestCase, tmp_path: Path
) -> None:
    resolved: tuple[str, str] | None = resolved_directories(
        root=tmp_path,
        marker_at=test_case.marker_at,
        working_directory=test_case.working_directory,
        pending=test_case.pending,
        results=test_case.results,
    )

    assert resolved == test_case.expected_directories


@pytest.mark.parametrize(
    "test_case",
    (
        MissingPendingTestCase(
            description="a missing pending directory fails instead of reporting no cases",
            expected_references=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_missing_pending_directory_when_loading_then_it_fails_instead_of_finding_none(
    test_case: MissingPendingTestCase, tmp_path: Path
) -> None:
    assert missing_directory_references(tmp_path) == test_case.expected_references
