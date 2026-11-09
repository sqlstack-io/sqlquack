"""Every referee generator, and the files they write."""

from __future__ import annotations

from pathlib import Path

from scripts.referee_cases._helpers.clauses import clause_statements
from scripts.referee_cases._helpers.data import environments
from scripts.referee_cases._helpers.files import environment_text, file_text
from scripts.referee_cases._helpers.functions import function_statements
from scripts.referee_cases._helpers.mutants import mutant_statements
from scripts.referee_cases._helpers.scenarios import name_statements, structured_statements
from scripts.referee_cases.constants import (
    CLAUSES_ORIGIN,
    ENVIRONMENTS_DIRECTORY,
    FUNCTIONS_ORIGIN,
    GENERATED_DIRECTORY,
    GENERATED_ENVIRONMENT,
    MUTANTS_ORIGIN,
    NAMES_ORIGIN,
    STRUCTURED_ENVIRONMENT,
    STRUCTURED_ORIGIN,
)
from scripts.referee_cases.models import RefereeFile, Statement


def referee_files() -> tuple[RefereeFile, ...]:
    """The generated referee files, in the order they are written."""
    functions: tuple[Statement, ...] = function_statements()
    names: tuple[Statement, ...] = name_statements()
    return (
        _file(name="functions", origin=FUNCTIONS_ORIGIN, statements=functions),
        _file(name="clauses", origin=CLAUSES_ORIGIN, statements=clause_statements()),
        _file(name="names", origin=NAMES_ORIGIN, statements=names),
        _file(
            name="mutants",
            origin=MUTANTS_ORIGIN,
            statements=mutant_statements((*functions, *names)),
        ),
        _file(
            name="structured",
            origin=STRUCTURED_ORIGIN,
            statements=structured_statements(),
            environment=STRUCTURED_ENVIRONMENT,
        ),
    )


def generated_texts() -> dict[str, str]:
    """Every generated path below `referee/` and its text, environments included."""
    texts: dict[str, str] = {
        referee_file.path: file_text(referee_file) for referee_file in referee_files()
    }
    for environment in environments().values():
        texts[f"{ENVIRONMENTS_DIRECTORY}/{environment.name}.json"] = environment_text(environment)
    return texts


def stale_files(*, directory: Path, cases: Path, texts: dict[str, str]) -> list[str]:
    """Generated files that differ from `texts`, are missing, or no generator writes."""
    existing: set[str] = {
        path.relative_to(cases).as_posix() for path in (cases / GENERATED_DIRECTORY).glob("*.json")
    } | {
        name
        for name in texts
        if name.startswith(ENVIRONMENTS_DIRECTORY) and (directory / name).is_file()
    }
    return sorted(
        name
        for name in set(texts) | existing
        if name not in texts
        or name not in existing
        or _path(directory=directory, cases=cases, name=name).read_text() != texts[name]
    )


def write_files(*, directory: Path, cases: Path, texts: dict[str, str]) -> None:
    """Write `texts` below `directory` and `cases`, removing generated files no generator writes."""
    for path in (cases / GENERATED_DIRECTORY).glob("*.json"):
        if path.relative_to(cases).as_posix() not in texts:
            path.unlink()
    for name, text in texts.items():
        path: Path = _path(directory=directory, cases=cases, name=name)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)


def _path(*, directory: Path, cases: Path, name: str) -> Path:
    """Where a generated file lives: cases under `cases`, environments under `directory`."""
    return (cases if name.startswith(f"{GENERATED_DIRECTORY}/") else directory) / name


def _file(
    *,
    name: str,
    origin: str,
    statements: tuple[Statement, ...],
    environment: str = GENERATED_ENVIRONMENT,
) -> RefereeFile:
    return RefereeFile(
        path=f"{GENERATED_DIRECTORY}/{name}.json",
        origin=origin,
        environment=environment,
        syntax_only=False,
        statements=statements,
    )
