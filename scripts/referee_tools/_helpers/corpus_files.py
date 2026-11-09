"""Turning a compiled project or sqlglot's tests into referee files below `referee/corpora/`."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.referee_tools._helpers.projects import create_table, project_corpus
from scripts.referee_tools._helpers.sqlglot_tests import sqlglot_statements
from scripts.referee_tools.constants import (
    CORPORA_DIRECTORY,
    ENVIRONMENTS_DIRECTORY,
    JSON_SUFFIX,
    REFEREE_DIRECTORY,
    REFEREE_FORMAT,
    SCENARIO_KIND,
    SESSION_PARAMETERS,
    SQLGLOT_CORPUS,
    SQLGLOT_ORIGIN,
)
from scripts.referee_tools.models import ProjectCorpus, ProjectImport, ProjectStatement


def write_project_corpus(*, repository: Path, project: ProjectImport) -> str:
    """Every `every`th usable model, with tables for the relations they read; a summary."""
    name: str = project.name
    corpus: ProjectCorpus = project_corpus(
        compiled=project.compiled,
        dag=project.dag,
        tags=() if project.split_tag is None else (project.split_tag,),
    )
    kept: tuple[ProjectStatement, ...] = corpus.statements[:: max(project.every, 1)]
    relations: set[str] = set()
    for statement in kept:
        relations.update(statement.relations)
    root: Path = repository / REFEREE_DIRECTORY
    write_environment(
        path=root / ENVIRONMENTS_DIRECTORY / f"{name}{JSON_SUFFIX}",
        name=name,
        session_parameters=dict(SESSION_PARAMETERS),
        setup_sql=[
            create_table(name=relation, columns=corpus.tables[relation])
            for relation in sorted(relations)
        ],
    )
    groups: dict[str, list[dict[str, str]]] = {}
    for statement in kept:
        suffix: str = f"_{statement.tag}" if statement.tag else ""
        groups.setdefault(f"{name}{suffix}", []).append(
            {"name": statement.name, "sql": statement.sql, "kind": SCENARIO_KIND}
        )
    for group, statements in groups.items():
        write_statements(
            path=root / CORPORA_DIRECTORY / f"{group}{JSON_SUFFIX}",
            origin=project.origin,
            environment=name,
            syntax_only=False,
            statements=statements,
        )
    skipped: str = ", ".join(
        f"{count} {reason}" for reason, count in sorted(corpus.skipped.items())
    )
    sizes: str = ", ".join(f"{group} {len(items)}" for group, items in groups.items())
    return f"{sizes} statements over {len(relations)} tables; skipped: {skipped or 'none'}"


def write_sqlglot_corpus(*, repository: Path, tests: Path, revision: str) -> str:
    """sqlglot's query statements as one syntax-only referee file; a summary."""
    statements, excluded = sqlglot_statements(tests)
    root: Path = repository / REFEREE_DIRECTORY
    write_environment(
        path=root / ENVIRONMENTS_DIRECTORY / f"{SQLGLOT_CORPUS}{JSON_SUFFIX}",
        name=SQLGLOT_CORPUS,
        session_parameters=dict(SESSION_PARAMETERS),
        setup_sql=[],
    )
    write_statements(
        path=root / CORPORA_DIRECTORY / f"{SQLGLOT_CORPUS}{JSON_SUFFIX}",
        origin=f"{SQLGLOT_ORIGIN}@{revision}",
        environment=SQLGLOT_CORPUS,
        syntax_only=True,
        statements=[
            {"name": f"statement_{index:04}", "sql": sql, "kind": SCENARIO_KIND}
            for index, sql in enumerate(statements, start=1)
        ],
    )
    return f"{len(statements)} sqlglot statements, {excluded} left out"


def write_environment(
    *,
    path: Path,
    name: str,
    session_parameters: dict[str, bool | int | str],
    setup_sql: list[str],
) -> None:
    """An environment file: session parameters and the setup statements creating its tables."""
    document: dict[str, object] = {
        "format": REFEREE_FORMAT,
        "name": name,
        "session_parameters": session_parameters,
        "setup_sql": setup_sql,
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_statements(
    *,
    path: Path,
    origin: str,
    environment: str,
    syntax_only: bool,
    statements: list[dict[str, str]],
) -> None:
    """A referee file: its header, then one statement per line."""
    header: dict[str, object] = {
        "format": REFEREE_FORMAT,
        "origin": origin,
        "environment": environment,
        "syntax_only": syntax_only,
    }
    head: str = json.dumps(header, sort_keys=True, ensure_ascii=False)[:-1]
    lines: list[str] = [
        "  " + json.dumps(statement, sort_keys=True, ensure_ascii=False) for statement in statements
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(head + ', "statements": [\n' + ",\n".join(lines) + "\n]}\n", encoding="utf-8")
