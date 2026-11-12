"""Small roots and repositories for the Snowflake obligation build's tests."""

from __future__ import annotations

import subprocess
import sys
from itertools import filterfalse
from pathlib import Path

from scripts.syntax_obligations_snowflake.main.build import run_build

REPOSITORY: Path = Path(__file__).parents[4]
FAMILY: str = "transaction_session"
CATALOGUE: str = """
[[form]]
id = "sf.transaction_session.use-database"
family = "transaction_session"
kind = "statement"
grammar = ["USE [ DATABASE ] <name>"]
uses = []
status = "catalogued"

[[form]]
id = "sf.transaction_session.use-role"
family = "transaction_session"
kind = "statement"
grammar = ["USE ROLE <name>"]
uses = []
status = "catalogued"
"""
WITHDRAWN_ROW: str = 'form = "sf.transaction_session.use-role"'
ELISION_FAMILY: str = "expressions"
ELISION_CATALOGUE: str = """
[[form]]
id = "sf.expressions.commands-model-function.model-default-version"
family = "expressions"
kind = "statement"
grammar = ["SELECT MODEL( <model_name> )!<method_name>( ... ) FROM <table_name>"]
uses = []
status = "catalogued"
"""
CATALOGUES: dict[str, str] = {FAMILY: CATALOGUE, ELISION_FAMILY: ELISION_CATALOGUE}
SELECTION_ROOT: str = "coverage/syntax"
SELECTION_FAMILIES: tuple[str, ...] = ("ddl", "expressions", "query", "utility")
SELECTION_LIMITS: str = """[[limit]]
form = "sf.ddl.create-table"
kind = "negative"
note = "a ddl limit"

[[suppress]]
form = "sf.query.select"
kind = "positive"
reason = "a query suppression"
"""
BUILDER_MODULE: str = "scripts/grammar_obligations/x.py"
LOADED_MODULES: str = (
    "import importlib, sys\n"
    "importlib.import_module(sys.argv[1])\n"
    "print('\\n'.join(m.__file__ for n, m in sys.modules.items() if n.split('.')[0] == 'scripts'))"
)


def built_root(*, directory: Path, family: str) -> Path:
    """A root holding one family's small catalogue, built once, so its files are as built."""
    root: Path = directory / "root"
    (root / "catalogue").mkdir(parents=True)
    (root / "catalogue" / f"{family}.toml").write_text(CATALOGUES[family])
    (root / "limits.toml").write_text("")
    assert run_build(["--root", str(root), family]) == 0
    return root


def committed_repository(directory: Path) -> Path:
    """A committed repository with each selection family's files, another dialect's and a module."""
    files: dict[str, str] = {
        f"{SELECTION_ROOT}/limits.toml": SELECTION_LIMITS,
        BUILDER_MODULE: "X = 0\n",
    }
    for part in ("catalogue", "obligations", "withdrawn"):
        files |= _family_files(prefix=f"{SELECTION_ROOT}/{part}", part=part)
        files |= _family_files(prefix=f"coverage/syntax-databricks/{part}", part=part)
    for relative, text in files.items():
        (directory / relative).parent.mkdir(parents=True, exist_ok=True)
        (directory / relative).write_text(text)
    _git(directory, "init", "-q", "-b", "main")
    commit(repository=directory, message="base")
    return directory


def edit(*, repository: Path, relative: str, old: str, new: str) -> None:
    """Replace the first old with new in one file of the repository."""
    path: Path = repository / relative
    path.write_text(path.read_text().replace(old, new, 1))


def commit(*, repository: Path, message: str) -> None:
    """Commit every change in the repository."""
    _git(repository, "add", "-A")
    _git(
        repository, "-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-qm", message
    )


def checkout(*, repository: Path, arguments: tuple[str, ...]) -> None:
    """Run git checkout with arguments, as to create or switch a branch."""
    _git(repository, "checkout", "-q", *arguments)


def scripts_outside(*, module: str, allowed: tuple[str, ...]) -> tuple[str, ...]:
    """The files of scripts modules a fresh import of module loads that no allowed prefix covers."""
    output: str = subprocess.run(
        (sys.executable, "-c", LOADED_MODULES, module),
        cwd=REPOSITORY,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    loaded: list[str] = [Path(line).relative_to(REPOSITORY).as_posix() for line in output.split()]
    assert loaded
    return tuple(filterfalse(lambda path: path.startswith(allowed), loaded))


def _family_files(*, prefix: str, part: str) -> dict[str, str]:
    return {f"{prefix}/{family}.toml": f"# {part} {family}\n" for family in SELECTION_FAMILIES}


def _git(repository: Path, *arguments: str) -> None:
    subprocess.run(("git", *arguments), cwd=repository, check=True, capture_output=True)
