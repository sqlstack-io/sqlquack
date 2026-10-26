"""Every probe of one relation measured over its sample."""

from __future__ import annotations

from pathlib import Path

import duckdb

from scripts.guard_measurement._helpers.conversion import probe_bodies
from scripts.guard_measurement._helpers.evaluation import evaluate, load_sample
from scripts.guard_measurement._helpers.sources import sample_file, sample_types, typed_arguments
from scripts.guard_measurement._helpers.tally import tally
from scripts.guard_measurement.constants import BARE_ARGUMENT, SEMI_STRUCTURED_TYPES
from scripts.guard_measurement.models import ConvertedProbe, Evaluation, Probe, ProbeResult
from scripts.guard_measurement.types import Converter, ProbeStatus


def measure_relation(
    *,
    samples: Path,
    probes: list[Probe],
    convert: Converter,
    limits: tuple[int, int],
) -> list[ProbeResult]:
    """Each probe's outcome; every probe here reads the same relation."""
    relation: str = probes[0].relation
    parquet: Path = sample_file(samples=samples, relation=relation)
    if not parquet.is_file():
        return [_unmeasured(probe=probe, status=ProbeStatus.NO_SAMPLE, rows=0) for probe in probes]
    recorded: dict[str, str] = sample_types(samples=samples, relation=relation)
    connection: duckdb.DuckDBPyConnection = duckdb.connect()
    rows: int = load_sample(connection=connection, parquet=parquet)
    present: set[str] = {
        str(row[0]).upper() for row in connection.execute("DESCRIBE sample_rows").fetchall()
    }
    results: list[ProbeResult] = [
        _measure(
            connection=connection,
            probe=probe,
            columns=typed_arguments(probe=probe, recorded=recorded),
            context=(present, rows, convert, limits),
        )
        for probe in probes
    ]
    connection.close()
    return results


def _measure(
    *,
    connection: duckdb.DuckDBPyConnection,
    probe: Probe,
    columns: tuple[tuple[str, str], ...] | None,
    context: tuple[set[str], int, Converter, tuple[int, int]],
) -> ProbeResult:
    """One probe: skipped, refused, failed, or measured."""
    present, rows, convert, limits = context
    skipped: str | None = _skip_reason(probe=probe, columns=columns, present=present)
    if skipped is not None or columns is None:
        return _unmeasured(probe=probe, status=ProbeStatus.SKIPPED, rows=rows, detail=skipped or "")
    sample_columns: tuple[tuple[str, str], ...] = tuple(
        (f"X{index}", data_type) for index, (_, data_type) in enumerate(columns, start=1)
    )
    refusals: list[str] = []
    for body in probe_bodies(expression=probe.expression, semi_structured=probe.semi_structured):
        converted: ConvertedProbe | str = convert(body=body, columns=sample_columns)
        if isinstance(converted, ConvertedProbe):
            return _evaluated(
                connection=connection,
                probe=probe,
                converted=converted,
                setting=(columns, rows, limits),
            )
        refusals.append(converted)
    return _unmeasured(
        probe=probe, status=ProbeStatus.REFUSED, rows=rows, detail="; ".join(refusals)
    )


def _evaluated(
    *,
    connection: duckdb.DuckDBPyConnection,
    probe: Probe,
    converted: ConvertedProbe,
    setting: tuple[tuple[tuple[str, str], ...], int, tuple[int, int]],
) -> ProbeResult:
    """The probe's raises over the sample, or the DuckDB error that kept it from running."""
    columns, rows, limits = setting
    try:
        evaluation: Evaluation = evaluate(
            connection=connection,
            converted=converted,
            columns=columns,
            aggregate=probe.aggregate,
            limits=limits,
        )
    except duckdb.Error as error:
        return _unmeasured(
            probe=probe, status=ProbeStatus.FAILED, rows=rows, detail=str(error).splitlines()[0]
        )
    reasons, label_rows, label_row_ids = tally(
        found=evaluation.found,
        divergence=probe.divergence,
        raising=len(evaluation.raising_rows),
    )
    return ProbeResult(
        probe=probe,
        status=ProbeStatus.MEASURED,
        rows=rows,
        exact=evaluation.exact,
        detail="",
        reasons=reasons,
        raising_row_ids=evaluation.raising_rows,
        label_rows=label_rows,
        label_row_ids=label_row_ids,
        reasons_estimated=not evaluation.reasons_exact,
    )


def _skip_reason(
    *, probe: Probe, columns: tuple[tuple[str, str], ...] | None, present: set[str]
) -> str | None:
    """Why the probe cannot be measured on this sample, or None."""
    if columns is None:
        return "an argument's type is unknown"
    if any(name.upper() not in present for name, _ in columns):
        return "a column is missing from the sample"
    bare: bool = BARE_ARGUMENT.fullmatch(probe.expression) is not None
    if bare and not columns[0][1].upper().startswith(SEMI_STRUCTURED_TYPES):
        return "a read of a value that is not semi-structured"
    return None


def _unmeasured(*, probe: Probe, status: str, rows: int, detail: str = "") -> ProbeResult:
    """A probe that did not run."""
    return ProbeResult(
        probe=probe,
        status=status,
        rows=rows,
        exact=True,
        detail=detail,
        reasons=(),
        raising_row_ids=frozenset(),
        label_rows={},
        label_row_ids={},
        reasons_estimated=False,
    )
