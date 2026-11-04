"""Importing every run directory with the driver its start event names."""

from __future__ import annotations

from pathlib import Path

from scripts.syntax_receipts._helpers.audit import index_inputs, read_audit
from scripts.syntax_receipts._helpers.canonical_run import import_canonical_run
from scripts.syntax_receipts._helpers.ddl_run import import_ddl_run
from scripts.syntax_receipts._helpers.diagnostic_run import import_diagnostic_run
from scripts.syntax_receipts._helpers.hashing import canonical_json, digest
from scripts.syntax_receipts._helpers.packet_run import import_packet_run
from scripts.syntax_receipts._helpers.receipts import refusal
from scripts.syntax_receipts.constants import (
    EXPLAIN_METHOD,
    MANIFEST_KEY,
    PROPOSAL_KEY,
    RECONCILED_STATUS,
    RUN_DATABASE_KEY,
    SCRIPTING_SUBMISSION,
    UNRECORDED_KEY,
    WHOLE_RUN_ROW,
)
from scripts.syntax_receipts.exceptions import RunRefused
from scripts.syntax_receipts.models import CaptureRun, Imported
from scripts.syntax_receipts.types import Row


def import_runs(*, runs: Path, input_dirs: list[Path]) -> Imported:
    """Every run directory under `runs`, imported or refused with a reason."""
    imported: Imported = Imported()
    index: dict[str, Path] = index_inputs(directories=input_dirs)
    for path in sorted(path for path in runs.iterdir() if path.is_dir()):
        try:
            run: CaptureRun = read_audit(run=path)
            found: Imported = _import_run(run=run, index=index)
            status: Row = {"run_id": path.name, "status": "imported", "audit_sha256": run.audit_sha}
            if run.reconciliation is not None:
                found = _reconciled(run=run, found=found)
                status["status"] = RECONCILED_STATUS
        except RunRefused as error:
            found = Imported(
                refusals=[refusal(run_id=path.name, row_id=WHOLE_RUN_ROW, reason=str(error))]
            )
            status = {"run_id": path.name, "status": "refused", "reason": str(error)}
        status.update(receipts=len(found.receipts), refused_rows=len(found.refusals))
        imported.receipts.extend(found.receipts)
        imported.refusals.extend(found.refusals)
        for sha, row in found.inputs.items():
            imported.inputs.setdefault(sha, row)
        imported.runs.append(status)
    return imported


def _import_run(*, run: CaptureRun, index: dict[str, Path]) -> Imported:
    start: Row = run.events[0]
    if RUN_DATABASE_KEY in start:
        return import_ddl_run(run=run, index=index)
    if UNRECORDED_KEY in (run.reconciliation or {}):
        raise RunRefused("only a DDL run can reconcile a submission without a raw record")
    if start.get("submission") == SCRIPTING_SUBMISSION or start.get("method") == EXPLAIN_METHOD:
        return import_packet_run(run=run, index=index)
    if PROPOSAL_KEY in start:
        return import_diagnostic_run(run=run)
    if MANIFEST_KEY in (start.get("binding") or {}):
        return import_canonical_run(run=run, index=index)
    raise RunRefused("start event matches no known capture driver")


def _reconciled(*, run: CaptureRun, found: Imported) -> Imported:
    """The receipts whose record the reconciliation lists with the same hash; the rest refused."""
    record: Row = run.reconciliation or {}
    listed: dict[str, str] = {
        str(entry.get("id")): str(entry.get("sha256")) for entry in record["raw_records"]["records"]
    }
    if UNRECORDED_KEY in record:
        unrecorded: Row = record[UNRECORDED_KEY]
        listed.setdefault(str(unrecorded.get("case_id")), digest(canonical_json(unrecorded)))
    kept: Imported = Imported(inputs=found.inputs, refusals=list(found.refusals))
    seen: set[str] = {row["row_id"] for row in found.refusals}
    for receipt in found.receipts:
        case_id: str = receipt["case_id"]
        seen.add(case_id)
        if listed.get(case_id) == receipt["raw_record_sha256"]:
            kept.receipts.append(receipt)
            continue
        reason: str = (
            "raw record hash differs from the run's reconciliation"
            if case_id in listed
            else "raw record is not listed in the run's reconciliation"
        )
        kept.refusals.append(refusal(run_id=run.path.name, row_id=case_id, reason=reason))
    kept.refusals.extend(
        refusal(
            run_id=run.path.name,
            row_id=case_id,
            reason="listed in the run's reconciliation but absent from its records",
        )
        for case_id in sorted(listed.keys() - seen)
    )
    return kept
