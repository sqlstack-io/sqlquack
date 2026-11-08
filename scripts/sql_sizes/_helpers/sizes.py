"""Reading sizes files, comparing them and summarising them."""

from __future__ import annotations

import json
import statistics
from pathlib import Path

from scripts.sql_sizes.constants import GROWTH_BYTES, GROWTH_SHARE, LARGE_BYTES, SIZES_FORMAT
from scripts.sql_sizes.models import Allowances
from scripts.sql_sizes.types import SizesFile


def load(*, path: Path) -> dict[str, int]:
    """The sizes in `path`, which must be in the sizes format."""
    file: SizesFile = json.loads(path.read_text())
    if file.get("format") != SIZES_FORMAT:
        raise SystemExit(f"sql sizes: {path} is not in the {SIZES_FORMAT} format")
    return file["cases"]


def write(*, path: Path, sizes: dict[str, int]) -> None:
    """Writes `sizes` to `path` in the sizes format, sorted by case."""
    file: SizesFile = {"format": SIZES_FORMAT, "cases": dict(sorted(sizes.items()))}
    path.write_text(json.dumps(file, indent=2) + "\n")


def growth(*, base: dict[str, int], measured: dict[str, int], allowed: Allowances) -> list[str]:
    """Cases that grew beyond their allowance, and the shared total if it grew beyond its own."""
    shared: list[str] = sorted(base.keys() & measured.keys())
    ruled: list[str] = [
        case for case in shared if measured[case] <= allowed.cases.get(case, base[case])
    ]
    failures: list[str] = [
        f"{case} grew from {base[case]} B to {measured[case]} B"
        for case in shared
        if case not in ruled
        and measured[case] - base[case] > max(GROWTH_BYTES, int(base[case] * GROWTH_SHARE))
    ]
    before: int = sum(base[case] for case in shared)
    after: int = sum(measured[case] for case in shared)
    ruled_growth: int = sum(max(0, measured[case] - base[case]) for case in ruled)
    if after - ruled_growth - allowed.total > before:
        failures.append(
            f"the {len(shared)} cases both measure grew from {before} B to {after} B, "
            f"{ruled_growth} B of it ruled and {allowed.total} B more allowed"
        )
    return failures


def summary(*, sizes: dict[str, int]) -> str:
    """Count, total, median, 90th percentile, largest and the counts above LARGE_BYTES."""
    values: list[int] = sorted(sizes.values())
    if not values:
        return "no converted cases"
    ninetieth: int = values[min(len(values) - 1, (len(values) * 9) // 10)]
    above: str = ", ".join(
        f"{_count_above(values=values, limit=limit)} over {limit} B" for limit in LARGE_BYTES
    )
    return (
        f"{len(values)} cases, {sum(values)} B in all, median {statistics.median_low(values)} B, "
        f"90th percentile {ninetieth} B, largest {values[-1]} B, {above}"
    )


def _count_above(*, values: list[int], limit: int) -> int:
    """How many of `values` exceed `limit`."""
    return len([value for value in values if value > limit])
