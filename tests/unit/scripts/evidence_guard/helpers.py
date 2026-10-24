"""A throwaway Git repository with recorded results, and the changes applied to it."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

from scripts.evidence_guard.main.check import run_evidence_guard
from tests.unit.scripts.evidence_guard._test_types import Change

_RESULTS: str = "evidence/results"
_RECORDER: str = "sqlquack-record 0.1.0"


def _git(repository: Path, *arguments: str) -> None:
    subprocess.run(
        ("git", "-c", "user.name=Test", "-c", "user.email=test@example.com", *arguments),
        cwd=repository,
        check=True,
        capture_output=True,
    )


def _write_result(*, results: Path, seed: str, recorder: str, arrow: bytes, claimed: bytes) -> str:
    case_hash: str = hashlib.sha256(seed.encode("utf-8")).hexdigest()
    (results / f"{case_hash}.arrow").write_bytes(arrow)
    metadata: dict[str, object] = {
        "format": 1,
        "case_hash": case_hash,
        "outcome": "rows",
        "recorder": recorder,
        "connector": "test",
        "query_id": None,
        "warehouse_version": "test",
        "duckdb_version": "v2.0.0",
        "recorded_at": "1970-01-01T00:00:00Z",
        "session_parameters": {},
        "columns": [],
        "row_count": 0,
        "arrow_sha256": hashlib.sha256(claimed).hexdigest(),
        "error": None,
    }
    (results / f"{case_hash}.json").write_text(json.dumps(metadata), encoding="utf-8")
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


def guard_after(*, repository: Path, change: Change) -> int:
    """Commit one recorded result as the base, apply `change` on top, and run the guard."""
    results: Path = repository / _RESULTS
    results.mkdir(parents=True)
    _git(repository, "init", "--quiet", "--initial-branch=main")
    stored: str = _write_result(
        results=results, seed="stored", recorder=_RECORDER, arrow=b"rows", claimed=b"rows"
    )
    _git(repository, "add", "-A")
    _git(repository, "commit", "--quiet", "-m", "base")
    change(results, stored)
    _git(repository, "add", "-A")
    _git(repository, "commit", "--quiet", "--allow-empty", "-m", "change")
    return run_evidence_guard(["--base", "HEAD~1", "--repository", str(repository)])
