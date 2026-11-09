"""Write coverage/syntax-bigquery/obligations/<family>.toml and cases/<family>.jsonl."""

from __future__ import annotations

import argparse
import itertools
import json
from pathlib import Path

from scripts.grammar_obligations.types import Row
from scripts.syntax_obligations_bigquery._helpers.cases import receipt_view
from scripts.syntax_obligations_bigquery._helpers.parser import (
    agreement,
    analysis_shaped,
    engine_confirmed,
    gate,
)
from scripts.syntax_obligations_bigquery._helpers.root import plan, refuse_vanished, write_root
from scripts.syntax_obligations_bigquery.constants import DEFAULT_ROOT, RECEIPTS_DIR
from scripts.syntax_obligations_bigquery.models import EngineEvidence, FamilyPlan, Verdict


def run_build(argv: list[str] | None = None) -> int:
    """Plan every family, refuse vanished ids and parser disagreements, then write the root."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument(
        "--parser", type=Path, required=True, help="the pinned GoogleSQL execute_query_linux"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    planned: dict[str, FamilyPlan] = plan(arguments.root)
    refuse_vanished(root=arguments.root, planned=planned)
    cases: list[Row] = list(itertools.chain.from_iterable(p.cases for p in planned.values()))
    settle: set[str] = set()
    withdrawn: list[tuple[str, str]] = []
    for family_plan in planned.values():
        settle |= family_plan.settle
        withdrawn += family_plan.withdrawn
    phases: dict[tuple[str, str], str] = receipt_view(arguments.root / RECEIPTS_DIR).phases
    confirmed: frozenset[str] = engine_confirmed(cases=cases, phases=phases)
    found: dict[str, Verdict] = gate(
        parser=arguments.parser,
        cases=cases,
        settle=frozenset(settle),
        withdrawn=withdrawn,
        evidence=EngineEvidence(
            confirmed=confirmed, shaped=analysis_shaped(arguments.root / RECEIPTS_DIR)
        ),
    )
    write_root(root=arguments.root, planned=planned)
    print(json.dumps({"parser": agreement(cases=cases, found=found, confirmed=confirmed)}))
    return 0
