"""Test types for connection arguments."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ConnectArgumentsTestCase:
    description: str
    connection_name: str | None
    warehouse: str | None
    overrides: tuple[tuple[str, str], ...]
    expected_arguments: dict[str, object] | None
