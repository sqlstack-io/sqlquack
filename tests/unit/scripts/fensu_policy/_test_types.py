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
    gated: frozenset[str] = frozenset()


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


@dataclass(frozen=True)
class FeatureGateTestCase:
    description: str
    manifest: str
    expected_gated: frozenset[str]
    expected_defaults: tuple[str, ...]


@dataclass(frozen=True)
class PathDialectTestCase:
    description: str
    parts: tuple[str, ...]
    expected_dialect: str | None


@dataclass(frozen=True)
class FolderFeatureTestCase:
    description: str
    manifest: str
    folders: frozenset[str]
    expected_missing: tuple[str, ...]


@dataclass(frozen=True)
class SharedLinesTestCase:
    description: str
    source: str
    registry: bool
    expected_lines: tuple[int, ...]
