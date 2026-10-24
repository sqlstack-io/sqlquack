"""A throwaway Git repository with recorded results."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

from scripts.evidence_guard.main.check import run_evidence_guard

_RESULTS: str = "evidence/results"


def _git(repository: Path, *arguments: str) -> None:
    subprocess.run(
        ("git", "-c", "user.name=Test", "-c", "user.email=test@example.com", *arguments),
        cwd=repository,
        check=True,
        capture_output=True,
    )


def _write_result(results: Path, seed: str, recorder: str, arrow: bytes, claimed: bytes) -> str:
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


def guard_after(repository: Path, change: str) -> int:
    """Commit one recorded result as the base, apply `change` on top, and run the guard."""
    results: Path = repository / _RESULTS
    results.mkdir(parents=True)
    _git(repository, "init", "--quiet", "--initial-branch=main")
    stored: str = _write_result(results, "stored", "sqlquack-record 0.1.0", b"rows", b"rows")
    _git(repository, "add", "-A")
    _git(repository, "commit", "--quiet", "-m", "base")
    if change == "edit":
        (results / f"{stored}.arrow").write_bytes(b"other rows")
    elif change == "delete":
        (results / f"{stored}.json").unlink()
    elif change == "add":
        _write_result(results, "new", "sqlquack-record 0.1.0", b"new rows", b"new rows")
    elif change == "add-mismatched":
        _write_result(results, "new", "sqlquack-record 0.1.0", b"new rows", b"claimed rows")
    elif change == "add-stand-in":
        _write_result(results, "new", "test fixture", b"new rows", b"new rows")
    _git(repository, "add", "-A")
    _git(repository, "commit", "--quiet", "--allow-empty", "-m", "change")
    return run_evidence_guard(["--base", "HEAD~1", "--repository", str(repository)])
