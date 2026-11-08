"""Receipts, private raw records and the run audit of one referee run."""

from __future__ import annotations

import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path

from scripts.databricks_receipts._helpers.rows import data_array
from scripts.databricks_receipts.constants import (
    ACCEPTED,
    CAPTURE,
    ENGINE,
    ENGINE_VERSION,
    OUTCOME_ACCEPTED,
    OUTCOME_ERROR,
    PARSER_CHECK,
    RECEIPT_FORMAT,
    RUN_AUDIT_FORMAT,
)
from scripts.databricks_receipts.exceptions import RefereeError
from scripts.databricks_receipts.models import Observation
from scripts.databricks_receipts.types import Row


def canonical(value: object) -> str:
    """The canonical JSON text the hashes are taken over."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(text: str) -> str:
    """SHA-256 of UTF-8 text, in hex."""
    return hashlib.sha256(text.encode()).hexdigest()


def version_of(reply: Row) -> Row:
    """current_version() as the warehouse answered it, refusing any release but the pinned one."""
    rows: list[list[str]] = data_array(reply)
    version: Row = json.loads(rows[0][0])
    if version.get("dbsql_version") != ENGINE_VERSION:
        raise RefereeError(f"the warehouse runs {version}, not Databricks SQL {ENGINE_VERSION}")
    return version


def now() -> str:
    """The current time, RFC 3339 in UTC."""
    return datetime.now(UTC).isoformat()


def raw_record(*, case: Row, observation: Observation, version: Row, recorded_at: str) -> Row:
    """The private record of one case: the case as submitted and everything observed."""
    return {
        "case": case,
        "observation": observation.__dict__,
        "current_version": version,
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
        "query_id_sha256": digest(str(observation.statement_id)),
        "engine": ENGINE,
        "engine_version": ENGINE_VERSION,
        "engine_image_sha256": context["engine_sha256"],
        "engine_code": observation.engine_code,
        "parser_check": PARSER_CHECK,
        "parse_only_sqlstate": observation.parse_only_code,
        "recorded_at": context["recorded_at"],
        "outcome": OUTCOME_ACCEPTED if observation.phase == ACCEPTED else OUTCOME_ERROR,
        "sqlstate": observation.engine_code,
        "phase": observation.phase,
        "phase_basis": observation.basis,
        "message": observation.message,
        "raw_record_sha256": context["raw_record_sha256"],
        "run_audit_sha256": context["run_audit_sha256"],
        "packet_sha256": context["packet_sha256"],
    }


def run_audit(*, run_id: str, versions: list[Row], raw_digests: list[str], phases: Row) -> Row:
    """The private audit of a run: the releases that answered and every raw record's digest."""
    return {
        "format": RUN_AUDIT_FORMAT,
        "run_id": run_id,
        "engine": ENGINE,
        "engine_version": ENGINE_VERSION,
        "current_versions": versions,
        "raw_records_sha256": digest(canonical(raw_digests)),
        "receipts": len(raw_digests),
        "phases": phases,
    }


def write_jsonl(*, path: Path, rows: list[Row]) -> None:
    """One canonical JSON object per line."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(canonical(row) + "\n" for row in rows))
