"""Reading the generators' data files: environments, functions and scenarios."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any

from scripts.referee_cases.constants import (
    DATA_DIRECTORY,
    ENVIRONMENTS_DATA,
    FUNCTIONS_DATA,
    SCALAR_SHAPE,
)
from scripts.referee_cases.models import Environment, Function, Table, TableColumn

_DATA: Path = Path(__file__).resolve().parent.parent / DATA_DIRECTORY


def environments() -> dict[str, Environment]:
    """Every generated environment, by name."""
    document: dict[str, Any] = _load(ENVIRONMENTS_DATA)
    parameters: tuple[tuple[str, bool | int | str], ...] = tuple(
        sorted(document["session_parameters"].items())
    )
    return {
        entry["name"]: Environment(
            name=entry["name"],
            tables=tuple(map(_table, entry["table"])),
            session_parameters=parameters,
        )
        for entry in document["environment"]
    }


def functions() -> tuple[Function, ...]:
    """Every function the generator calls, in file order."""
    return tuple(map(_function, _load(FUNCTIONS_DATA)["functions"]))


def argument_texts() -> dict[str, str]:
    """What each argument kind is written as."""
    return dict(_load(FUNCTIONS_DATA)["arguments"])


def sweeps() -> tuple[tuple[str, ...], tuple[str, ...]]:
    """The columns the first argument is swept over, and the fewer the second is."""
    document: dict[str, Any] = _load(FUNCTIONS_DATA)["sweeps"]
    return tuple(document["first"]), tuple(document["second"])


def scenarios(name: str) -> tuple[tuple[str, str, bool], ...]:
    """The name, SQL and whether it is written to compile, of each scenario in a data file."""
    return tuple(
        (entry["name"], entry["sql"], bool(entry.get("valid", False)))
        for entry in _load(name)["scenario"]
    )


def _table(entry: dict[str, Any]) -> Table:
    return Table(name=entry["name"], columns=tuple(map(_column, entry["columns"])))


def _column(entry: dict[str, Any]) -> TableColumn:
    return TableColumn(
        name=entry["name"], data_type=entry["type"], nullable=entry.get("nullable", True)
    )


def _function(entry: dict[str, Any]) -> Function:
    return Function(
        name=entry["name"],
        arguments=tuple(entry["arguments"].split()),
        minimum=entry["minimum"],
        maximum=entry.get("maximum"),
        shape=entry.get("shape", SCALAR_SHAPE),
    )


def _load(name: str) -> dict[str, Any]:
    with (_DATA / name).open("rb") as handle:
        return tomllib.load(handle)
