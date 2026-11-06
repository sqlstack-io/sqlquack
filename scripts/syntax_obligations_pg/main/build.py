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
        rows: list[dict[str, object]] = tomllib.loads(path.read_text()).get("obligation", [])
        issued.update(str(row["id"]) for row in rows)
    return issued


def run_build(argv: list[str] | None = None) -> int:
    """Regenerate every family's obligations and the focus forms' cases."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--focus", type=Path, default=DEFAULT_FOCUS)
    arguments: argparse.Namespace = parser.parse_args(argv)
    install()
    receipts: ReceiptView = receipt_view(arguments.root / RECEIPTS_DIR)
    focus: set[str] = set(focus_forms(arguments.focus))
    for family in FAMILIES:
        obligations, cases = write_family(
            root=arguments.root, family=family, focus=focus, receipts=receipts
        )
        print(json.dumps({"family": family, "obligations": obligations, "cases": cases}))
    reissued: set[str] = retired_ids(arguments.root) & _issued_ids(arguments.root)
    if reissued:
        raise ObligationError(f"retired obligation ids issued again: {sorted(reissued)}")
    return 0
