"""Session parameter values and the warehouse session the recorder drives."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from typing import Protocol

from sqlquack_record.recording.models import StatementResult, WarehouseFailure

type SessionValue = bool | int | str
type SessionParameters = tuple[tuple[str, SessionValue], ...]


class WarehouseSession(Protocol):
    """A warehouse connection with fixed session parameters."""

    def warehouse_version(self) -> str:
        """The warehouse version."""
        ...

    def connector(self) -> str:
        """The client name and version."""
        ...

    def run_batch(self, statements: Sequence[str]) -> list[StatementResult] | None:
        """Run statements in one request; `None` asks for a rerun one statement at a time."""
        ...

    def run_one(self, statement: str) -> StatementResult:
        """Run one statement; a recordable SQL error or an unrecordable result is the result."""
        ...

    def compile_failure(self, statement: str) -> WarehouseFailure | None:
        """Compile a statement without running it or creating anything; absent when it compiles."""
        ...

    def execute(self, statement: str) -> None:
        """Run a statement that is not a case, such as creating a case's function; errors raise."""
        ...

    def current_schema(self) -> str | None:
        """`CURRENT_SCHEMA()`, absent when none is in use."""
        ...

    def session_parameters(self) -> dict[str, str]:
        """Every parameter's value as `SHOW PARAMETERS IN SESSION` reports it, by name."""
        ...

    def close(self) -> None:
        """Close the connection."""
        ...


type SessionFactory = Callable[[SessionParameters], WarehouseSession]
