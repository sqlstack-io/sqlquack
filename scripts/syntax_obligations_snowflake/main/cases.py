"""Write coverage/syntax/cases/<family>.jsonl: one case per distinct rendered SQL text."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from scripts.grammar_obligations._helpers.cases import (
    family_outputs,
    held_cases,
    packet_text,
    receipt_phases,
    receipt_settlements,
    recorded_receipts,
    run_database_setups,
    sync_outputs,
)
from scripts.grammar_obligations.models import ReceiptView
from scripts.grammar_obligations.types import Row
from scripts.syntax_obligations_snowflake._helpers.rules import install


def run_cases(argv: list[str] | None = None) -> int:
    """Regenerate the cases of the named families and list those that need a capture."""
    install()
    arguments: argparse.Namespace = _parse_args(argv)
    receipts: ReceiptView = ReceiptView(
        phases=receipt_phases(arguments.receipts),
        settlements=receipt_settlements(arguments.receipts),
    )
    recorded: set[tuple[str, str]] = recorded_receipts(arguments.receipts)
    setups, flagged = run_database_setups(arguments.setups or arguments.receipts)
    print(json.dumps({"setup_review": flagged}))
    held: frozenset[str] = held_cases(arguments.held)
    to_capture: list[Row] = []
    differing: list[Path] = []
    for family in arguments.families:
        cases, arity, outputs = family_outputs(
            root=arguments.root,
            evidence=arguments.evidence or arguments.root,
            family=family,
            receipts=receipts,
            setups=setups,
            held=held,
        )
        differing += sync_outputs(outputs=outputs, check=arguments.check)
        to_capture += [c for c in cases + arity if (c["case_id"], c["sql_sha256"]) not in recorded]
        print(
            json.dumps(
                {
                    "family": family,
                    "cases": len(cases),
                    "arity": len(arity),
                    "to_capture": len(to_capture),
                }
            )
        )
    if arguments.check:
        for path in differing:
            print(f"syntax cases: {path} differs from what the receipts regenerate")
        print(f"syntax cases: {'FAILED' if differing else 'passed'}, {len(differing)} files differ")
        return 1 if differing else 0
    if arguments.packet:
        arguments.packet.write_text(packet_text(to_capture))
    return 0


def _parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("coverage/syntax"))
    parser.add_argument(
        "--evidence",
        type=Path,
        help="directory whose cases/ and mapped-cases/ the build reads and writes "
        "(default: --root)",
    )
    parser.add_argument(
        "--receipts",
        type=Path,
        required=True,
        help="receipt directory: matches SQL hashes and gives accepted cases their stage",
    )
    parser.add_argument(
        "--setups",
        type=Path,
        help="receipt directory whose run-database receipts give ddl cases their setup "
        "(default: --receipts)",
    )
    parser.add_argument(
        "--held",
        type=Path,
        action="append",
        default=[],
        help="held-case list (JSON lines with case_id) whose cases run in a run database",
    )
    parser.add_argument("--packet", type=Path, help="write the cases still to capture here")
    parser.add_argument(
        "--check",
        action="store_true",
        help="write nothing; fail when a family's files differ from what the receipts regenerate",
    )
    parser.add_argument("families", nargs="+")
    return parser.parse_args(argv)
