"""Reading the ruled growth allowances and keeping those that apply to the base baseline."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any

from scripts.sql_sizes.constants import REQUIRED_TEXT
from scripts.sql_sizes.models import Allowances


def applying(*, path: Path | None, base: dict[str, int]) -> Allowances:
    """The allowances in `path` whose `from` matches `base`; the others are reported unused."""
    if path is None or not path.exists():
        return Allowances()
    document: dict[str, Any] = tomllib.loads(path.read_text())
    cases: dict[str, int] = {}
    total: int = 0
    unused: list[str] = []
    for entry in document.get("case", []):
        _require(entry=entry, fields=("case", "from", "to", *REQUIRED_TEXT), path=path)
        if base.get(entry["case"]) == entry["from"]:
            cases[entry["case"]] = entry["to"]
        else:
            unused.append(f"case {entry['case']} from {entry['from']} B ({entry['ruling']})")
    for entry in document.get("total", []):
        _require(entry=entry, fields=("from", "bytes", *REQUIRED_TEXT), path=path)
        if sum(base.values()) == entry["from"]:
            total += entry["bytes"]
        else:
            unused.append(f"total from {entry['from']} B ({entry['ruling']})")
    return Allowances(cases=cases, total=total, unused=tuple(unused))


def _require(*, entry: dict[str, Any], fields: tuple[str, ...], path: Path) -> None:
    """Stops unless `entry` holds every field, with a non-empty reason and ruling."""
    missing: list[str] = [name for name in fields if name not in entry]
    blank: list[str] = [name for name in REQUIRED_TEXT if not str(entry.get(name, "")).strip()]
    if missing or blank:
        raise SystemExit(f"sql sizes: {path}: an allowance lacks {', '.join(missing + blank)}")
