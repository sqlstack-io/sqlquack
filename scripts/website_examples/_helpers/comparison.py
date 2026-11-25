"""Whether shown text matches real output, `…` standing for what the page leaves out."""

from __future__ import annotations

import difflib
import re
from collections.abc import Iterator

from scripts.website_examples.constants import ELISION, PROGRESS_LINE


def shows(*, shown: str, actual: str) -> bool:
    """Whether `shown` is `actual`, a line of only `…` for lines and `…` in a line for its text."""
    pieces: list[str] = []
    for line in shown.split("\n"):
        if line.strip() == ELISION:
            pieces.append(r"[^\n]*(?:\n[^\n]*)*?")
            continue
        pieces.append(r"[^\n]*".join(re.escape(part) for part in line.split(ELISION)))
    return re.fullmatch("\n".join(pieces), actual) is not None


def without_progress(*, shown: str, actual: str) -> str:
    """`actual` without its leading progress line when `shown` leaves that line out."""
    first, _, rest = actual.partition("\n")
    if PROGRESS_LINE.fullmatch(first) and not PROGRESS_LINE.fullmatch(shown.partition("\n")[0]):
        return rest
    return actual


def difference(*, shown: str, actual: str) -> str:
    """The shown text against the real output, as a unified diff."""
    lines: Iterator[str] = difflib.unified_diff(
        shown.split("\n"), actual.split("\n"), "shown on the page", "real output", lineterm=""
    )
    return "\n".join(lines)
