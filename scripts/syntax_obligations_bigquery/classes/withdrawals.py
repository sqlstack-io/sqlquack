"""`Withdrawals`: the drafts that documented suppressions withdrew while the builder ran."""

from __future__ import annotations

from collections.abc import Callable

from scripts.syntax_obligations._helpers import obligations
from scripts.syntax_obligations.models import Draft
from scripts.syntax_obligations.types import Row

_SHARED_SUPPRESSED: Callable[..., bool] = obligations._suppressed  # noqa: SLF001


class Withdrawals:
    """The drafts `documented` suppressions withdrew, in the order the builder met them."""

    def __init__(self) -> None:
        self.drafts: list[Draft] = []

    def suppressed(self, *, draft: Draft, rows: list[Row]) -> bool:
        """Whether a row withdraws the draft, recording it when one does."""
        found: bool = _SHARED_SUPPRESSED(draft=draft, rows=rows)
        if found:
            self.drafts.append(draft)
        return found
