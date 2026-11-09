"""A small root of the committed transaction_session family, and the build in its own process."""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from pathlib import Path

from scripts.syntax_obligations_sqlite.constants import FAMILIES

REPOSITORY: Path = Path(__file__).parents[4]
ROOT: Path = REPOSITORY / "coverage" / "syntax-sqlite"
BUILD_MODULE: str = "scripts.build_syntax_obligations_sqlite"
RAISED: re.Pattern[str] = re.compile(r"^[\w.]+(?:Error|Exception): (.*)$", re.MULTILINE)
FAMILY: str = "transaction_session"
POSITIVE_ID: str = "sl.transaction_session.begin-stmt.o001"
SUPPRESS_POSITIVE: str = (
    '[[suppress]]\nform = "sl.transaction_session.begin-stmt"\nkind = "positive"\ntarget = ""\n'
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
