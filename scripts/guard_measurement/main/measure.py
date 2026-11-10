"""Measures how often each guarded expression raises over sampled source rows."""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

from scripts.guard_measurement._helpers.conversion import binary_converter
from scripts.guard_measurement._helpers.relation import measure_relation
from scripts.guard_measurement._helpers.report import progress, report, summary
from scripts.guard_measurement._helpers.sources import read_probes
from scripts.guard_measurement.constants import DEFAULT_EXAMPLE_LIMIT, DEFAULT_QUERY_LIMIT
from scripts.guard_measurement.models import Probe, ProbeResult
from scripts.guard_measurement.types import Converter, ProbeStatus


def run_measure(*, argv: list[str] | None = None, converter: Converter | None = None) -> int:
    """Write the JSON report to --out and print the text summary."""
    arguments: argparse.Namespace = _parser().parse_args(argv)
    convert: Converter = converter or binary_converter(
        binary=arguments.sqlquack, policy=arguments.config
    )
    probes: list[Probe] = read_probes(path=arguments.sources)
    print(f"guard measurement: measuring {len(probes)} probes…", file=sys.stderr)
    by_relation: dict[str, list[Probe]] = {}
    for probe in probes:
        by_relation.setdefault(probe.relation, []).append(probe)
    results: list[ProbeResult] = []
    started: float = time.monotonic()
    for number, (relation, members) in enumerate(sorted(by_relation.items()), start=1):
        relation_started: float = time.monotonic()
        measured: list[ProbeResult] = measure_relation(
            samples=arguments.samples,
            probes=members,
            convert=convert,
            limits=(arguments.examples, arguments.query_limit),
        )
        results.extend(measured)
        print(
            f"guard measurement: [{number}/{len(by_relation)}] {relation}: "
            f"{progress(results=measured)} ({time.monotonic() - relation_started:.0f}s; "
            f"{len(results)}/{len(probes)} probes, {time.monotonic() - started:.0f}s total)",
            file=sys.stderr,
            flush=True,
        )
    document: dict[str, object] = report(results=results)
    arguments.out.write_text(json.dumps(document, indent=1), encoding="utf-8")
    print(summary(document=document))
    measured: int = sum(result.status == ProbeStatus.MEASURED for result in results)
    print(
        f"guard measurement: done, {measured} of {len(results)} probes measured, "
        f"report in {arguments.out}",
        file=sys.stderr,
    )
    return 0


def _parser() -> argparse.ArgumentParser:
    """The command line: sources file, samples directory, report path and the binary."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(
        description="Run SQLQuack's emitted guard SQL over Parquet samples (docs/architecture.md)."
    )
    parser.add_argument("--sources", type=Path, required=True)
    parser.add_argument("--samples", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--sqlquack", type=Path, default=Path("sqlquack"))
    parser.add_argument("--config", type=Path, default=Path("sqlquack.toml"))
    parser.add_argument(
        "--examples",
        type=int,
        default=DEFAULT_EXAMPLE_LIMIT,
        help="raising tuples per probe evaluated alone for their reasons; shares scale from them",
    )
    parser.add_argument(
        "--query-limit",
        type=int,
        default=DEFAULT_QUERY_LIMIT,
        help="bisection queries per probe, only for probes DuckDB cannot count with TRY",
    )
    return parser
