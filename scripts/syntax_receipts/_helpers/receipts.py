"""Building receipts, refusals and input rows from bound observations."""

from __future__ import annotations

import re

from scripts.syntax_receipts._helpers.hashing import canonical_json, digest
from scripts.syntax_receipts.constants import (
    CAPTURE_EXPLAIN,
    COMPILATION_PREFIX,
    MISSING_OBJECT,
    MISSING_OBJECT_MESSAGE,
    OTHER_REASON,
    REASON_BY_ERRNO,
    RECEIPT_FORMAT,
    RUNTIME_ERRNO_FLOOR,
    SYNTAX_ERRNO,
    UNCATEGORIZED_PHASES,
    UNCAUGHT_PREFIX,
    UNSUPPORTED_ERRNO,
    UNSUPPORTED_PREFIX,
)
from scripts.syntax_receipts.exceptions import RowRefused
from scripts.syntax_receipts.models import Observation
from scripts.syntax_receipts.types import Row


def classify(*, errno: int | None, message: str) -> tuple[str, str] | None:
    """The phase of a Snowflake error, or None when the recorded fields do not establish one."""
    if message.startswith(UNCAUGHT_PREFIX):
        return "runtime_error", "uncaught exception raised while the block ran"
    if errno == UNSUPPORTED_ERRNO and message.startswith(UNSUPPORTED_PREFIX):
        return "unsupported_feature", "errno 2 Unsupported feature"
    if errno is not None and errno >= RUNTIME_ERRNO_FLOOR:
        return "runtime_error", f"errno {errno} is an execution-time code"
    compiled: bool = message.lower().startswith(COMPILATION_PREFIX.lower())
    if errno == SYNTAX_ERRNO and compiled:
        return "syntax_error", "errno 1003 SQL compilation syntax error"
    if errno is not None and compiled:
        return "semantic_error", f"errno {errno} compilation error after parsing"
    return None


def reason_category(*, errno: int | None, message: str) -> tuple[str, str]:
    """Snowflake's reason for a non-syntax error: missing object, option or value, type, other."""
    if errno in REASON_BY_ERRNO:
        return REASON_BY_ERRNO[errno], f"errno {errno}"
    if re.search(MISSING_OBJECT_MESSAGE, message, re.IGNORECASE):
        return MISSING_OBJECT, "message names a missing object"
    return OTHER_REASON, f"errno {errno} has no more specific reason"


def receipt(*, observation: Observation, extra: Row) -> Row:
    """The receipt for `observation`, refused when its error phase is not established."""
    phase, basis = _phase(observation=observation)
    reason: Row = {}
    if phase not in UNCATEGORIZED_PHASES:
        category, category_basis = reason_category(
            errno=observation.errno, message=observation.message
        )
        reason = {"reason_category": category, "reason_category_basis": category_basis}
    return {
        "format": RECEIPT_FORMAT,
        "case_id": observation.case_id,
        "packet_sha256": observation.packet_sha,
        "sql_sha256": observation.sql_sha,
        "setup_sha256": digest(canonical_json(observation.setup)),
        "session_sha256": digest(canonical_json(observation.session)),
        "session_parameters": observation.session,
        "run_id": observation.run_id,
        "capture_method": observation.method,
        "outcome": "accepted" if observation.accepted else "error",
        "errno": observation.errno,
        "sqlstate": observation.sqlstate,
        "phase": phase,
        "phase_basis": basis,
        "raw_record_sha256": digest(observation.raw_bytes),
        "run_audit_sha256": observation.audit_sha,
        **reason,
        **extra,
    }


def input_row(*, sql: str, setup: list[str]) -> Row:
    """The exact input a receipt's SQL hash names."""
    sha: str = digest(sql.encode())
    return {"sql_sha256": sha, "sql": sql, "setup": setup}


def refusal(*, run_id: str, row_id: str, reason: str) -> Row:
    """A row, or with row id `*` a whole run, that did not become a receipt."""
    return {"run_id": run_id, "row_id": row_id, "reason": reason}


def _phase(*, observation: Observation) -> tuple[str, str]:
    if observation.accepted:
        return "accepted", "statement compiled" if observation.method == CAPTURE_EXPLAIN else "ran"
    classified: tuple[str, str] | None = classify(
        errno=observation.errno, message=observation.message
    )
    if classified is None:
        raise RowRefused(f"error phase not established (errno {observation.errno})")
    return classified
