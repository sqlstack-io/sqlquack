"""Comparing the two formatters' layouts of each statement."""

from __future__ import annotations

import difflib


def compared_layouts(
    *, pairs: list[tuple[str, str, str]], theirs: list[str]
) -> tuple[dict[str, int], list[str]]:
    """How many statements each formatter lays out alike, and a diff of each that differs."""
    counts: dict[str, int] = {"declined": 0, "exact": 0, "cased": 0}
    diffs: list[str] = []
    for (case_id, plain, ours), clean in zip(pairs, theirs, strict=True):
        outcome: str | None = layout_outcome(plain=plain, ours=ours, clean=clean)
        if outcome is not None:
            counts[outcome] += 1
            continue
        lines: list[str] = list(
            difflib.unified_diff(
                clean.splitlines(), ours.splitlines(), "sqlclean", "sqlquack", lineterm=""
            )
        )
        diffs.append(f"## {case_id}\n" + "\n".join(lines))
    counts["different"] = len(diffs)
    return counts, diffs


def layout_outcome(*, plain: str, ours: str, clean: str) -> str | None:
    """`declined` when SQLClean left it unformatted, `exact` or `cased` when alike, else None."""
    if clean == plain and ours != plain:
        return "declined"
    if ours == clean:
        return "exact"
    return "cased" if ours.lower() == clean.lower() else None


def summary_lines(counts: dict[str, int]) -> list[str]:
    """The measurement's summary, as percentages of the statements both formatters laid out."""
    compared: int = max(sum(counts.values()) - counts["declined"], 1)
    layout: int = counts["exact"] + counts["cased"]
    return [
        f"  left unformatted by SQLClean, not compared: {counts['declined']}",
        f"  same text:                      {counts['exact']} ({counts['exact'] / compared:.1%})",
        f"  same layout, keyword case only: {counts['cased']} ({counts['cased'] / compared:.1%})",
        f"  same layout in all:             {layout} ({layout / compared:.1%})",
        f"  different layout:               {counts['different']} "
        f"({counts['different'] / compared:.1%})",
    ]
