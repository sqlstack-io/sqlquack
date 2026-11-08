"""Receipts, private raw records and the run audit of one referee run."""

from __future__ import annotations

import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path

from scripts.bigquery_referee.constants import (
    ACCEPTED,
    CAPTURE,
    ENGINE,
    ENGINE_IDENTITY,
    ENGINE_IMAGE_SHA256,
    ENGINE_VERSION,
    OUTCOME_ACCEPTED,
    OUTCOME_ERROR,
    PARSER_CHECK,
    RECEIPT_FORMAT,
    RUN_AUDIT_FORMAT,
)
from scripts.bigquery_referee.models import Observation, RunNames
from scripts.bigquery_referee.types import Row


def canonical(value: object) -> str:
    """The canonical JSON text the hashes are taken over."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(text: str) -> str:
    """SHA-256 of UTF-8 text, in hex."""
    return hashlib.sha256(text.encode()).hexdigest()


def now() -> str:
    """The current time, RFC 3339 in UTC."""
    return datetime.now(UTC).isoformat()


def raw_record(*, case: Row, submitted: str, observation: Observation, recorded_at: str) -> Row:
    """The private record of one case: the case, the text submitted and everything observed."""
    return {
        "case": case,
        "submitted_sql": submitted,
        "observation": observation.__dict__,
        "recorded_at": recorded_at,
    }


def receipt(*, case: Row, observation: Observation, context: Row) -> Row:
    """The published receipt of one case."""
    return {
        "format": RECEIPT_FORMAT,
        "case_id": case["case_id"],
        "sql_sha256": case["sql_sha256"],
        "setup_sha256": case["setup_sha256"],
        "session_sha256": case["session_sha256"],
        "session_parameters": case["session"],
        "capture_method": CAPTURE,
        "run_id": context["run_id"],
        "engine": ENGINE,
        "engine_version": ENGINE_VERSION,
        "engine_image_sha256": ENGINE_IMAGE_SHA256,
        "engine_code": observation.engine_code,
        "parser_check": PARSER_CHECK,
        "parse_only_sqlstate": observation.parse_only_code,
        "recorded_at": context["recorded_at"],
        "outcome": OUTCOME_ACCEPTED if observation.phase == ACCEPTED else OUTCOME_ERROR,
        "sqlstate": None,
        "phase": observation.phase,
        "phase_basis": observation.basis,
        "message": observation.message,
        "raw_record_sha256": context["raw_record_sha256"],
        "run_audit_sha256": context["run_audit_sha256"],
        "packet_sha256": context["packet_sha256"],
    }


def run_audit(
    *, run_id: str, names: RunNames, state: Row, raw_digests: list[str], counts: Row
) -> Row:
    """The private audit of a run: project, datasets, state and every raw record's digest."""
    return {
        "format": RUN_AUDIT_FORMAT,
        "run_id": run_id,
        "engine": ENGINE,
        "engine_version": ENGINE_VERSION,
        "engine_identity": ENGINE_IDENTITY,
        "project": names.project,
        "datasets": [names.dataset, names.dataset2],
        "state": state,
        "raw_records_sha256": digest(canonical(raw_digests)),
        "receipts": len(raw_digests),
        **counts,
    }


def write_jsonl(*, path: Path, rows: list[Row]) -> None:
    """One canonical JSON object per line."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(canonical(row) + "\n" for row in rows))
