"""Test types for loading pending case files."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class LoadPendingTestCase:
    description: str
    file_name: str
    replace: str
    with_text: str
    expected_references: tuple[str, ...] | None


@dataclass(frozen=True)
class EvidenceDirectoriesTestCase:
    description: str
    marker_at: str | None
    working_directory: str
    pending: str | None
    results: str | None
    expected_directories: tuple[str, str] | None


@dataclass(frozen=True)
class MissingPendingTestCase:
    description: str
    expected_references: tuple[str, ...] | None
