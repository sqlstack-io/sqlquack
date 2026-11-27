"""Reading the catalogue and cases, and writing one family's obligations."""

from __future__ import annotations

import json
import tomllib
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from scripts.translation_obligations_databricks.constants import (
    CASES_PREFIX,
    HEADER,
    KINDS,
    LIVE_STATUS,
    NO_INPUT,
    READING,
    TRANSLATION_DIR,
)
from scripts.translation_obligations_databricks.types import Case, Entry


def live_forms(*, catalogue: Path) -> set[str]:
    """The ids of the catalogue's forms still in their source."""
    forms: set[str] = set()
    for path in sorted(catalogue.glob("*.toml")):
        for form in tomllib.loads(path.read_text()).get("form", []):
            if form.get("status") == LIVE_STATUS:
                forms.add(str(form["id"]))
    return forms


def _kind(*, case: Case) -> tuple[str, str] | None:
    """The obligation kind and what it catches for the case's first measured tag."""
    kinds: list[tuple[str, str]] = [KINDS[tag] for tag in case["tags"] if tag in KINDS]
    return kinds[0] if kinds else None


def family_entries(*, cases: Path, live: set[str]) -> dict[str, list[Entry]]:
    """Each family's type-combination, NULL and error-parity cases of live forms."""
    families: dict[str, list[Entry]] = defaultdict(list)
    for path in sorted((cases / TRANSLATION_DIR).glob("*/*.toml")):
        document: dict[str, Any] = tomllib.loads(path.read_text())
        form: str = str(document["construct"])
        if form not in live:
            continue
        reference: str = f"{CASES_PREFIX}/{path.relative_to(cases).as_posix()}"
        for case in document["case"]:
            kind: tuple[str, str] | None = _kind(case=case)
            if kind is not None:
                families[path.parent.name].append((form, f"{reference}#{case['id']}", case, kind))
    return families


def family_text(*, family: str, entries: list[Entry]) -> str:
    """The obligations file of one family, numbered per form in case order."""
    numbers: Counter[str] = Counter()
    lines: list[str] = [HEADER.format(family=family).rstrip("\n")]
    for form, reference, case, (kind, catches) in sorted(entries, key=lambda e: (e[0], e[1])):
        numbers[form] += 1
        columns: list[dict[str, str]] = case.get("columns", [])
        target: str = ", ".join(column["type"] for column in columns) or NO_INPUT
        fields: list[tuple[str, str]] = [
            ("id", f"{form}.t{numbers[form]:03d}"),
            ("form", form),
            ("kind", kind),
            ("target", target),
            ("note", str(case["description"])),
            ("catches", catches),
            ("reading", READING),
        ]
        lines += ["", "[[obligation]]"]
        lines += [f"{key} = {json.dumps(value, ensure_ascii=False)}" for key, value in fields]
        lines += [f"cases = [{json.dumps(reference, ensure_ascii=False)}]", "required = true"]
    return "\n".join(lines) + "\n"
