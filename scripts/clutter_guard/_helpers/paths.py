"""Path rules: where a tracked file may live and what it may be named."""

from __future__ import annotations

import re

from scripts.clutter_guard.constants import (
    AGENT_FILE,
    AGENT_NAMES_LOWER_CASE,
    DIALECT_SUITE_FILE_MINIMUM_PARTS,
    DIALECT_SUITES,
    DIALECTS,
    PATH_NOT_ALLOWED,
    PROCESS_NOTE_FILE,
    PROCESS_NOTE_PATTERNS,
    RUN_RECORD_NAMES,
    SOURCE_FILE_SUFFIXES,
    SUITE_FILE_MINIMUM_PARTS,
    SUITE_FOLDERS,
    SUITES_DIRECTORY,
    TOP_LEVEL_ENTRIES,
)
from scripts.clutter_guard.models import Finding


def path_findings(*, path: str) -> list[Finding]:
    """The path rules `path` breaks, each reported once against the path itself."""
    parts: list[str] = path.split("/")
    named: list[tuple[str, str | None]] = [
        (PATH_NOT_ALLOWED, _outside_layout(parts)),
        (AGENT_FILE, next((part for part in parts if _is_agent_name(part)), None)),
        (
            PROCESS_NOTE_FILE,
            next((part for part in _named_parts(parts) if _is_process_note(part)), None),
        ),
    ]
    return [
        Finding(path=path, line=0, clutter_class=clutter_class, snippet=snippet)
        for clutter_class, snippet in named
        if snippet is not None
    ]


def _outside_layout(parts: list[str]) -> str | None:
    if parts[0] not in TOP_LEVEL_ENTRIES:
        return parts[0]
    if parts[0] != SUITES_DIRECTORY:
        return None
    if len(parts) < SUITE_FILE_MINIMUM_PARTS or parts[1] not in SUITE_FOLDERS:
        return "/".join(parts[:2])
    if parts[1] in DIALECT_SUITES and (
        len(parts) < DIALECT_SUITE_FILE_MINIMUM_PARTS or parts[2] not in DIALECTS
    ):
        return "/".join(parts[:3])
    return None


def _named_parts(parts: list[str]) -> list[str]:
    return parts[:-1] if parts[-1].endswith(SOURCE_FILE_SUFFIXES) else parts


def _is_agent_name(part: str) -> bool:
    return part.lower() in AGENT_NAMES_LOWER_CASE


def _is_process_note(part: str) -> bool:
    name: str = part.lower()
    return name in RUN_RECORD_NAMES or any(
        re.search(pattern, name) is not None for pattern in PROCESS_NOTE_PATTERNS
    )
