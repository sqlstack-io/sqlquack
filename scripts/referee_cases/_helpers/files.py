"""Referee files and environments as JSON text, written the same way every time."""

from __future__ import annotations

import json

from scripts.referee_cases._helpers.ddl import setup_sql
from scripts.referee_cases.constants import REFEREE_FORMAT
from scripts.referee_cases.models import Environment, RefereeFile, Statement


def environment_text(environment: Environment) -> str:
    """An environment: its session parameters and the setup statements creating its tables."""
    document: dict[str, object] = {
        "format": REFEREE_FORMAT,
        "name": environment.name,
        "session_parameters": dict(environment.session_parameters),
        "setup_sql": list(setup_sql(environment)),
    }
    return json.dumps(document, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def file_text(referee_file: RefereeFile) -> str:
    """A referee file: its header, then one statement per line."""
    header: dict[str, object] = {
        "format": REFEREE_FORMAT,
        "origin": referee_file.origin,
        "environment": referee_file.environment,
        "syntax_only": referee_file.syntax_only,
    }
    lines: list[str] = [_line(statement) for statement in referee_file.statements]
    head: str = json.dumps(header, sort_keys=True, ensure_ascii=False)[:-1]
    return head + ', "statements": [\n' + ",\n".join(lines) + "\n]}\n"


def _line(statement: Statement) -> str:
    document: dict[str, object] = {
        "name": statement.name,
        "sql": statement.sql,
        "kind": statement.kind,
    }
    if statement.mutation is not None:
        document["mutation"] = statement.mutation
        document["base"] = statement.base
    return "  " + json.dumps(document, sort_keys=True, ensure_ascii=False)
