"""Compare generated-SQL sizes with the committed baseline and with the base branch's."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.sql_sizes._helpers.allowances import applying
from scripts.sql_sizes._helpers.sizes import growth, load, summary, write
from scripts.sql_sizes.models import Allowances


def run_sql_size_check(argv: list[str] | None = None) -> int:
    """Fail when the baseline is stale, or when SQL grew against the base branch's baseline."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sizes", type=Path, help="the file `sqlquack verify --sql-sizes` wrote")
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--against", type=Path, help="the base branch's baseline")
    parser.add_argument("--allowances", type=Path, help="ruled growth, honoured against --against")
    parser.add_argument("--update", action="store_true", help="rewrite the baseline")
    arguments: argparse.Namespace = parser.parse_args(argv)
    measured: dict[str, int] = load(path=arguments.sizes)
    print(f"sql sizes: {summary(sizes=measured)}", file=sys.stderr)
    failures: list[str] = []
    if arguments.against is not None:
        base: dict[str, int] = load(path=arguments.against)
        print(f"sql sizes: base {summary(sizes=base)}", file=sys.stderr)
        allowed: Allowances = applying(path=arguments.allowances, base=base)
        for entry in allowed.unused:
            print(f"sql sizes: note: the allowance for {entry} applies to nothing", file=sys.stderr)
        failures += growth(base=base, measured=measured, allowed=allowed)
    if arguments.update:
        write(path=arguments.baseline, sizes=measured)
        print(f"sql sizes: wrote {arguments.baseline}", file=sys.stderr)
    elif load(path=arguments.baseline) != measured:
        committed: dict[str, int] = load(path=arguments.baseline)
        changed: int = len(set(committed.items()) ^ set(measured.items()))
        failures.append(
            f"{arguments.baseline} does not hold the measured sizes ({changed} entries differ); "
            "run `make sql-size-baseline` and commit it"
        )
    for failure in failures:
        print(f"sql sizes: {failure}", file=sys.stderr)
    print(f"sql sizes: {'FAILED' if failures else 'passed'}", file=sys.stderr)
    return 1 if failures else 0
