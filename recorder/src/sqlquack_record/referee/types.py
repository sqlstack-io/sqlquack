"""The warehouse session a referee run drives."""

from __future__ import annotations

from collections.abc import Callable
from typing import Protocol

from sqlquack_record.recording.models import ColumnDescription, WarehouseFailure
from sqlquack_record.recording.types import SessionParameters


class RefereeSession(Protocol):
    """A warehouse connection that compiles statements without running them."""

    def warehouse_version(self) -> str:
        """The warehouse version."""
        ...

    def connector(self) -> str:
        """The client name and version."""
        ...

    def compile_failure(self, statement: str) -> WarehouseFailure | None:
        """`EXPLAIN` a statement; absent when it compiles."""
        ...

    def describe(self, statement: str) -> tuple[ColumnDescription, ...] | WarehouseFailure:
        """The result columns of a statement, described without running it."""
        ...

    def execute(self, statement: str) -> None:
        """Run a setup statement; errors raise."""
        ...

    def builtin_functions(self) -> list[dict[str, str]]:
        """`SHOW FUNCTIONS` rows of built-in functions only, every value as text."""
        ...

    def identity(self) -> tuple[tuple[str, str], ...]:
        """The session's account, user, role, warehouse, database and schema names, by kind."""
        ...

    def session_parameters(self) -> dict[str, str]:
        """Every parameter's value as the session reports it, by name."""
        ...

    def close(self) -> None:
        """Close the connection, which also drops its temporary tables."""
        ...


type RefereeSessionFactory = Callable[[SessionParameters], RefereeSession]
