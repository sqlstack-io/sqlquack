"""Referee report entry point: compare SQLQuack's parse with Snowflake's recorded outcomes."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.referee_tools._helpers.arities import (
    arity_mismatches,
    catalogue_arities,
    sqlquack_arities,
)
from scripts.referee_tools._helpers.checking import parse_verdicts
from scripts.referee_tools._helpers.classify import comparisons
from scripts.referee_tools._helpers.loading import referee_cases
from scripts.referee_tools._helpers.report_text import disagreement_lines, report_markdown
from scripts.referee_tools.constants import (
    EVIDENCE_DIRECTORY,
    FUNCTIONS_DIRECTORY,
    JSON_SUFFIX,
    RESULTS_DIRECTORY,
)
from scripts.referee_tools.models import Comparison, ParseVerdict, RefereeCase


def run_report(argv: list[str] | None = None) -> int:
    """Write report.md and disagreements.jsonl to --out; the report never leaves that folder."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--binary", type=Path, required=True, help="a sqlquack release binary")
    parser.add_argument("--library", type=Path, required=True, help="libduckdb.so's directory")
    parser.add_argument("--out", type=Path, required=True)
    arguments: argparse.Namespace = parser.parse_args(argv)
    repository: Path = arguments.repository
    evidence: Path = repository / EVIDENCE_DIRECTORY
    cases: list[RefereeCase] = referee_cases(repository=repository, patterns=("**/*.json",))
    print(f"referee report: parsing {len(cases)} statements with SQLQuack…", file=sys.stderr)
    verdicts: dict[str, ParseVerdict] = parse_verdicts(
        binary=arguments.binary, library=arguments.library, statements=[c.statement for c in cases]
    )
    compared: list[Comparison] = comparisons(
        cases=cases, verdicts=verdicts, results=evidence / RESULTS_DIRECTORY
    )
    catalogues: list[Path] = sorted((evidence / FUNCTIONS_DIRECTORY).glob(f"*{JSON_SUFFIX}"))
    rows: list[tuple[str, str, str]] = (
        arity_mismatches(
            sqlquack=sqlquack_arities(repository), snowflake=catalogue_arities(catalogues[-1])
        )
        if catalogues
        else []
    )
    arguments.out.mkdir(parents=True, exist_ok=True)
    (arguments.out / "report.md").write_text(
        report_markdown(
            comparisons=compared,
            arity_rows=rows,
            catalogue=catalogues[-1].stem[:12] if catalogues else "no catalogue recorded",
        ),
        encoding="utf-8",
    )
    (arguments.out / "disagreements.jsonl").write_text(
        disagreement_lines(compared), encoding="utf-8"
    )
    print(f"referee report: done, written to {arguments.out}", file=sys.stderr)
    return 0
