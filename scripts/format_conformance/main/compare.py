"""Compare `sqlquack convert --pretty` with `sqlclean format` on the cases' converted SQL."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.format_conformance._helpers.cases import case_queries
from scripts.format_conformance._helpers.formatters import converted, sqlclean_formatted
from scripts.format_conformance._helpers.layouts import compared_layouts, summary_lines
from scripts.format_conformance.constants import CASES_DIR, DIFFS_SHOWN


def run_format_conformance(argv: list[str] | None = None) -> int:
    """Print how often SQLQuack's layout matches SQLClean's; a measurement that exits 0."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sqlquack", default="target/debug/sqlquack")
    parser.add_argument("--sqlclean", default="sqlclean")
    parser.add_argument("--cases", type=Path, default=Path(CASES_DIR))
    parser.add_argument("--diffs", type=Path, help="write every differing case's diff here")
    arguments: argparse.Namespace = parser.parse_args(argv)
    pairs: list[tuple[str, str, str]] = []
    for case_id, query, schema in case_queries(arguments.cases):
        sql: tuple[str, str] | None = converted(
            sqlquack=arguments.sqlquack, query=query, schema=schema
        )
        if sql is not None:
            pairs.append((case_id, *sql))
    theirs: list[str] = sqlclean_formatted(
        sqlclean=arguments.sqlclean, statements=[plain for _, plain, _ in pairs]
    )
    counts, diffs = compared_layouts(pairs=pairs, theirs=theirs)
    print(f"format conformance: {len(pairs)} converted statements from {arguments.cases}")
    for line in summary_lines(counts):
        print(line)
    if arguments.diffs:
        arguments.diffs.write_text("\n\n".join(diffs) + "\n")
    for diff in diffs[:DIFFS_SHOWN]:
        print(f"\n{diff}", file=sys.stderr)
    return 0
