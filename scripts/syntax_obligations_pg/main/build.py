"""Write coverage/syntax-postgres/obligations/<family>.toml and the cases of the focus forms."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from scripts.syntax_catalogue._helpers.toml_out import write_tables
from scripts.syntax_obligations._helpers.cases import (
    jsonl_text,
    receipt_phases,
    receipt_settlements,
)
from scripts.syntax_obligations._helpers.obligations import family_obligations, obligation_rows
from scripts.syntax_obligations.models import ReceiptView
from scripts.syntax_obligations.types import Row
from scripts.syntax_obligations_pg._helpers.adapt import install
from scripts.syntax_obligations_pg._helpers.cases import family_cases, focus_forms
from scripts.syntax_obligations_pg.constants import (
    DEFAULT_FOCUS,
    DEFAULT_ROOT,
    FAMILIES,
    OBLIGATIONS_HEADER,
    RECEIPTS_DIR,
)


def run_build(argv: list[str] | None = None) -> int:
    """Regenerate every family's obligations and the focus forms' cases."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--focus", type=Path, default=DEFAULT_FOCUS)
    arguments: argparse.Namespace = parser.parse_args(argv)
    root: Path = arguments.root
    install()
    receipts: ReceiptView = ReceiptView(
        phases=receipt_phases(root / RECEIPTS_DIR),
        settlements=receipt_settlements(root / RECEIPTS_DIR),
    )
    focus: set[str] = set(focus_forms(arguments.focus))
    for family in FAMILIES:
        numbered, trees, _, _, _ = family_obligations(root=root, family=family)
        rows: list[Row] = obligation_rows(numbered)
        path: Path = root / "obligations" / f"{family}.toml"
        path.parent.mkdir(parents=True, exist_ok=True)
        write_tables(
            path=path,
            header=OBLIGATIONS_HEADER.format(family=family),
            tables=[("obligation", rows)],
        )
        cases: list[Row] = family_cases(
            family=family, numbered=numbered, trees=trees, forms=focus, receipts=receipts
        )
        cases_path: Path = root / "cases" / f"{family}.jsonl"
        cases_path.parent.mkdir(parents=True, exist_ok=True)
        cases_path.write_text(jsonl_text(cases))
        print(json.dumps({"family": family, "obligations": len(rows), "cases": len(cases)}))
    return 0
