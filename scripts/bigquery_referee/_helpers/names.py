"""The run's names: token substitution, message normalisation and the leak check."""

from __future__ import annotations

import os
import re
from collections.abc import Callable

from scripts.bigquery_referee.constants import (
    ADDRESS_PATTERN,
    CASE_SUFFIX,
    EMAIL_MARK,
    EMAIL_PATTERN,
    JOB_ID_PATTERN,
    JOB_MARK,
    NEVER_TOUCHED_PREFIXES,
    POSITION_PATTERN,
    PROJECT_ID_PATTERN,
    PROJECT_NUMBER_PATTERN,
    PROJECT_VARIABLE,
    READ_BEFORE_MARK,
    READ_BEFORE_PATTERN,
    RUN_DATASET_PREFIX,
    SECOND_DATASET_SUFFIX,
    SNAPSHOT_MARK,
    SNAPSHOT_PATTERN,
    TOKEN_DASHED_PROJECT,
    TOKEN_DATASET,
    TOKEN_DATASET2,
    TOKEN_PATTERN,
    TOKEN_PROJECT,
    URL_MARK,
    URL_PATTERN,
)
from scripts.bigquery_referee.exceptions import RefereeError
from scripts.bigquery_referee.models import RunNames

type _Span = tuple[int, int, int]


def run_project() -> str:
    """The sandbox project from its environment variable; no file names it."""
    project: str = os.environ.get(PROJECT_VARIABLE, "").strip()
    if not re.match(PROJECT_ID_PATTERN, project):
        raise RefereeError(f"set {PROJECT_VARIABLE} to the sandbox project id")
    return project


def run_names(*, run_id: str, project: str) -> RunNames:
    """The project and the two datasets of a run: lower case, no quoting needed."""
    dataset: str = RUN_DATASET_PREFIX + re.sub(r"[^a-z0-9]", "_", run_id.lower())
    if dataset.startswith(NEVER_TOUCHED_PREFIXES):
        raise RefereeError(f"refused a run dataset name: {dataset}")
    return RunNames(project=project, dataset=dataset, dataset2=dataset + SECOND_DATASET_SUFFIX)


def case_names(*, names: RunNames, index: int) -> RunNames:
    """A case's own two datasets, named after the run's; created and dropped for that case."""
    dataset: str = names.dataset + CASE_SUFFIX.format(index=index)
    return RunNames(
        project=names.project, dataset=dataset, dataset2=dataset + SECOND_DATASET_SUFFIX
    )


def _by_token(names: RunNames) -> dict[str, str]:
    """Each token's value; the dashed project token masks back as the project token."""
    return {
        TOKEN_PROJECT: names.project,
        TOKEN_DATASET: names.dataset,
        TOKEN_DATASET2: names.dataset2,
        TOKEN_DASHED_PROJECT: names.project,
    }


def substitute(*, sql: str, names: RunNames) -> str:
    """The case text with each whole token replaced, inside backquotes as well as outside."""
    values: dict[str, str] = _by_token(names)
    return re.sub(TOKEN_PATTERN, lambda match: values[match.group(0)], sql)


def _spans(*, line: str, values: dict[str, str]) -> list[_Span]:
    """Each token of a line: its start, its length and the length of the name replacing it."""
    return [
        (match.start(), len(match.group(0)), len(values[match.group(0)]))
        for match in re.finditer(TOKEN_PATTERN, line)
    ]


def _committed_column(*, column: int, spans: list[_Span]) -> int:
    """The zero-based column in the committed line of a column in the submitted line."""
    shift: int = 0
    for start, token, value in spans:
        if column < start + shift:
            break
        if column < start + shift + value:
            return start
        shift += value - token
    return column - shift


def positions(*, sql: str, names: RunNames, line_offset: int = 0) -> Callable[[str], str]:
    """Rewrite each `[line:column]` of the submitted text as the committed text's position."""
    values: dict[str, str] = _by_token(names)
    lines: list[list[_Span]] = [_spans(line=line, values=values) for line in sql.split("\n")]

    def committed(match: re.Match[str]) -> str:
        line: int = int(match.group(1)) - line_offset
        if not 1 <= line <= len(lines):
            return match.group(0)
        column: int = _committed_column(column=int(match.group(2)) - 1, spans=lines[line - 1])
        return f"[{line}:{column + 1}]"

    return lambda text: re.sub(POSITION_PATTERN, committed, text)


def normalise(*, text: str, names: RunNames, position: Callable[[str], str] | None = None) -> str:
    """Engine text with the run's names as tokens, ids, URLs and emails masked, positions fixed."""
    found: str = re.sub(URL_PATTERN, URL_MARK, text)
    found = re.sub(EMAIL_PATTERN, EMAIL_MARK, found)
    found = re.sub(JOB_ID_PATTERN, JOB_MARK, found)
    found = re.sub(SNAPSHOT_PATTERN, SNAPSHOT_MARK, found)
    found = re.sub(PROJECT_NUMBER_PATTERN, TOKEN_PROJECT, found)
    found = re.sub(READ_BEFORE_PATTERN, READ_BEFORE_MARK, found)
    for token, value in sorted(_by_token(names).items(), key=lambda pair: -len(pair[1])):
        found = re.sub(
            rf"(?<![A-Za-z0-9_-]){re.escape(value)}(?![A-Za-z0-9_-])", token, found, flags=re.I
        )
    return position(found) if position else found


def leaks(*, text: str, names: RunNames) -> bool:
    """Whether text holds the project (by id or number), a run dataset, job id, URL or address."""
    lower: str = text.lower()
    return (
        any(name.lower() in lower for name in (names.project, names.dataset, names.dataset2))
        or re.search(URL_PATTERN, text) is not None
        or re.search(EMAIL_PATTERN, text) is not None
        or re.search(JOB_ID_PATTERN, text) is not None
        or re.search(ADDRESS_PATTERN, text) is not None
        or re.search(PROJECT_NUMBER_PATTERN, text) is not None
    )
