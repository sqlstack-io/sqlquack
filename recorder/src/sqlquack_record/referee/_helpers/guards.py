"""What a referee session may run: empty temporary tables, and single queries to compile."""

from __future__ import annotations

import re

from sqlquack_record.referee.constants import (
    FORBIDDEN_TEXT,
    LEADING_COMMENTS,
    LIST_CLOSE,
    LIST_OPEN,
    QUERY_START,
    QUOTED_NAME_DELIMITER,
    SETUP_FORBIDDEN_TEXT,
    SETUP_PATTERN,
    SETUP_QUERY_WORD,
    STATEMENT_SEPARATOR,
)


def setup_refusal(statement: str) -> str | None:
    """Why a setup statement may not run; only `CREATE TEMPORARY TABLE <name> (...)` may."""
    start: re.Match[str] | None = SETUP_PATTERN.match(statement)
    if (
        start is None
        or STATEMENT_SEPARATOR in statement
        or any(text in statement.upper() for text in SETUP_FORBIDDEN_TEXT)
        or SETUP_QUERY_WORD.search(statement) is not None
        or not _column_list_ends_statement(statement=statement, list_start=start.end())
    ):
        return "a setup statement must create one temporary table from a column list"
    return None


def _column_list_ends_statement(*, statement: str, list_start: int) -> bool:
    """Whether the column list opened at `list_start` closes at the statement's last character."""
    depth: int = 1
    quoted: bool = False
    body: str = statement.rstrip()
    for position in range(list_start, len(body)):
        character: str = body[position]
        if character == QUOTED_NAME_DELIMITER:
            quoted = not quoted
        elif not quoted and character in (LIST_OPEN, LIST_CLOSE):
            depth += 1 if character == LIST_OPEN else -1
            if depth == 0:
                return position == len(body) - 1
    return False


def statement_refusal(statement: str) -> str | None:
    """Why a case statement may not be compiled; only one query without system calls may."""
    comments: re.Match[str] | None = LEADING_COMMENTS.match(statement)
    body: str = (
        statement[0 if comments is None else comments.end() :].rstrip().rstrip(STATEMENT_SEPARATOR)
    )
    if QUERY_START.match(body) is None:
        return "a referee statement must be a query"
    for text in FORBIDDEN_TEXT:
        if text in body.upper():
            return f"a referee statement may not contain {text}"
    return None
