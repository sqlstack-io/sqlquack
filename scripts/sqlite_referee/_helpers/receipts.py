"""Receipts, private raw records and the run audit of one referee run."""

from __future__ import annotations

import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path

from scripts.sqlite_referee.constants import (
    ACCEPTED,
    ENGINE,
    ENGINE_DIR,
    ENGINE_VERSION,
    FETCH_RECORD,
    LIBRARY_FILE,
    LIBRARY_KEY,
    OUTCOME_ACCEPTED,
    OUTCOME_ERROR,
    PARSER_CHECK,
    RECEIPT_FORMAT,
    RUN_AUDIT_FORMAT,
)
from scripts.sqlite_referee.exceptions import RefereeError
from scripts.sqlite_referee.models import Observation
from scripts.sqlite_referee.types import Row
from scripts.syntax_catalogue_sqlite._helpers.fetch import archive_path, member
from scripts.syntax_catalogue_sqlite.constants import (
    AMALGAMATION,
    AMALGAMATION_FILES,
    AMALGAMATION_PREFIX,
    ARCHIVES,
)


def canonical(value: object) -> str:
    """The canonical JSON text the hashes are taken over."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(text: str) -> str:
    """SHA-256 of UTF-8 text, in hex."""
    return hashlib.sha256(text.encode()).hexdigest()


def engine(cache: Path) -> tuple[Path, str]:
    """The pinned library and its SHA-256, refusing a library not built from the pinned archive."""
    archive: Path = archive_path(cache=cache, name=AMALGAMATION)
    pinned: str = ARCHIVES[AMALGAMATION][1]
    if hashlib.sha256(archive.read_bytes()).hexdigest() != pinned:
        raise RefereeError(f"{archive.name} is not the pinned amalgamation {pinned}")
    source: Path = cache / ENGINE_DIR / AMALGAMATION_FILES[0]
    if source.read_bytes() != member(archive=archive, name=AMALGAMATION_PREFIX + source.name):
        raise RefereeError(f"{source} is not the amalgamation's {source.name}")
    library: Path = cache / ENGINE_DIR / LIBRARY_FILE
    found: str = hashlib.sha256(library.read_bytes()).hexdigest()
    recorded: str = str(json.loads((cache / FETCH_RECORD).read_text())[LIBRARY_KEY])
    if found != recorded:
        raise RefereeError(f"{library} is {found}, not the library fetch.json recorded")
    return library, found


def now() -> str:
    """The current time, RFC 3339 in UTC."""
    return datetime.now(UTC).isoformat()


def raw_record(*, case: Row, observation: Observation, recorded_at: str) -> Row:
    """The private record of one case: the case as submitted and everything observed."""
    return {
        "case": case,
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
        "capture_method": case["capture"],
        "run_id": context["run_id"],
        "engine": ENGINE,
        "engine_version": ENGINE_VERSION,
        "engine_image_sha256": context["engine_sha256"],
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


def run_audit(*, run_id: str, engine: str, raw_digests: list[str], phases: dict[str, int]) -> Row:
    """The private audit of a run: what ran it, and the digest of every raw record in order."""
    return {
        "format": RUN_AUDIT_FORMAT,
        "run_id": run_id,
        "engine": ENGINE,
        "engine_version": ENGINE_VERSION,
        "engine_sha256": engine,
        "raw_records_sha256": digest(canonical(raw_digests)),
        "receipts": len(raw_digests),
        "phases": phases,
    }


def write_jsonl(*, path: Path, rows: list[Row]) -> None:
    """One canonical JSON object per line."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(canonical(row) + "\n" for row in rows))
