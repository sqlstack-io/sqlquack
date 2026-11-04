"""Writing the receipts not yet published and the import report, append-only."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.syntax_receipts._helpers.hashing import canonical_json, digest
from scripts.syntax_receipts.constants import RECEIPT_FORMAT
from scripts.syntax_receipts.models import Imported
from scripts.syntax_receipts.types import Row


def write_import(
    *, imported: Imported, published: list[Path], receipts_path: Path, report_path: Path
) -> Row:
    """Write the receipts absent from `published` and the report; the counts written."""
    known: set[bytes] = _published_lines(paths=published)
    receipts: list[Row] = [r for r in imported.receipts if canonical_json(r) not in known]
    lines: bytes = b"".join(canonical_json(r) + b"\n" for r in receipts)
    report: Row = {
        "format": RECEIPT_FORMAT,
        "receipts_file_sha256": digest(lines),
        "receipts": len(receipts),
        "refused": len(imported.refusals),
        "runs": imported.runs,
        "refusals": imported.refusals,
        "inputs": sorted(imported.inputs.values(), key=lambda row: row["sql_sha256"]),
    }
    _write_new(path=receipts_path, data=lines)
    _write_new(
        path=report_path,
        data=json.dumps(report, indent=1, sort_keys=True, ensure_ascii=False).encode() + b"\n",
    )
    return {"receipts": len(receipts), "refused": len(imported.refusals)}


def _published_lines(*, paths: list[Path]) -> set[bytes]:
    known: set[bytes] = set()
    for path in paths:
        known.update(line.strip() for line in path.read_bytes().splitlines() if line.strip())
    return known


def _write_new(*, path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as handle:
        handle.write(data)
