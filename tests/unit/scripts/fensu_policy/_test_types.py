"""Test types for the repository layout rules."""

from __future__ import annotations

from dataclasses import dataclass

from fensu import RuleFile


@dataclass(frozen=True)
class CrateNameTestCase:
    description: str
    name: str
    directory: str
    expected_fragment: str | None


@dataclass(frozen=True)
class DependencyDialectTestCase:
    description: str
    name: str
    directory: str
    runtime_dependencies: tuple[str, ...]
    expected_pairs: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class EnumTestCase:
    description: str
    source: str
    parser_dialects: tuple[str, ...]
    expected_missing: tuple[str, ...]
    expected_extra: tuple[str, ...]


@dataclass(frozen=True)
class BranchTestCase:
    description: str
    source: str
    expected_lines: tuple[int, ...]


@dataclass(frozen=True)
class EntriesTestCase:
    description: str
    entries: tuple[str, ...]
    expected_entries: tuple[str, ...]


@dataclass(frozen=True)
class ScriptRuleTestCase:
    description: str
    path: str
    source: str
    expected_fault_count: int
    files: tuple[RuleFile, ...] = ()
    scope: str = "tooling"
    scope_root: str | None = "scripts"
