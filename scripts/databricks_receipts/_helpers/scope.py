"""Which cases the referee can isolate in their own schema; the others are held before a run."""

from __future__ import annotations

import re

from scripts.databricks_receipts.constants import (
    ACCOUNT_SCOPE,
    CASE_HEAD,
    CASE_SCOPE_HEADS,
    STATEMENT_WORDS,
)
from scripts.databricks_receipts.types import Row


def held_reason(sql: str) -> str | None:
    """Why a case cannot run in its own schema, or None when it can."""
    for pattern, reason in ACCOUNT_SCOPE:
        if pattern.search(sql):
            return reason
    head: re.Match[str] | None = CASE_HEAD.match(sql)
    word: str = (head.group(2) or "") if head else ""
    if STATEMENT_WORDS.match(word) and word not in CASE_SCOPE_HEADS:
        return f"begins with {word}, which the referee cannot place in the case schema"
    return None


def held_cases(cases: list[Row]) -> list[Row]:
    """The held cases, each with its reason, in the order given."""
    found: list[Row] = []
    for case in cases:
        reason: str | None = held_reason(str(case["sql"]))
        if reason is not None:
            found.append({"case_id": case["case_id"], "sql": case["sql"], "reason": reason})
    return found
