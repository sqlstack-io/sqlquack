"""Probe results as a JSON report and a short text summary; no sampled value is written."""

from __future__ import annotations

from collections import defaultdict

from scripts.guard_measurement.constants import (
    SHAPES_PER_REASON,
    SUMMARY_COLUMNS,
    SUMMARY_REASONS,
)
from scripts.guard_measurement.models import ProbeResult, ReasonCount
from scripts.guard_measurement.types import ProbeStatus


def report(*, results: list[ProbeResult]) -> dict[str, object]:
    """Per probe, per divergence and per source column: rows, raising rows, share, reasons."""
    measured: list[ProbeResult] = [
        result for result in results if result.status == ProbeStatus.MEASURED
    ]
    columns: list[dict[str, object]] = _by_column(results=measured)
    return {
        "probes": [_probe_entry(result=result) for result in results],
        "by_divergence": _by_divergence(results=results, columns=columns),
        "by_column": columns,
    }


def summary(*, document: dict[str, object]) -> str:
    """Divergence totals, the columns raising most and their top reasons."""
    lines: list[str] = [
        "divergence: probes measured, source-column rows checked, rows raising (share)"
    ]
    totals: object = document["by_divergence"]
    for name, counted in sorted(totals.items()) if isinstance(totals, dict) else []:
        share: str = _percent(part=counted["raising_rows"], whole=counted["rows"])
        lines.append(
            f"  {name}: {counted['measured']} of {counted['probes']} probes, "
            f"{counted['rows']} rows, {counted['raising_rows']} raising ({share})"
        )
    listed: object = document["by_column"]
    ranked: list[dict[str, object]] = sorted(
        (
            column
            for column in (listed if isinstance(listed, list) else [])
            if int(str(column["raising_rows"])) > 0
        ),
        key=lambda column: (-float(str(column["share"])), str(column["relation"])),
    )
    lines.append(f"columns raising most (top {SUMMARY_COLUMNS} of {len(ranked)} raising):")
    lines.extend(_column_line(column=column) for column in ranked[:SUMMARY_COLUMNS])
    return "\n".join(lines)


def progress(*, results: list[ProbeResult]) -> str:
    """One relation's measured probes and its worst raising share with that probe's top reason."""
    measured: list[ProbeResult] = [
        result for result in results if result.status == ProbeStatus.MEASURED
    ]
    worst: ProbeResult | None = max(measured, key=_raising_count, default=None)
    counted: str = f"{len(measured)}/{len(results)} probes measured"
    if worst is None or not worst.raising_row_ids:
        return f"{counted}, none raising"
    share: str = _percent(part=len(worst.raising_row_ids), whole=worst.rows)
    bound: str = "" if worst.exact else " at least"
    reason: str = (
        ""
        if not worst.reasons
        else f"; top reason {worst.reasons[0].reason}"
        + (" (estimated share)" if worst.reasons_estimated else "")
    )
    return (
        f"{counted}, worst {worst.probe.divergence} {len(worst.raising_row_ids)}/{worst.rows}"
        f"{bound} ({share}){reason}"
    )


def _raising_count(result: ProbeResult) -> int:
    """How many sample rows raise for the probe."""
    return len(result.raising_row_ids)


def _column_line(*, column: dict[str, object]) -> str:
    """One column's share and top reasons."""
    reasons: object = column["reasons"]
    top: str = "; ".join(
        f"{entry['reason']} ×{entry['rows']}"
        for entry in (reasons if isinstance(reasons, list) else [])[:SUMMARY_REASONS]
    )
    share: str = _percent(part=column["raising_rows"], whole=column["rows"])
    estimated: str = " (estimated from examples)" if column["estimated"] else ""
    return (
        f"  {column['divergence']} {column['relation']}.{column['column']}: "
        f"{column['raising_rows']}/{column['rows']} ({share}){estimated} — {top}"
    )


def _probe_entry(*, result: ProbeResult) -> dict[str, object]:
    """One probe's outcome; the raising count is exact, the split by divergence may be estimated."""
    raising: frozenset[int] = result.raising_row_ids
    return {
        "divergence": result.probe.divergence,
        "construct": result.probe.construct,
        "relation": result.probe.relation,
        "expression": result.probe.expression,
        "arguments": [list(argument) for argument in result.probe.arguments],
        "status": result.status,
        "detail": result.detail,
        "rows": result.rows,
        "raising_rows": len(raising),
        "raising_rows_by_divergence": dict(sorted(result.label_rows.items())),
        "share": _share(part=len(raising), whole=result.rows),
        "exact": result.exact,
        "reasons_estimated": result.reasons_estimated,
        "reasons": [_reason_entry(reason=reason) for reason in result.reasons],
    }


def _by_divergence(
    *, results: list[ProbeResult], columns: list[dict[str, object]]
) -> dict[str, dict[str, int]]:
    """Per divergence: its probes, those measured, and source-column rows checked and raising."""
    totals: dict[str, dict[str, int]] = defaultdict(
        lambda: {"probes": 0, "measured": 0, "rows": 0, "raising_rows": 0}
    )
    for result in results:
        entry: dict[str, int] = totals[result.probe.divergence]
        entry["probes"] += 1
        entry["measured"] += result.status == ProbeStatus.MEASURED
    for column in columns:
        counted: dict[str, int] = totals[str(column["divergence"])]
        counted["rows"] += int(str(column["rows"]))
        counted["raising_rows"] += int(str(column["raising_rows"]))
    return dict(totals)


def _by_column(*, results: list[ProbeResult]) -> list[dict[str, object]]:
    """Per source column and divergence: rows on which any probe reading it raises for it."""
    grouped: dict[tuple[str, str], list[ProbeResult]] = defaultdict(list)
    for result in results:
        for name, _ in result.probe.arguments:
            grouped[(result.probe.relation, name)].append(result)
    entries: list[dict[str, object]] = []
    for (relation, column), members in sorted(grouped.items()):
        labels: set[str] = {member.probe.divergence for member in members}
        for member in members:
            labels.update(member.label_rows)
        entries.extend(
            _column_entry(relation=relation, column=column, label=label, members=members)
            for label in sorted(labels)
        )
    return entries


def _column_entry(
    *, relation: str, column: str, label: str, members: list[ProbeResult]
) -> dict[str, object]:
    """One column's rows raising for one divergence across the probes reading it."""
    rows: int = max(member.rows for member in members)
    estimated: bool = any(member.reasons_estimated for member in members)
    raising: int = (
        max((member.label_rows.get(label, 0) for member in members), default=0)
        if estimated
        else len(
            frozenset().union(*(member.label_row_ids.get(label, frozenset()) for member in members))
        )
    )
    merged: dict[tuple[str, str], ReasonCount] = {}
    for member in members:
        for reason in member.reasons:
            key: tuple[str, str] = (reason.origin, reason.reason)
            before: ReasonCount | None = merged.get(key)
            merged[key] = reason if before is None or reason.rows > before.rows else before
    kept: list[ReasonCount] = sorted(
        (reason for reason in merged.values() if reason.divergence == label),
        key=lambda reason: (-reason.rows, reason.reason),
    )
    return {
        "relation": relation,
        "column": column,
        "divergence": label,
        "probes": sum(member.probe.divergence == label for member in members),
        "rows": rows,
        "raising_rows": raising,
        "share": _share(part=raising, whole=rows),
        "exact": all(member.exact for member in members),
        "estimated": estimated,
        "reasons": [_reason_entry(reason=reason) for reason in kept],
    }


def _reason_entry(*, reason: ReasonCount) -> dict[str, object]:
    """One reason with its counts and shapes."""
    return {
        "origin": reason.origin,
        "divergence": reason.divergence,
        "reason": reason.reason,
        "rows": reason.rows,
        "shapes": list(reason.shapes[:SHAPES_PER_REASON]),
    }


def _share(*, part: int, whole: int) -> float:
    """`part` of `whole` as a fraction, 0 for an empty whole."""
    return round(part / whole, 6) if whole else 0.0


def _percent(*, part: object, whole: object) -> str:
    """`part` of `whole` as a percentage."""
    return f"{100 * _share(part=int(str(part)), whole=int(str(whole))):.2f}%"
