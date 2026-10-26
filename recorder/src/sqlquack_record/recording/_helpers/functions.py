"""A case's project functions under names unique to the case, and the statements dropping them."""

from __future__ import annotations

import re

from sqlquack_record.recording.constants import (
    ARGUMENT_SEPARATOR,
    CREATE_FUNCTION_PATTERN,
    PARENTHESIS_DEPTH,
    SCRATCH_FUNCTION_SUFFIX,
    SCRATCH_HASH_LENGTH,
)
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import ScratchFunctions


def scratch_functions(
    *, setup_sql: tuple[str, ...], source_sql: str, case_hash: str, run_nonce: str
) -> ScratchFunctions:
    """The setup and case SQL with every function renamed `<name>__sqlquack_<hash>_<nonce>`."""
    suffix: str = f"{SCRATCH_FUNCTION_SUFFIX}{case_hash[:SCRATCH_HASH_LENGTH]}_{run_nonce}"
    signatures: list[tuple[str, str]] = [_signature(statement) for statement in setup_sql]
    renamed_setup: list[str] = list(setup_sql)
    renamed_source: str = source_sql
    for name, _ in signatures:
        call: re.Pattern[str] = re.compile(rf"\b{re.escape(name)}(\s*\()", re.IGNORECASE)
        renamed_setup = [call.sub(rf"{name}{suffix}\1", text) for text in renamed_setup]
        renamed_source = call.sub(rf"{name}{suffix}\1", renamed_source)
    return ScratchFunctions(
        setup_sql=tuple(renamed_setup),
        source_sql=renamed_source,
        drop_sql=tuple(
            f"DROP FUNCTION IF EXISTS {name}{suffix}({types})" for name, types in signatures
        ),
    )


def _signature(statement: str) -> tuple[str, str]:
    """The name and comma-separated argument types of a `CREATE FUNCTION` statement."""
    match: re.Match[str] | None = re.match(CREATE_FUNCTION_PATTERN, statement)
    if match is None:
        raise RecorderError(f"setup statement is not CREATE FUNCTION: {statement[:60]}")
    types: list[str] = [
        argument.strip().split(None, 1)[1]
        for argument in _top_level_split(match.group("arguments"))
        if argument.strip()
    ]
    return match.group("name"), ", ".join(types)


def _top_level_split(text: str) -> list[str]:
    """`text` split at commas outside parentheses."""
    parts: list[str] = [""]
    depth: int = 0
    for character in text:
        depth += PARENTHESIS_DEPTH.get(character, 0)
        if character == ARGUMENT_SEPARATOR and depth == 0:
            parts.append("")
            continue
        parts[-1] += character
    return parts
