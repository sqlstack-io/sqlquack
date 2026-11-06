"""Write coverage/syntax-postgres/obligations/<family>.toml and the cases of the focus forms."""

from __future__ import annotations

import argparse
import json
import tomllib
from pathlib import Path

from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import ReceiptView
from scripts.syntax_obligations_pg._helpers.adapt import install, retired_ids
from scripts.syntax_obligations_pg._helpers.cases import focus_forms, receipt_view, write_family
from scripts.syntax_obligations_pg._helpers.files import restore, snapshot
from scripts.syntax_obligations_pg.constants import (
    DEFAULT_FOCUS,
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
    """Regenerate every family's obligations and the focus forms' cases."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--focus", type=Path, default=DEFAULT_FOCUS)
    arguments: argparse.Namespace = parser.parse_args(argv)
    previous: set[str] = _issued_ids(arguments.root)
    kept: dict[Path, bytes] = snapshot(arguments.root)
    try:
        _build(root=arguments.root, focus_file=arguments.focus, previous=previous)
    except BaseException:
        restore(root=arguments.root, kept=kept)
        raise
    return 0


def _build(*, root: Path, focus_file: Path, previous: set[str]) -> None:
    """Write every family, then refuse a reissued id or one that left without a tombstone."""
    install()
    receipts: ReceiptView = receipt_view(root / RECEIPTS_DIR)
    focus: set[str] = set(focus_forms(focus_file))
    for family in FAMILIES:
        obligations, cases = write_family(root=root, family=family, focus=focus, receipts=receipts)
        print(json.dumps({"family": family, "obligations": obligations, "cases": cases}))
    retired: set[str] = retired_ids(root)
    issued: set[str] = _issued_ids(root)
    reissued: set[str] = retired & issued
    if reissued:
        raise ObligationError(f"retired obligation ids issued again: {sorted(reissued)}")
    vanished: set[str] = previous - issued - retired
    if vanished:
        raise ObligationError(
            f"obligation ids left the root without a tombstone: {sorted(vanished)}"
        )
