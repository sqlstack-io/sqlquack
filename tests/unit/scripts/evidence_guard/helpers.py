"""A throwaway Git repository with recorded results, and the changes applied to it."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

from scripts.evidence_guard.main.check import run_evidence_guard
from tests.unit.scripts.evidence_guard._test_types import Change

_RESULTS: str = "evidence/results"
_AUDIT_RESULTS: str = "evidence/audit/results"
_RECORDER: str = "sqlquack-record 0.1.0"


def _git(repository: Path, *arguments: str) -> None:
    subprocess.run(
        ("git", "-c", "user.name=Test", "-c", "user.email=test@example.com", *arguments),
        cwd=repository,
        check=True,
        capture_output=True,
    )


def _write_result(
    *,
    results: Path,
    seed: str,
    recorder: str,
    arrow: bytes,
    claimed: bytes,
    replaced: tuple[tuple[str, object], ...] = (),
) -> str:
    case_hash: str = hashlib.sha256(seed.encode("utf-8")).hexdigest()
    for _ in filter(None, (arrow,)):
        (results / f"{case_hash}.arrow").write_bytes(arrow)
    metadata: dict[str, object] = {
        "format": 1,
        "case_hash": case_hash,
        "outcome": "rows",
        "recorder": recorder,
        "connector": "test",
        "query_id_sha256": hashlib.sha256(b"01b2-fake-query").hexdigest(),
        "warehouse_version": "test",
        "duckdb_version": "v2.0.0",
        "recorded_at": "1970-01-01T00:00:00Z",
        "session_parameters": {},
        "columns": [],
        "row_count": 0,
        "arrow_sha256": hashlib.sha256(claimed).hexdigest(),
        "error": None,
    }
    (results / f"{case_hash}.json").write_text(
        json.dumps({**metadata, **dict(replaced)}), encoding="utf-8"
    )
    return case_hash


def unchanged(_: Path, __: str) -> None:
    """Changes nothing under the results."""


def edit_stored(results: Path, stored: str) -> None:
    """Rewrites the stored Arrow file."""
    (results / f"{stored}.arrow").write_bytes(b"other rows")


def delete_stored(results: Path, stored: str) -> None:
    """Deletes the stored metadata file."""
    (results / f"{stored}.json").unlink()


def add_valid(results: Path, _: str) -> None:
    """Adds a well-formed result."""
    _write_result(results=results, seed="new", recorder=_RECORDER, arrow=b"new", claimed=b"new")


def add_mismatched(results: Path, _: str) -> None:
    """Adds a result whose Arrow file does not match its metadata."""
    _write_result(results=results, seed="new", recorder=_RECORDER, arrow=b"new", claimed=b"old")


def add_stand_in(results: Path, _: str) -> None:
    """Adds a result that the recorder did not write."""
    _write_result(
        results=results, seed="new", recorder="test fixture", arrow=b"new", claimed=b"new"
    )


def add_pinned(results: Path, _: str) -> None:
    """Adds a result recording its effective session parameters and their snapshot hash."""
    _write_result(
        results=results,
        seed="new",
        recorder=_RECORDER,
        arrow=b"new",
        claimed=b"new",
        replaced=(
            ("effective_parameters", {"TIMEZONE": "UTC"}),
            ("parameters_sha256", hashlib.sha256(b"TIMEZONE=UTC\n").hexdigest()),
        ),
    )


def add_raw_parameters_hash(results: Path, _: str) -> None:
    """Adds a result whose parameter snapshot is not a SHA-256 digest."""
    _write_result(
        results=results,
        seed="new",
        recorder=_RECORDER,
        arrow=b"new",
        claimed=b"new",
        replaced=(("effective_parameters", {}), ("parameters_sha256", "TIMEZONE=UTC")),
    )


def add_raw_query_id(results: Path, _: str) -> None:
    """Adds a result that stores a raw query ID instead of its hash."""
    _write_result(
        results=results,
        seed="new",
        recorder=_RECORDER,
        arrow=b"new",
        claimed=b"new",
        replaced=(("query_id_sha256", "01b2-fake-query"),),
    )


def add_types(results: Path, _: str) -> None:
    """Adds a typing case's result: column types and a row count, no Arrow file."""
    _write_result(
        results=results,
        seed="new",
        recorder=_RECORDER,
        arrow=b"",
        claimed=b"",
        replaced=(("outcome", "types"), ("arrow_sha256", None), ("row_count", 3)),
    )


def add_types_with_arrow(results: Path, _: str) -> None:
    """Adds a types outcome that still carries an Arrow file."""
    _write_result(
        results=results,
        seed="new",
        recorder=_RECORDER,
        arrow=b"new",
        claimed=b"new",
        replaced=(("outcome", "types"),),
    )


def add_error_message(results: Path, _: str) -> None:
    """Adds an error result that keeps the warehouse's message."""
    _write_result(
        results=results,
        seed="new",
        recorder=_RECORDER,
        arrow=b"new",
        claimed=b"new",
        replaced=(("error", {"errno": 2003, "sqlstate": "42S02", "message": "Object missing"}),),
    )


def add_error_number_message(results: Path, _: str) -> None:
    """Adds an error result whose message is a number rather than text."""
    _write_result(
        results=results,
        seed="new",
        recorder=_RECORDER,
        arrow=b"new",
        claimed=b"new",
        replaced=(("error", {"errno": 2003, "sqlstate": "42S02", "message": 7}),),
    )


def add_audit_valid(results: Path, _: str) -> None:
    """Adds a well-formed re-recorded result under the audit results."""
    audit: Path = results.parent / "audit" / "results"
    audit.mkdir(parents=True, exist_ok=True)
    _write_result(results=audit, seed="new", recorder=_RECORDER, arrow=b"new", claimed=b"new")


def edit_audit(results: Path, _: str) -> None:
    """Rewrites a committed re-recorded result."""
    audit: Path = results.parent / "audit" / "results"
    stored: str = hashlib.sha256(b"audited").hexdigest()
    (audit / f"{stored}.arrow").write_bytes(b"other rows")


def guard_after(*, repository: Path, change: Change) -> int:
    """Commit one recorded result as the base, apply `change` on top, and run the guard."""
    results: Path = repository / _RESULTS
    results.mkdir(parents=True)
    _git(repository, "init", "--quiet", "--initial-branch=main")
    stored: str = _write_result(
        results=results, seed="stored", recorder=_RECORDER, arrow=b"rows", claimed=b"rows"
    )
    audit: Path = repository / _AUDIT_RESULTS
    audit.mkdir(parents=True)
    _write_result(results=audit, seed="audited", recorder=_RECORDER, arrow=b"rows", claimed=b"rows")
    _git(repository, "add", "-A")
    _git(repository, "commit", "--quiet", "-m", "base")
    change(results, stored)
    _git(repository, "add", "-A")
    _git(repository, "commit", "--quiet", "--allow-empty", "-m", "change")
    return run_evidence_guard(["--base", "HEAD~1", "--repository", str(repository)])
