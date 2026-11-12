"""Content rules: what a tracked text file may say."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.clutter_guard.constants import (
    AFTER_ASSIGNMENT_PATTERN,
    ALLOWED_SECRET_NAMES,
    BEFORE_ENVIRONMENT_READ_PATTERN,
    BEFORE_LINE_START_ASSIGNMENT_PATTERN,
    BINARY_PROBE_BYTES,
    CLASSES_NOT_COUNTED_IN_RECORD_DATA,
    CONTENT_PATTERNS,
    ENVIRONMENT_VARIABLE_PROSE_PATTERN,
    PULL_REQUEST_TRIGGER_PATTERN,
    RECORD_DATA_FIELD_PATTERN,
    SECRET_PART_PAIRS,
    SECRET_PART_SEARCH_PATTERN,
    SECRET_PARTS,
    SECRET_VARIABLE_NAME,
    SNIPPET_CONTEXT,
    SNIPPET_LIMIT,
    VARIABLE_NAME_CHARACTERS,
    VARIABLE_NAME_PATTERN,
    WORKFLOW_SECRET_NAMES,
)
from scripts.clutter_guard.models import Finding


def file_findings(*, root: Path, path: str) -> list[Finding]:
    """The content rules the tracked file `path` breaks; none for a binary or missing file."""
    file: Path = root / path
    if not file.is_file():
        return []
    data: bytes = file.read_bytes()
    if b"\0" in data[:BINARY_PROBE_BYTES]:
        return []
    return text_findings(path=path, text=data.decode("utf-8", errors="replace"))


def text_findings(*, path: str, text: str) -> list[Finding]:
    """One finding per line and class for every content rule `text` breaks."""
    spans: list[tuple[str, int, int]] = []
    for clutter_class, pattern in CONTENT_PATTERNS:
        spans.extend(_pattern_spans(text=text, clutter_class=clutter_class, pattern=pattern))
    spans.extend(
        (SECRET_VARIABLE_NAME, *span)
        for span in _secret_variables(text)
        if not _workflow_secret(path=path, name=text[span[0] : span[1]], text=text)
    )
    found: dict[tuple[int, str], Finding] = {}
    line: int = 1
    counted_to: int = 0
    for clutter_class, start, end in sorted(spans, key=lambda span: span[1]):
        line += text.count("\n", counted_to, start)
        counted_to = start
        if (line, clutter_class) not in found:
            found[(line, clutter_class)] = Finding(
                path=path,
                line=line,
                clutter_class=clutter_class,
                snippet=_snippet(text=text, start=start, end=end),
            )
    return list(found.values())


def _pattern_spans(*, text: str, clutter_class: str, pattern: str) -> list[tuple[str, int, int]]:
    """The spans `pattern` matches in `text`, less record data for the classes that spare it."""
    spared: bool = clutter_class in CLASSES_NOT_COUNTED_IN_RECORD_DATA
    return [
        (clutter_class, match.start(), match.end())
        for match in re.finditer(pattern, text)
        if not (spared and _is_record_data(text=text, start=match.start(), end=match.end()))
    ]


def _is_record_data(*, text: str, start: int, end: int) -> bool:
    """Whether the match sits inside a SQL field's value or an identifier-valued `name` field."""
    line_start: int = text.rfind("\n", 0, start) + 1
    line_end: int = text.find("\n", end)
    line: str = text[line_start : len(text) if line_end < 0 else line_end]
    return any(
        field.start(field.lastindex or 0) <= start - line_start
        and end - line_start <= field.end(field.lastindex or 0)
        for field in re.finditer(RECORD_DATA_FIELD_PATTERN, line)
    )


def _secret_variables(text: str) -> list[tuple[int, int]]:
    """The spans of secret-ish names that `text` reads or sets as process settings."""
    spans: set[tuple[int, int]] = set()
    for match in re.finditer(SECRET_PART_SEARCH_PATTERN, text):
        start: int = match.start()
        while start > 0 and text[start - 1] in VARIABLE_NAME_CHARACTERS:
            start -= 1
        end: int = match.end()
        while end < len(text) and text[end] in VARIABLE_NAME_CHARACTERS:
            end += 1
        if (start, end) not in spans and _is_secret_variable(text=text, start=start, end=end):
            spans.add((start, end))
    return sorted(spans)


def _is_secret_variable(*, text: str, start: int, end: int) -> bool:
    name: str = text[start:end]
    bounded: bool = not (start > 0 and text[start - 1].isalnum()) and not (
        end < len(text) and text[end].isalnum()
    )
    if not bounded or not re.fullmatch(VARIABLE_NAME_PATTERN, name):
        return False
    parts: list[str] = name.split("_")
    secret_ish: bool = any(part in SECRET_PARTS for part in parts) or any(
        pair in SECRET_PART_PAIRS for pair in zip(parts, parts[1:], strict=False)
    )
    if not secret_ish or name in ALLOWED_SECRET_NAMES:
        return False
    line_start: int = text.rfind("\n", 0, start) + 1
    line_end: int = text.find("\n", end)
    line_end = len(text) if line_end < 0 else line_end
    before: str = text[line_start:start]
    after: str = text[end:line_end]
    return (
        re.search(BEFORE_ENVIRONMENT_READ_PATTERN, before) is not None
        or (
            re.match(BEFORE_LINE_START_ASSIGNMENT_PATTERN, before) is not None
            and re.match(AFTER_ASSIGNMENT_PATTERN, after) is not None
        )
        or re.search(ENVIRONMENT_VARIABLE_PROSE_PATTERN, text[line_start:line_end]) is not None
    )


def _workflow_secret(*, path: str, name: str, text: str) -> bool:
    """Whether `name` is a secret only `path` may read, and `path` has no pull request trigger."""
    return (
        WORKFLOW_SECRET_NAMES.get(name) == path
        and re.search(PULL_REQUEST_TRIGGER_PATTERN, text) is None
    )


def _snippet(*, text: str, start: int, end: int) -> str:
    line_start: int = text.rfind("\n", 0, start) + 1
    line_end: int = text.find("\n", end)
    left: int = max(line_start, start - SNIPPET_CONTEXT)
    right: int = min(len(text) if line_end < 0 else line_end, end + SNIPPET_CONTEXT)
    snippet: str = " ".join(text[left:right].split())
    return snippet[:SNIPPET_LIMIT]
