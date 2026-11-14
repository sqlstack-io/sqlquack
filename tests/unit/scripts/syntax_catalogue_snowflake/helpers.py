"""Reading the catalogue build's TOML output in its tests."""

from __future__ import annotations

import tomllib
from pathlib import Path


def toml_rows(path: Path, table: str) -> dict[str, dict[str, object]]:
    """The rows of one array of tables in a TOML file, by id."""
    return {str(row["id"]): row for row in tomllib.loads(path.read_text()).get(table, [])}
