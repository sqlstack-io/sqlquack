"""Connection settings."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ConnectionSettings:
    """How to reach Snowflake: a named connection plus explicit overrides, never stored in the repo."""

    connection_name: str | None
    warehouse: str | None
    overrides: tuple[tuple[str, str], ...]
