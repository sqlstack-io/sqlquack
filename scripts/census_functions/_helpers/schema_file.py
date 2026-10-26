"""Compiled functions as schema file entries."""

from __future__ import annotations

from pathlib import Path

from scripts.census_functions._helpers.headers import placeholder, signature
from scripts.census_functions.constants import FUNCTION_SUFFIX
from scripts.census_functions.models import Signature


def schema_entry(found: Signature) -> dict[str, object]:
    """One function as the schema file writes it."""
    returns: object = (
        found.returns
        if isinstance(found.returns, str)
        else {"table": [list(pair) for pair in found.returns]}
    )
    return {
        "language": found.language,
        "arguments": [list(pair) for pair in found.arguments],
        "returns": returns,
    }


def compiled_functions(directory: Path) -> tuple[dict[str, dict[str, object]], int]:
    """Every compiled function by its census placeholder name, and how many did not parse."""
    functions: dict[str, dict[str, object]] = {}
    unread: int = 0
    for path in sorted(directory.rglob(f"*{FUNCTION_SUFFIX}")):
        found: Signature | None = signature(path.read_text(encoding="utf-8"))
        if found is None:
            unread += 1
            continue
        name: str = placeholder(stem=path.stem, table=not isinstance(found.returns, str))
        functions[name] = schema_entry(found)
    return functions, unread
