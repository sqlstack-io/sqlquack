"""Models of the BigQuery obligation and case builders."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Verdict:
    """What the pinned parser did with one case text: parsed it, or refused it with a message."""

    parsed: bool
    message: str


@dataclass(frozen=True)
class EngineEvidence:
    """What the receipts settle: cases whose receipt gives their expectation, and SHAPE texts."""

    confirmed: frozenset[str] = frozenset()
    shaped: frozenset[str] = frozenset()


@dataclass(frozen=True)
class Fixture:
    """One object of the declared fixture set: its name, the DDL that makes it, what it needs."""

    name: str
    ddl: str
    needs: tuple[str, ...] = ()


@dataclass(frozen=True)
class FamilyPlan:
    """One family as the build would write it, with the texts the parser gate still settles."""

    rows: list[dict[str, object]]
    cases: list[dict[str, object]]
    settle: frozenset[str]
    withdrawn: tuple[tuple[str, str], ...]
