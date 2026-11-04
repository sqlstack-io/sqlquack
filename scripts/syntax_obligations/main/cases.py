"""Write coverage/syntax/cases/<family>.jsonl: one case per distinct rendered SQL text."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from scripts.syntax_obligations._helpers.cases import (
    held_cases,
    jsonl_text,
    packet_text,
    receipt_phases,
    recorded_receipts,
    run_database_setups,
    settled_cases,
)
from scripts.syntax_obligations.types import Row


def run_cases(argv: list[str] | None = None) -> int:
    """Regenerate the cases of the named families and list those that need a capture."""
    arguments: argparse.Namespace = _parse_args(argv)
    root: Path = arguments.root
    phases: dict[tuple[str, str], str] = receipt_phases(arguments.receipts)
    recorded: set[tuple[str, str]] = recorded_receipts(arguments.receipts)
    setups, flagged = run_database_setups(arguments.setups or arguments.receipts)
    print(json.dumps({"setup_review": flagged}))
    held: frozenset[str] = held_cases(arguments.held)
    to_capture: list[Row] = []
    for family in arguments.families:
        cases, arity = settled_cases(
            root=root, family=family, phases=phases, setups=setups, held=held
        )
        path: Path = root / "cases" / f"{family}.jsonl"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(jsonl_text(cases))
        arity_path: Path = root / "arity" / f"{family}.jsonl"
        if arity:
            arity_path.parent.mkdir(parents=True, exist_ok=True)
            arity_path.write_text(jsonl_text(arity))
        elif arity_path.exists():
            arity_path.unlink()
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
    if arguments.packet:
        arguments.packet.write_text(packet_text(to_capture))
    return 0


def _parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("coverage/syntax"))
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
        help="held-case list (JSON lines with case_id) whose cases run in a d103 run database",
    )
    parser.add_argument("--packet", type=Path, help="write the cases still to capture here")
    parser.add_argument("families", nargs="+")
    return parser.parse_args(argv)
