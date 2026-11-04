"""Test cases for the sqlquack Python API."""

from __future__ import annotations

from dataclasses import dataclass

from sqlquack import ConvertOptions, Span


@dataclass(frozen=True)
class CheckTestCase:
    description: str
    sql: str
    expected_ok: bool
    expected_codes: tuple[str, ...]
    expected_positions: tuple[tuple[int, int], ...]


@dataclass(frozen=True)
class RenderTestCase:
    description: str
    sql: str
    path: str
    expected_rendered: str
    expected_help: str | None


@dataclass(frozen=True)
class FailureTestCase:
    description: str
    sql: str
    options: ConvertOptions | None
    expected_message: str


@dataclass(frozen=True)
class CommandLineTestCase:
    description: str
    sql: str
    arguments: tuple[str, ...]
    options: ConvertOptions
    expected_converted: bool


@dataclass(frozen=True)
class TypedDiagnosticTestCase:
    description: str
    sql: str
    options: ConvertOptions
    expected_identity: tuple[str, str, str]
    expected_span: Span
