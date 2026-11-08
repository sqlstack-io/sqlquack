"""A small root: the committed catalogue and limits, with only transaction_session obligations."""

from __future__ import annotations

import shutil
from pathlib import Path

from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations_bigquery.constants import FAMILIES
from scripts.syntax_obligations_bigquery.main.build import run_build

ROOT: Path = Path(__file__).parents[4] / "coverage" / "syntax-bigquery"
FAMILY: str = "transaction_session"
POSITIVE_ID: str = "bq.transaction_session.set.set.o001"
SUPPRESS_POSITIVE: str = (
    '[[suppress]]\nform = "bq.transaction_session.set.set"\nkind = "positive"\ntarget = ""\n'
    'note = "the documented form with every required part and no optional part"\n'
    'disposition = "documented"\nreason = "test"\n'
)
RETIRE_POSITIVE: str = (
    f'[[retired]]\nretired_in = "test"\nreason = "test"\nids = ["{POSITIVE_ID}"]\n'
)
MISSING_PARSER: str = "no-parser-here"


def small_root(*, directory: Path, limits: str, retired: str) -> Path:
    """The committed catalogue, one family's obligations, limits.toml plus extras, retired ids."""
    root: Path = directory / "root"
    (root / "catalogue").mkdir(parents=True)
    (root / "obligations").mkdir()
    for family in FAMILIES:
        shutil.copy(ROOT / "catalogue" / f"{family}.toml", root / "catalogue" / f"{family}.toml")
    shutil.copy(ROOT / "obligations" / f"{FAMILY}.toml", root / "obligations" / f"{FAMILY}.toml")
    (root / "limits.toml").write_text((ROOT / "limits.toml").read_text() + "\n" + limits)
    (root / "retired.toml").write_text(retired)
    return root


def obligations_text(root: Path) -> str:
    """The family's obligations file as it stands."""
    return (root / "obligations" / f"{FAMILY}.toml").read_text()


def build_error(*, root: Path, parser: Path) -> str:
    """The build's refusal; with no parser, a build the guard passes stops before writing."""
    try:
        run_build(["--root", str(root), "--parser", str(parser)])
    except ObligationError as raised:
        return str(raised)
    return ""
