"""SQLQuack's built-in argument-count table against the recorded function catalogue."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from scripts.referee_tools.constants import (
    ARITY_FUNCTION,
    ARITY_NAME,
    RETURN_MARKER,
    SIGNATURES_SOURCE,
    VARIADIC_MARKER,
)


def sqlquack_arities(repository: Path) -> dict[str, tuple[int, int | None]]:
    """Each name in the built-in arity table: at least, and at most when bounded."""
    text: str = (repository / SIGNATURES_SOURCE).read_text(encoding="utf-8")
    body: str = text[text.index("fn builtin_arity") :]
    body = body[: body.index("\n}\n")]
    arities: dict[str, tuple[int, int | None]] = {}
    for match in ARITY_FUNCTION.finditer(body):
        bound: int | None = None if match.group(4) is None else int(match.group(4))
        for name in ARITY_NAME.findall(match.group(1)):
            arities[name] = (int(match.group(2)), bound)
    return arities


def catalogue_arities(path: Path) -> dict[str, tuple[int, int | None]]:
    """Each built-in's fewest and most arguments over its overloads; no bound when variadic."""
    document: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    arities: dict[str, tuple[int, int | None]] = {}
    for function in document["functions"]:
        name: str = function["name"].lower()
        low: int = int(function["min_num_arguments"] or 0)
        high: int | None = _maximum(function)
        known: tuple[int, int | None] = arities.get(name, (low, high))
        arities[name] = (min(known[0], low), _larger(left=known[1], right=high))
    return arities


def arity_mismatches(
    *, sqlquack: dict[str, tuple[int, int | None]], snowflake: dict[str, tuple[int, int | None]]
) -> list[tuple[str, str, str]]:
    """Each SQLQuack entry whose bounds differ from Snowflake's, or that Snowflake lacks."""
    rows: list[tuple[str, str, str]] = []
    for name, bounds in sorted(sqlquack.items()):
        recorded: tuple[int, int | None] | None = snowflake.get(name)
        if recorded != bounds:
            rows.append((name, _text(bounds), "absent" if recorded is None else _text(recorded)))
    return rows


def _maximum(function: dict[str, Any]) -> int | None:
    arguments: str = str(function.get("arguments", ""))
    most: int = int(function["max_num_arguments"] or 0)
    if most < 0 or VARIADIC_MARKER in arguments.split(RETURN_MARKER)[0]:
        return None
    return most


def _larger(*, left: int | None, right: int | None) -> int | None:
    if left is None or right is None:
        return None
    return max(left, right)


def _text(bounds: tuple[int, int | None]) -> str:
    return f"{bounds[0]}..{'n' if bounds[1] is None else bounds[1]}"
