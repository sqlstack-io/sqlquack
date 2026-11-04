"""Writing the receipts not yet published and the import report, append-only."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.syntax_receipts._helpers.hashing import canonical_json, digest
from scripts.syntax_receipts.constants import RECEIPT_FORMAT, SUPERSEDES_KEY
from scripts.syntax_receipts.models import Imported
from scripts.syntax_receipts.types import Row


def write_import(
    *, imported: Imported, published: list[Path], receipts_path: Path, report_path: Path
) -> Row:
    """Write the receipts absent from `published`, superseding changed ones, and the report."""
    known: dict[tuple[str, str, str], Row] = _published_observations(paths=published)
    receipts: list[Row] = []
    for row in imported.receipts:
        earlier: Row | None = known.get(_observation(receipt=row))
        if earlier is None:
            receipts.append(row)
        elif earlier["phase"] != row["phase"]:
            receipts.append({**row, SUPERSEDES_KEY: earlier["receipt_sha256"]})
    reasons: dict[tuple[str, str], str] = {
        (r["run_id"], r["row_id"]): r["reason"] for r in imported.refusals
    }
    withdrawn: list[Row] = [
        {
            "receipt_sha256": earlier["receipt_sha256"],
            "run_id": key[0],
            "case_id": key[1],
            "phase": earlier["phase"],
            "reason": reasons[key[0], key[1]],
        }
        for key, earlier in sorted(known.items())
        if (key[0], key[1]) in reasons
    ]
    lines: bytes = b"".join(canonical_json(r) + b"\n" for r in receipts)
    report: Row = {
        "format": RECEIPT_FORMAT,
        "receipts_file_sha256": digest(lines),
        "receipts": len(receipts),
        "superseding": sum(SUPERSEDES_KEY in r for r in receipts),
        "withdrawn": withdrawn,
        "refused": len(imported.refusals),
        "runs": imported.runs,
        "refusals": imported.refusals,
        "inputs": sorted(
            imported.inputs.values(), key=lambda row: (row["sql_sha256"], row["setup_sha256"])
        ),
    }
    _write_new(path=receipts_path, data=lines)
    _write_new(
        path=report_path,
        data=json.dumps(report, indent=1, sort_keys=True, ensure_ascii=False).encode() + b"\n",
    )
    return {"receipts": len(receipts), "withdrawn": len(withdrawn), "refused": len(imported.refusals)}


def _published_observations(*, paths: list[Path]) -> dict[tuple[str, str, str], Row]:
    """Published receipts by recorded observation, with the phase and hash of the published line."""
    known: dict[tuple[str, str, str], Row] = {}
    for path in paths:
        for line in path.read_bytes().splitlines():
            if line.strip():
                row: Row = json.loads(line)
                known[_observation(receipt=row)] = {
                    "phase": row["phase"],
                    "receipt_sha256": digest(line.strip()),
                }
    return known


def _observation(*, receipt: Row) -> tuple[str, str, str]:
    return receipt["run_id"], receipt["case_id"], receipt["raw_record_sha256"]


def _write_new(*, path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as handle:
        handle.write(data)
