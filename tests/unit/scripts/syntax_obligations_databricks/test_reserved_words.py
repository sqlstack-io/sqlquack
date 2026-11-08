"""The reserved-word cases follow the reserved-words page: only a table alias refuses ANTI."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

from scripts.syntax_obligations_databricks._helpers.cases import runtime_only_forms
from scripts.syntax_obligations_databricks.constants import (
    CASES_DIR,
    FAMILIES,
    OBLIGATIONS_DIR,
)

ROOT: Path = Path(__file__).resolve().parents[4] / "coverage" / "syntax-databricks"


def _runtime_forms() -> set[str]:
    """Every form documented for Databricks Runtime only, whose positives are observed."""
    found: set[str] = set()
    for family in FAMILIES:
        found |= runtime_only_forms(root=ROOT, family=family)
    return found


def _class_cases(*, runtime_only: bool = False) -> list[tuple[str, str, str, str]]:
    """(label, target, expect, sql) of each case of a reserved or keyword identifier obligation,
    of the Runtime-only forms or of the others."""
    runtime: set[str] = _runtime_forms()
    notes: dict[str, tuple[str, str]] = {}
    for path in (ROOT / OBLIGATIONS_DIR).glob("*.toml"):
        for row in tomllib.loads(path.read_text()).get("obligation", []):
            if row["kind"] == "identifier_class":
                notes[row["id"]] = (str(row.get("target", "")), str(row.get("note", "")))
    found: list[tuple[str, str, str, str]] = []
    for path in (ROOT / CASES_DIR).glob("*.jsonl"):
        for line in path.read_text().splitlines():
            case = json.loads(line)
            form: str = str(case["case_id"]).rsplit(".k", 1)[0]
            if (form in runtime) != runtime_only:
                continue
            for identifier in case["obligations"]:
                target, note = notes.get(identifier, ("", ""))
                if note.startswith(("reserved:", "keyword:")):
                    label: str = note.split(":", 1)[0]
                    found.append((label, target, case["expect"], case["sql"]))
    return found


def test_given_the_built_cases_when_read_then_only_unquoted_anti_as_table_alias_is_refused() -> (
    None
):
    found: list[tuple[str, str, str, str]] = _class_cases()
    refused: list[tuple[str, str, str, str]] = [row for row in found if row[2] == "reject"]

    assert any(label == "keyword" for label, _, _, _ in found)
    assert refused
    assert all(
        label == "reserved" and target.endswith("_alias>") and "`ANTI`" not in sql
        for label, target, _, sql in refused
    )
    assert all(expect == "accept" for label, _, expect, _ in found if label == "keyword")


def test_given_runtime_only_forms_when_read_then_their_keyword_cases_are_observed() -> None:
    found: list[tuple[str, str, str, str]] = _class_cases(runtime_only=True)

    assert found
    assert all(expect == "observe" for label, _, expect, _ in found if label == "keyword")
