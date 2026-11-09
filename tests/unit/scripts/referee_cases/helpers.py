"""Changes applied to generated referee files."""

from __future__ import annotations

import json
from pathlib import Path


def untouched(_: Path) -> None:
    """Changes nothing."""


def edited(repository: Path) -> None:
    """Hand-edits a generated referee file."""
    path: Path = repository / "referee" / "generated" / "names.json"
    path.write_text(path.read_text(encoding="utf-8").replace("orders", "order_rows", 1))


def stray(repository: Path) -> None:
    """Adds a file no generator writes."""
    (repository / "referee" / "generated" / "extra.json").write_text("{}", encoding="utf-8")


def split_layout(*, repository: Path, cases: Path) -> dict[str, bool]:
    """Where the generated cases and the environments landed, as named facts."""
    return {
        "cases written under the cases directory": (cases / "generated" / "names.json").is_file(),
        "environments written in the repository": any(
            (repository / "referee" / "environments").glob("*.json")
        ),
        "no cases in the repository": not (repository / "referee" / "generated").exists(),
        "no environments in the cases directory": not (cases / "environments").exists(),
    }


def duplicate_names(texts: dict[str, str]) -> int:
    """How many statement names repeat within their file, over every generated file."""
    repeats: int = 0
    for text in texts.values():
        names: list[str] = [entry["name"] for entry in json.loads(text).get("statements", [])]
        repeats += len(names) - len(set(names))
    return repeats
