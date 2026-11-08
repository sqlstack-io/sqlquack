"""What the referee observed for one case, one run, and the account kinds it snapshots."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Failure:
    """A statement the warehouse refused: its error class, SQLSTATE and whole message."""

    error_class: str
    sqlstate: str
    message: str
    parser: bool
    # Empty in a journal written before failures kept their statement.
    statement_id: str = ""


@dataclass(frozen=True)
class Observation:
    """One case's phase, the error that decided it and whether the parser refused it."""

    phase: str
    basis: str
    engine_code: str | None
    parse_only_code: str | None
    message: str | None
    error_class: str | None
    statement_id: str | None


@dataclass(frozen=True)
class Run:
    """One referee run: its id, the stem of its case schemas and where it writes."""

    run_id: str
    schema_stem: str
    out: Path
    private: Path
    seed: int | None


@dataclass(frozen=True)
class SnapshotKind:
    """One kind of account- or catalog-level object the sequential phase snapshots over REST.

    `path` is listed page by page (`paging`); each object of the reply's `items` list is keyed by
    its `name` field and fingerprinted by `fields`, where `a.b` is the sorted `b` of each item of
    list `a`. An object whose `skip` field has the given value, or whose name starts with
    `skip_prefix`, belongs to another kind or to the referee. `variant` names the field that
    selects the object's reviewed drop.
    """

    path: str
    items: str
    name: str
    fields: tuple[str, ...]
    paging: str
    skip: tuple[str, str] | None = None
    skip_prefix: str | None = None
    variant: str | None = None
