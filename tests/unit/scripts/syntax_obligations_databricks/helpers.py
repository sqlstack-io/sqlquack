"""A small root holding the committed transaction_session catalogue and its obligations."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import cast

from scripts.grammar_obligations.types import Row
from scripts.syntax_obligations_databricks._helpers.cases import runtime_only_forms
from scripts.syntax_obligations_databricks.constants import CASES_DIR, FAMILIES, OBLIGATIONS_DIR

REPOSITORY: Path = Path(__file__).parents[4]
ROOT: Path = REPOSITORY / "coverage" / "syntax-databricks"
BUILD_MODULE: str = "scripts.build_syntax_obligations_databricks"
RAISED: re.Pattern[str] = re.compile(r"^[\w.]+(?:Error|Exception): (.*)$", re.MULTILINE)
FAMILY: str = "transaction_session"
POSITIVE_ID: str = "db.transaction_session.begin.o001"
SUPPRESS_POSITIVE: str = (
    '[[suppress]]\nform = "db.transaction_session.begin"\nkind = "positive"\ntarget = ""\n'
    'note = "the documented form with every required part and no optional part"\n'
    'reason = "test"\n'
)
RETIRE_POSITIVE: str = (
    f'[[retired]]\nretired_in = "test"\nreason = "test"\nids = ["{POSITIVE_ID}"]\n'
)
EMPTY_CATALOGUE: str = "# no forms\n"
IDENTIFIER_CLASS_KIND: str = "identifier_class"
CLASS_NOTE_PREFIXES: tuple[str, ...] = ("reserved:", "keyword:")
NO_NOTE: tuple[str, str] = ("", "")


def small_root(*, directory: Path, limits: str, retired: str) -> Path:
    """One committed family, its obligations and limits.toml, plus extra limits and retired ids."""
    root: Path = directory / "root"
    (root / "catalogue").mkdir(parents=True)
    (root / "obligations").mkdir()
    for family in FAMILIES:
        (root / "catalogue" / f"{family}.toml").write_text(EMPTY_CATALOGUE)
    for part in ("catalogue", "obligations"):
        shutil.copy(ROOT / part / f"{FAMILY}.toml", root / part / f"{FAMILY}.toml")
    (root / "limits.toml").write_text((ROOT / "limits.toml").read_text() + "\n" + limits)
    (root / "retired.toml").write_text(retired)
    return root


def obligations_text(root: Path) -> str:
    """The family's obligations file as written."""
    return (root / "obligations" / f"{FAMILY}.toml").read_text()


def build_process(arguments: list[str]) -> subprocess.CompletedProcess[str]:
    """The build with `arguments`, run from the repository root in a fresh interpreter."""
    return subprocess.run(
        [sys.executable, "-m", BUILD_MODULE, *arguments],
        cwd=REPOSITORY,
        capture_output=True,
        text=True,
        check=False,
    )


def build_error(root: Path) -> str:
    """The message of the exception the build process raised, or the empty string if none."""
    return "".join(RAISED.findall(build_process(["--root", str(root)]).stderr))


def runtime_forms() -> set[str]:
    """Every form documented for Databricks Runtime only, whose positives are observed."""
    found: set[str] = set()
    for family in FAMILIES:
        found |= runtime_only_forms(root=ROOT, family=family)
    return found


def _identifier_class_notes() -> dict[str, tuple[str, str]]:
    """(target, note) of every identifier-class obligation, by id."""
    notes: dict[str, tuple[str, str]] = {}
    for path in (ROOT / OBLIGATIONS_DIR).glob("*.toml"):
        rows: list[Row] = tomllib.loads(path.read_text()).get("obligation", [])
        for row in filter(lambda row: row["kind"] == IDENTIFIER_CLASS_KIND, rows):
            notes[str(row["id"])] = (str(row.get("target", "")), str(row.get("note", "")))
    return notes


def class_cases(*, runtime_only: bool = False) -> list[tuple[str, str, str, str]]:
    """(label, target, expect, sql) of each reserved or keyword identifier case, by form scope."""
    runtime: set[str] = runtime_forms()
    notes: dict[str, tuple[str, str]] = _identifier_class_notes()
    found: list[tuple[str, str, str, str]] = []
    for path in (ROOT / CASES_DIR).glob("*.jsonl"):
        cases: list[Row] = [json.loads(line) for line in path.read_text().splitlines()]
        scoped: filter[Row] = filter(
            lambda case: (str(case["case_id"]).rsplit(".k", 1)[0] in runtime) == runtime_only,
            cases,
        )
        for case in scoped:
            pairs: list[tuple[str, str]] = [
                notes.get(str(identifier), NO_NOTE) for identifier in _texts(case["obligations"])
            ]
            for target, note in filter(lambda pair: pair[1].startswith(CLASS_NOTE_PREFIXES), pairs):
                found.append((note.split(":", 1)[0], target, str(case["expect"]), str(case["sql"])))
    return found


def _texts(value: object) -> list[str]:
    """A JSON list of strings."""
    return cast("list[str]", value)


def refused_cases(found: list[tuple[str, str, str, str]]) -> list[tuple[str, str, str, str]]:
    """The class cases expected to be refused."""
    return list(filter(lambda row: row[2] == "reject", found))


def keyword_expects(found: list[tuple[str, str, str, str]]) -> list[str]:
    """The expectation of each keyword class case."""
    return [expect for _, _, expect, _ in filter(lambda row: row[0] == "keyword", found)]
