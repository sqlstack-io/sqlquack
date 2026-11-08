"""Time each workload's converted SQL against its hand-written reference on DuckDB."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.sql_benchmark._helpers.timing import timed
from scripts.sql_benchmark.constants import BUDGET_FORMAT, RATIO_HEADROOM, WORKLOADS
from scripts.sql_benchmark.types import BudgetFile, Timing


def run_sql_benchmark(argv: list[str] | None = None) -> int:
    """Fail when a workload's time over its reference grew beyond the recorded ratio's headroom."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sqlquack", type=Path, required=True, help="the sqlquack binary")
    parser.add_argument("--policy", type=Path, default=Path("sqlquack.toml"))
    parser.add_argument("--budget", type=Path, required=True)
    parser.add_argument("--rows", type=int, default=1_000_000)
    parser.add_argument("--update", action="store_true", help="rewrite the budget")
    arguments: argparse.Namespace = parser.parse_args(argv)
    timings: dict[str, Timing] = {
        workload.name: timed(
            workload=workload,
            sqlquack=arguments.sqlquack,
            policy=arguments.policy,
            rows=arguments.rows,
        )
        for workload in WORKLOADS
    }
    for name, timing in timings.items():
        print(
            f"sql benchmark: {name}: {timing['sql_bytes']} B, {timing['seconds']:.3f} s against "
            f"{timing['reference_seconds']:.3f} s hand-written, {timing['ratio']:.1f}x",
            file=sys.stderr,
        )
    if arguments.update:
        written: BudgetFile = {
            "format": BUDGET_FORMAT,
            "rows": arguments.rows,
            "workloads": timings,
        }
        arguments.budget.write_text(json.dumps(written, indent=2) + "\n")
        print(f"sql benchmark: wrote {arguments.budget}", file=sys.stderr)
        return 0
    budget: BudgetFile = json.loads(arguments.budget.read_text())
    failures: list[str] = [
        f"{name} has no recorded ratio in {arguments.budget}; record it with --update"
        for name in timings
        if name not in budget["workloads"]
    ]
    failures += [
        f"{name} takes {timing['ratio']:.1f}x its reference, over {RATIO_HEADROOM}x the "
        f"recorded {budget['workloads'][name]['ratio']:.1f}x"
        for name, timing in timings.items()
        if name in budget["workloads"]
        and timing["ratio"] > budget["workloads"][name]["ratio"] * RATIO_HEADROOM
    ]
    for failure in failures:
        print(f"sql benchmark: {failure}", file=sys.stderr)
    print(f"sql benchmark: {'FAILED' if failures else 'passed'}", file=sys.stderr)
    return 1 if failures else 0
