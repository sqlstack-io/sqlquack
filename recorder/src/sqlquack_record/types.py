"""The warehouse session the recorder drives."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from typing import Protocol

from sqlquack_record.models import SessionValue, StatementResult


class WarehouseSession(Protocol):
    """A warehouse connection with fixed session parameters."""

    def warehouse_version(self) -> str:
        """The warehouse version."""
        ...

    def connector(self) -> str:
        """The client name and version."""
        ...

    def run_batch(self, statements: Sequence[str]) -> list[StatementResult] | None:
        """Run statements in one request; `None` when a statement raised a recordable SQL error."""
        ...

    def run_one(self, statement: str) -> StatementResult:
        """Run one statement, returning a recordable SQL error as an error result."""
        ...

    def close(self) -> None:
        """Close the connection."""
        ...


SessionFactory = Callable[[tuple[tuple[str, SessionValue], ...]], WarehouseSession]
