"""Writing referee outcomes and the function catalogue, append-only."""

from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path

from sqlquack_record.recording.constants import RECORDER_NAME, RECORDER_VERSION
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import ColumnDescription, WarehouseFailure
from sqlquack_record.referee.constants import (
    ACCEPTED_OUTCOME,
    BUILTIN_COLUMN,
    BUILTIN_VALUE,
    CATALOGUE_COLUMNS,
    REFEREE_FORMAT,
    REJECTED_OUTCOME,
    SYNTAX_ERROR_TEXT,
)
from sqlquack_record.referee.models import RefereeCase, RefereeContext, RefereeOutcome


def write_outcome(
    *, directory: Path, case: RefereeCase, outcome: RefereeOutcome, context: RefereeContext
) -> None:
    """Store `outcome` for `case`; a syntax-only case keeps only a syntax error's message."""
    document: dict[str, object] = {
        "format": REFEREE_FORMAT,
        "case_hash": case.case_hash,
        "environment_sha256": case.environment_sha256,
        "syntax_only": case.syntax_only,
        "outcome": REJECTED_OUTCOME if outcome.error is not None else ACCEPTED_OUTCOME,
        "recorder": f"{RECORDER_NAME} {RECORDER_VERSION}",
        "connector": context.connector,
        "warehouse_version": context.warehouse_version,
        "recorded_at": context.recorded_at,
        "effective_parameters": dict(context.session_parameters),
        "error": _failure(failure=outcome.error, case=case, context=context),
        "columns": None
        if outcome.columns is None
        else [_column(column) for column in outcome.columns],
        "describe_error": _failure(failure=outcome.describe_error, case=case, context=context),
    }
    create_exclusively(
        path=directory / f"{case.case_hash}.json",
        content=(json.dumps(document, indent=2, sort_keys=True) + "\n").encode("utf-8"),
    )


def write_catalogue(
    *, directory: Path, rows: list[dict[str, str]], context: RefereeContext
) -> str | None:
    """Store the built-in function catalogue under the hash of its rows; absent if stored."""
    functions: list[dict[str, str]] = sorted(
        (_function(row) for row in rows if row.get(BUILTIN_COLUMN) == BUILTIN_VALUE),
        key=lambda function: (function["name"], function["arguments"]),
    )
    digest: str = hashlib.sha256(
        json.dumps(functions, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    path: Path = directory / f"{digest}.json"
    if path.exists():
        return None
    document: dict[str, object] = {
        "format": REFEREE_FORMAT,
        "catalogue_sha256": digest,
        "recorder": f"{RECORDER_NAME} {RECORDER_VERSION}",
        "connector": context.connector,
        "warehouse_version": context.warehouse_version,
        "recorded_at": context.recorded_at,
        "functions": functions,
    }
    directory.mkdir(parents=True, exist_ok=True)
    create_exclusively(
        path=path,
        content=(json.dumps(document, indent=2, sort_keys=True) + "\n").encode("utf-8"),
    )
    return digest


def scrubbed(*, message: str, identity: tuple[tuple[str, str], ...]) -> str:
    """`message` with the session's account, user, role and object names replaced by kind."""
    for kind, name in sorted(identity, key=lambda pair: -len(pair[1])):
        if name:
            message = re.sub(
                rf"(?<![A-Za-z0-9_$]){re.escape(name)}(?![A-Za-z0-9_$])",
                f"<{kind}>",
                message,
                flags=re.IGNORECASE,
            )
    return message


def _failure(
    *, failure: WarehouseFailure | None, case: RefereeCase, context: RefereeContext
) -> dict[str, object] | None:
    if failure is None:
        return None
    message: str | None = failure.message
    if message is not None and case.syntax_only and SYNTAX_ERROR_TEXT not in message.lower():
        message = None
    if message is not None:
        message = scrubbed(message=message, identity=context.identity)
    return {"errno": failure.errno, "sqlstate": failure.sqlstate, "message": message}


def _column(column: ColumnDescription) -> dict[str, object]:
    return {
        "name": column.name,
        "type_code": column.type_code,
        "type_name": column.type_name,
        "precision": column.precision,
        "scale": column.scale,
        "internal_size": column.internal_size,
        "nullable": column.nullable,
    }


def _function(row: dict[str, str]) -> dict[str, str]:
    return {name: row.get(name, "") for name in CATALOGUE_COLUMNS}


def create_exclusively(*, path: Path, content: bytes) -> None:
    """Write `content` to a new file at `path`; an existing file is never replaced."""
    try:
        descriptor: int = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    except FileExistsError as error:
        raise RecorderError(
            f"{path.name} already exists; referee outcomes are append-only"
        ) from error
    with os.fdopen(descriptor, "wb") as handle:
        handle.write(content)
