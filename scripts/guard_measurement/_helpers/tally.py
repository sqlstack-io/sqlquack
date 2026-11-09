"""Raising value tuples tallied as reasons and as rows per divergence."""

from __future__ import annotations

from scripts.guard_measurement._helpers.reasons import classify, number_detail, shape
from scripts.guard_measurement.constants import SHAPES_PER_REASON
from scripts.guard_measurement.models import Found, ReasonCount
from scripts.guard_measurement.types import Origin


def tally(
    *, found: tuple[Found, ...], divergence: str, raising: int
) -> tuple[tuple[ReasonCount, ...], dict[str, int], dict[str, frozenset[int]]]:
    """Reasons and rows per label as the examples' shares of `raising`, and examples' rows."""
    rows_by_reason: dict[tuple[str, str], set[int]] = {}
    shapes_by_reason: dict[tuple[str, str], list[str]] = {}
    labels: dict[tuple[str, str], str] = {}
    rows_by_label: dict[str, set[int]] = {}
    for each in found:
        origin, reason = classify(message=each.message)
        reason, token_shape = number_detail(reason=reason, values=each.values)
        key: tuple[str, str] = (origin, reason)
        labels[key] = _label(origin=origin, reason=reason, divergence=divergence)
        rows_by_reason.setdefault(key, set()).update(each.row_ids)
        rows_by_label.setdefault(labels[key], set()).update(each.row_ids)
        example: str | None = token_shape or next(
            (shape(value=value) for value in each.values if value), None
        )
        known: list[str] = shapes_by_reason.setdefault(key, [])
        known.extend(
            [example] if example and example not in known and len(known) < SHAPES_PER_REASON else []
        )
    examined: int = len(frozenset().union(*(each.row_ids for each in found)))

    def scaled(rows: set[int]) -> int:
        return round(raising * len(rows) / examined) if examined else 0

    reasons: list[ReasonCount] = [
        ReasonCount(
            origin=origin,
            reason=reason,
            divergence=labels[(origin, reason)],
            rows=scaled(rows),
            shapes=tuple(shapes_by_reason[(origin, reason)]),
        )
        for (origin, reason), rows in rows_by_reason.items()
    ]
    reasons.sort(key=lambda counted: (-counted.rows, counted.reason))
    return (
        tuple(reasons),
        {label: scaled(rows) for label, rows in rows_by_label.items()},
        {label: frozenset(rows) for label, rows in rows_by_label.items()},
    )


def _label(*, origin: str, reason: str, divergence: str) -> str:
    """The divergence a raise counts against: its guard's, Snowflake's error, or the probed one."""
    by_origin: dict[str, str] = {
        Origin.DIVERGENCE: reason.split(":", 1)[0],
        Origin.SNOWFLAKE_ERROR: Origin.SNOWFLAKE_ERROR,
    }
    return by_origin.get(origin, divergence)
