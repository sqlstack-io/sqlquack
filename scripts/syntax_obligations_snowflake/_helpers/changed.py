"""The families whose generated obligations can differ since a base revision."""

from __future__ import annotations

import subprocess
import tomllib
from pathlib import Path

from scripts.grammar_obligations.constants import FAMILY_ELISIONS
from scripts.grammar_obligations.types import Row
from scripts.replay_scope._helpers.git import changed
from scripts.syntax_obligations_snowflake.constants import (
    BUILDER_PATHS,
    CATALOGUE_DIR,
    FAMILY_DIRS,
    FAMILY_FILE_DEPTH,
    FAMILY_FILE_SUFFIX,
    LIMITS_FILE,
    LIMITS_TABLES,
)


def changed_families(*, root: Path, base: str, families: list[str]) -> list[str]:
    """The named families whose catalogue, limits rows, generated files or builder changed."""
    top: Path = Path(_git(cwd=root, arguments=("rev-parse", "--show-toplevel")).strip())
    prefix: str = root.resolve().relative_to(top.resolve()).as_posix() + "/"
    paths: list[str] = changed(root=top, base=base)
    if any(path.startswith(BUILDER_PATHS) for path in paths):
        return list(families)
    selected: set[str] = set()
    for path in paths:
        selected |= (
            _file_families(relative=path.removeprefix(prefix)) if path.startswith(prefix) else set()
        )
    if prefix + LIMITS_FILE in paths:
        selected |= _limits_families(root=root, top=top, base=base, relative=prefix + LIMITS_FILE)
    return [family for family in families if family in selected]


def _file_families(*, relative: str) -> set[str]:
    """The families a changed file under the root feeds: its own, plus the elisions' readers."""
    parts: list[str] = relative.split("/")
    if (
        len(parts) != FAMILY_FILE_DEPTH
        or parts[0] not in FAMILY_DIRS
        or not parts[1].endswith(FAMILY_FILE_SUFFIX)
    ):
        return set()
    family: set[str] = {parts[1].removesuffix(FAMILY_FILE_SUFFIX)}
    return family | FAMILY_ELISIONS.keys() if parts[0] == CATALOGUE_DIR else family


def _limits_families(*, root: Path, top: Path, base: str, relative: str) -> set[str]:
    """The families whose rows differ between the merge base's limits.toml and the root's."""
    merge_base: str = _git(cwd=top, arguments=("merge-base", base, "HEAD")).strip()
    before: dict[str, list[tuple[str, Row]]] = _limits_by_family(
        _git(cwd=top, arguments=("show", f"{merge_base}:{relative}"), missing_ok=True)
    )
    path: Path = root / LIMITS_FILE
    after: dict[str, list[tuple[str, Row]]] = _limits_by_family(
        path.read_text() if path.exists() else ""
    )
    return {
        family for family in before.keys() | after.keys() if before.get(family) != after.get(family)
    }


def _limits_by_family(text: str) -> dict[str, list[tuple[str, Row]]]:
    """limits.toml's rows grouped by the family their form id names, in file order per table."""
    grouped: dict[str, list[tuple[str, Row]]] = {}
    document: dict[str, list[Row]] = tomllib.loads(text)
    for table in LIMITS_TABLES:
        for row in document.get(table, []):
            family: str = str(row["form"]).split(".")[1]
            grouped.setdefault(family, []).append((table, row))
    return grouped


def _git(*, cwd: Path, arguments: tuple[str, ...], missing_ok: bool = False) -> str:
    """A Git command's output; with missing_ok, empty when it fails, as for a file not there."""
    result: subprocess.CompletedProcess[str] = subprocess.run(
        ("git", *arguments), cwd=cwd, check=not missing_ok, capture_output=True, text=True
    )
    return result.stdout if result.returncode == 0 else ""
