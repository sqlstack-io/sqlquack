"""What the referee submits and observes for one case."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class ErrorClass:
    """One reviewed error class: the reason, a pattern on the message, its code and phase."""

    reason: str
    pattern: str
    code: str
    phase: str


@dataclass(frozen=True)
class Fixture:
    """One fixture of the run: a setup statement of the cases and the object it creates."""

    name: str
    ddl: str


@dataclass(frozen=True)
class RunPaths:
    """Where a run reads its cases and writes its receipts and its private records."""

    cases: tuple[Path, ...]
    out: Path
    private: Path


@dataclass(frozen=True)
class RunNames:
    """The run's project and datasets, which replace the case tokens and never reach a receipt."""

    project: str
    dataset: str
    dataset2: str


@dataclass(frozen=True)
class EngineMessage:
    """One error the engine returned, as it reported it."""

    reason: str
    message: str


@dataclass(frozen=True)
class JobResult:
    """One job: its errors, the statement type and objects a dry run reports, and the raw reply."""

    errors: tuple[EngineMessage, ...]
    statement_type: str
    referenced: tuple[str, ...]
    raw: dict[str, object]


@dataclass(frozen=True)
class Observation:
    """One case's phase, the error that decided it and the dry run's code, or why it is held."""

    phase: str
    basis: str
    engine_code: str | None
    parse_only_code: str | None
    message: str | None
    unmapped: tuple[str, ...] = ()
    held: str = ""
    mode: str = ""
    jobs: tuple[dict[str, object], ...] = ()
