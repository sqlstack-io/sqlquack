"""Write coverage/syntax-databricks obligations and cases, or with --check compare them."""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import ReceiptView
from scripts.syntax_obligations.types import Row
from scripts.syntax_obligations_databricks._helpers.adapt import install, retired_ids
from scripts.syntax_obligations_databricks._helpers.cases import (
    family_rows,
    receipt_view,
    stale_files,
    write_family,
)
from scripts.syntax_obligations_databricks.constants import (
    DEFAULT_ROOT,
    FAMILIES,
    OBLIGATIONS_DIR,
    RECEIPTS_DIR,
)


def _issued_ids(root: Path) -> set[str]:
    """Every obligation id written under the root's obligations/."""
    issued: set[str] = set()
    for family in FAMILIES:
        path: Path = root / OBLIGATIONS_DIR / f"{family}.toml"
        if not path.exists():
            continue
        rows: list[dict[str, object]] = tomllib.loads(path.read_text()).get("obligation", [])
        issued.update(str(row["id"]) for row in rows)
    return issued


def run_build(argv: list[str] | None = None) -> int:
    """Regenerate every family's obligations and cases, or write nothing if an id would vanish."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument(
        "--check", action="store_true", help="write nothing; fail when a committed file differs"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    previous: set[str] = _issued_ids(arguments.root)
    install()
    receipts: ReceiptView = receipt_view(arguments.root / RECEIPTS_DIR)
    planned: dict[str, tuple[list[Row], list[Row]]] = {
        family: family_rows(root=arguments.root, family=family, receipts=receipts)
        for family in FAMILIES
    }
    issued: set[str] = set()
    for rows, _ in planned.values():
        issued.update(str(row["id"]) for row in rows)
    retired: set[str] = retired_ids(arguments.root)
    reissued: set[str] = retired & issued
    if reissued:
        raise ObligationError(f"retired obligation ids issued again: {sorted(reissued)}")
    vanished: set[str] = previous - issued - retired
    if vanished:
        raise ObligationError(
            f"obligation ids would leave the root without a tombstone: {sorted(vanished)}"
        )
    if arguments.check:
        return _check(root=arguments.root, planned=planned)
    for family, (rows, cases) in planned.items():
        write_family(root=arguments.root, family=family, rows=rows, cases=cases)
        print(json.dumps({"family": family, "obligations": len(rows), "cases": len(cases)}))
    return 0


def _check(*, root: Path, planned: dict[str, tuple[list[Row], list[Row]]]) -> int:
    """Print each committed file that differs from the build, and fail when any does."""
    stale: list[Path] = stale_files(root=root, planned=planned)
    for path in stale:
        print(f"stale: {path} is missing, differs, or is not written by the build", file=sys.stderr)
    print(f"dialect cases: {root}: {len(stale)} stale files; run without --check to regenerate")
    return 1 if stale else 0
