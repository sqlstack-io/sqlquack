"""The planned root: every family's rows and cases, checked against the written root, written."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

from scripts.grammar_obligations.exceptions import ObligationError
from scripts.grammar_obligations.models import ReceiptView
from scripts.syntax_obligations_bigquery._helpers.adapt import install, retired_ids
from scripts.syntax_obligations_bigquery._helpers.cases import (
    family_rows,
    receipt_view,
    write_family,
)
from scripts.syntax_obligations_bigquery.classes.withdrawals import Withdrawals
from scripts.syntax_obligations_bigquery.constants import FAMILIES, OBLIGATIONS_DIR, RECEIPTS_DIR
from scripts.syntax_obligations_bigquery.models import FamilyPlan


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


def plan(root: Path) -> dict[str, FamilyPlan]:
    """Every family's obligation rows and cases, as the build would write them."""
    withdrawn: Withdrawals = install(root)
    receipts: ReceiptView = receipt_view(root / RECEIPTS_DIR)
    return {
        family: family_rows(root=root, family=family, receipts=receipts, withdrawn=withdrawn)
        for family in FAMILIES
    }


def refuse_vanished(*, root: Path, planned: dict[str, FamilyPlan]) -> None:
    """Refuse, before writing, a retired id issued again or an issued id with no tombstone."""
    issued: set[str] = set()
    for found in planned.values():
        issued.update(str(row["id"]) for row in found.rows)
    retired: set[str] = retired_ids(root)
    reissued: set[str] = retired & issued
    if reissued:
        raise ObligationError(f"retired obligation ids issued again: {sorted(reissued)}")
    vanished: set[str] = _issued_ids(root) - issued - retired
    if vanished:
        raise ObligationError(
            f"obligation ids would leave the root without a tombstone: {sorted(vanished)}"
        )


def write_root(*, root: Path, planned: dict[str, FamilyPlan]) -> None:
    """Write each family's obligations and cases, reporting their counts."""
    for family, found in planned.items():
        write_family(root=root, family=family, rows=found.rows, cases=found.cases)
        print(
            json.dumps(
                {"family": family, "obligations": len(found.rows), "cases": len(found.cases)}
            )
        )
