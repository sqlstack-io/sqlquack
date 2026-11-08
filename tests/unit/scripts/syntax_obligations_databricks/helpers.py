"""A small root holding the committed transaction_session catalogue and its obligations."""

from __future__ import annotations

import shutil
from pathlib import Path

from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations_databricks.constants import FAMILIES
from scripts.syntax_obligations_databricks.main.build import run_build

ROOT: Path = Path(__file__).parents[4] / "coverage" / "syntax-databricks"
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


def build_error(root: Path) -> str:
    """The build's refusal, or the empty string when it wrote the root."""
    try:
        run_build(["--root", str(root)])
    except ObligationError as raised:
        return str(raised)
    return ""
