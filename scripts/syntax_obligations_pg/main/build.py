"""Write coverage/syntax-postgres/obligations/<family>.toml and the cases of the focus forms."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from scripts.syntax_obligations.models import ReceiptView
from scripts.syntax_obligations_pg._helpers.adapt import install
from scripts.syntax_obligations_pg._helpers.cases import focus_forms, receipt_view, write_family
from scripts.syntax_obligations_pg.constants import (
    DEFAULT_FOCUS,
    DEFAULT_ROOT,
    FAMILIES,
    RECEIPTS_DIR,
)


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
    return 0
