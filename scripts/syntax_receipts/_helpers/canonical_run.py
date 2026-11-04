"""Canonical recorder runs: one result document per case, bound by the audit's file hashes."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.syntax_receipts._helpers.audit import completed
from scripts.syntax_receipts._helpers.hashing import digest
from scripts.syntax_receipts._helpers.receipts import input_row, receipt, refusal
from scripts.syntax_receipts.constants import CANONICAL_ACCEPTED_OUTCOMES, CAPTURE_CANONICAL
from scripts.syntax_receipts.exceptions import RowRefused, RunRefused
from scripts.syntax_receipts.models import CaptureRun, Imported, Observation
from scripts.syntax_receipts.types import Row


def import_canonical_run(*, run: CaptureRun, index: dict[str, Path]) -> Imported:
    """Receipts for the bound cases whose result document and pending SQL match the audit."""
    start: Row = run.events[0]
    binding: Row = start["binding"]
    manifest_path: Path | None = index.get(binding.get("manifest_sha256", ""))
    if manifest_path is None:
        raise RunRefused(f"manifest {binding.get('manifest_sha256')} not found among the inputs")
    manifest: Row = json.loads(manifest_path.read_text())
    by_hash: dict[str, Row] = {case["case_hash"]: case for case in manifest["cases"]}
    files: Row = completed(run=run).get("files") or {}
    results: Path = Path(start["results"])
    found: Imported = Imported()
    for case_hash in binding["cases"]:
        result_path: Path = results / f"{case_hash}.json"
        try:
            case, sql, result, session = _bound_case(
                result_path=result_path,
                files=files,
                pending_path=manifest_path.parent / "pending" / f"{case_hash}.json",
                case=by_hash.get(case_hash),
            )
        except RowRefused as refused:
            found.refusals.append(
                refusal(run_id=run.path.name, row_id=case_hash, reason=str(refused))
            )
            continue
        setup: list[str] = list(case.get("setup_sql") or [])
        exact: Row = input_row(sql=sql, setup=setup)
        found.inputs.setdefault(exact["sql_sha256"], exact)
        error: Row = result.get("error") or {}
        observation: Observation = Observation(
            run_id=run.path.name,
            audit_sha=run.audit_sha,
            raw_bytes=result_path.read_bytes(),
            case_id=case.get("reference", case_hash),
            packet_sha=binding["manifest_sha256"],
            sql_sha=exact["sql_sha256"],
            setup=setup,
            session=session,
            method=CAPTURE_CANONICAL,
            accepted=result.get("outcome") in CANONICAL_ACCEPTED_OUTCOMES,
            errno=error.get("errno"),
            sqlstate=error.get("sqlstate"),
            message=str(error.get("message") or ""),
        )
        extra: Row = {
            "query_id": None,
            "query_id_sha256": result.get("query_id_sha256"),
            "warehouse_version": result.get("warehouse_version"),
            "recorded_at": result.get("recorded_at"),
            "case_hash": case_hash,
        }
        try:
            found.receipts.append(receipt(observation=observation, extra=extra))
        except RowRefused as failure:
            found.refusals.append(
                refusal(run_id=run.path.name, row_id=case_hash, reason=str(failure))
            )
    return found


def _bound_case(
    *, result_path: Path, files: Row, pending_path: Path, case: Row | None
) -> tuple[Row, str, Row, Row]:
    if not result_path.is_file() or digest(result_path.read_bytes()) != files.get(result_path.name):
        raise RowRefused("result document missing or differs from the audit")
    if case is None or not pending_path.is_file():
        raise RowRefused("case is not in the bound manifest")
    pending: Row = json.loads(pending_path.read_text())
    sql: object = pending.get("source_sql")
    if not isinstance(sql, str) or digest(sql.encode()) != case.get("source_sql_sha256"):
        raise RowRefused("pending SQL does not match the manifest's hash")
    result: Row = json.loads(result_path.read_bytes())
    pinned: Row = case.get("pinned_parameters") or {}
    session: Row = {**pinned, **(result.get("session_parameters") or {})}
    if not result.get("session_parameters"):
        raise RowRefused("result records no session parameters")
    return case, sql, result, session
