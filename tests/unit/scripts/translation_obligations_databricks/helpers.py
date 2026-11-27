"""Fixtures for the Databricks translation obligation build tests."""

from __future__ import annotations

from pathlib import Path

CATALOGUE: str = """
[[form]]
id = "db.functions.abs"
status = "catalogued"

[[form]]
id = "db.functions.gone"
status = "absent_from_source"
"""
CASE_FILE: str = """
construct = "{form}"

[[case]]
id = "t001"
description = "types"
columns = [{{ name = "x1", type = "INT" }}]
tags = ["type-combination"]

[[case]]
id = "t002"
description = "boundary"
columns = [{{ name = "x1", type = "INT" }}]
tags = ["boundary"]

[[case]]
id = "t003"
description = "null"
columns = [{{ name = "x1", type = "INT" }}]
tags = ["null", "hand-written"]
"""


def small_root(*, root: Path) -> tuple[Path, Path, Path]:
    """A catalogue, a live and an absent form's cases, and an output directory under `root`."""
    catalogue: Path = root / "catalogue"
    cases: Path = root / "cases"
    functions: Path = cases / "translation" / "functions"
    functions.mkdir(parents=True)
    catalogue.mkdir()
    (catalogue / "functions.toml").write_text(CATALOGUE)
    (functions / "abs.toml").write_text(CASE_FILE.format(form="db.functions.abs"))
    (functions / "gone.toml").write_text(CASE_FILE.format(form="db.functions.gone"))
    return catalogue, cases, root / "out"
