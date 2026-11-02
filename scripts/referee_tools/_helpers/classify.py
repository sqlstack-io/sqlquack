"""Classifying each referee case by Snowflake's recorded outcome and SQLQuack's parse."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from scripts.referee_tools.constants import (
    ACCEPTED_OUTCOME,
    BOTH_ACCEPT,
    BOTH_SYNTAX,
    CAUGHT_AT_PARSE,
    JSON_SUFFIX,
    MISSED,
    NOT_RECORDED,
    SYNTAX_ERRNO,
    SYNTAX_ERROR_TEXT,
    TOO_LENIENT,
    TOO_STRICT,
)
from scripts.referee_tools.models import Comparison, ParseVerdict, RefereeCase


def category(*, outcome: dict[str, Any] | None, verdict: ParseVerdict) -> str:
    """Which of the comparison categories a case falls in."""
    if outcome is None:
        return NOT_RECORDED
    if outcome["outcome"] == ACCEPTED_OUTCOME:
        return BOTH_ACCEPT if verdict.parses else TOO_STRICT
    if is_syntax_error(outcome):
        return TOO_LENIENT if verdict.parses else BOTH_SYNTAX
    return MISSED if verdict.parses else CAUGHT_AT_PARSE


def is_syntax_error(outcome: dict[str, Any]) -> bool:
    """Whether Snowflake rejected the statement as a syntax error rather than at binding."""
    error: dict[str, Any] = outcome.get("error") or {}
    message: str = str(error.get("message") or "").lower()
    return error.get("errno") == SYNTAX_ERRNO or SYNTAX_ERROR_TEXT in message


def comparisons(
    *, cases: list[RefereeCase], verdicts: dict[str, ParseVerdict], results: Path
) -> list[Comparison]:
    """Each case with its recorded outcome, if any, SQLQuack's parse and their category."""
    compared: list[Comparison] = []
    for case in cases:
        path: Path = results / f"{case.case_hash}{JSON_SUFFIX}"
        outcome: dict[str, Any] | None = (
            json.loads(path.read_text(encoding="utf-8")) if path.is_file() else None
        )
        verdict: ParseVerdict = verdicts[case.statement]
        compared.append(
            Comparison(
                case=case,
                outcome=outcome,
                verdict=verdict,
                category=category(outcome=outcome, verdict=verdict),
            )
        )
    return compared
