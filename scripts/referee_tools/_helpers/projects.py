"""Referee statements from a compiled SQLBuild project, over tables of its declared columns."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from scripts.referee_tools.constants import (
    COMMENT_LINE,
    CURSOR_END,
    CURSOR_START,
    MODEL_KIND,
    PLACEHOLDER,
    RELATION_KINDS,
    RELATION_PLACEHOLDER,
    RELATION_PREFIX,
    STATEMENT_SEPARATOR,
    SYSTEM_FUNCTION,
)
from scripts.referee_tools.models import ProjectCorpus, ProjectStatement


def project_corpus(*, compiled: Path, dag: Path, tags: tuple[str, ...]) -> ProjectCorpus:
    """Each compiled model whose placeholders all name declared relations, tagged by area."""
    document: dict[str, Any] = json.loads(dag.read_text(encoding="utf-8"))
    nodes: dict[str, dict[str, Any]] = {
        node["name"]: node for node in document["nodes"] if node["kind"] in RELATION_KINDS
    }
    declared: dict[str, tuple[dict[str, Any], ...]] = {
        name: tuple(node["columns"]) for name, node in nodes.items() if _typed(node)
    }
    statements: list[ProjectStatement] = []
    skipped: dict[str, int] = {}
    for _, node in sorted(nodes.items()):
        if node["kind"] != MODEL_KIND:
            continue
        reason, statement = _statement(node=node, compiled=compiled, declared=declared, tags=tags)
        if statement is None:
            skipped[reason] = skipped.get(reason, 0) + 1
            continue
        statements.append(statement)
    return ProjectCorpus(statements=tuple(statements), tables=declared, skipped=skipped)


def create_table(*, name: str, columns: tuple[dict[str, Any], ...]) -> str:
    """`CREATE TEMPORARY TABLE` of a relation, prefixed so that a CTE of its name never reads it."""
    rendered: list[str] = [
        f"{column['name']} {column['type']}{'' if column.get('nullable', True) else ' NOT NULL'}"
        for column in columns
    ]
    return f"CREATE TEMPORARY TABLE {RELATION_PREFIX}{name} ({', '.join(rendered)})"


def _statement(
    *,
    node: dict[str, Any],
    compiled: Path,
    declared: dict[str, tuple[dict[str, Any], ...]],
    tags: tuple[str, ...],
) -> tuple[str, ProjectStatement | None]:
    path: Path = compiled / node["path"]
    if not path.is_file():
        return "no compiled file", None
    text: str = "\n".join(
        line
        for line in path.read_text(encoding="utf-8").splitlines()
        if not COMMENT_LINE.match(line)
    )
    relations: frozenset[str] = frozenset(RELATION_PLACEHOLDER.findall(text))
    text = RELATION_PLACEHOLDER.sub(lambda match: RELATION_PREFIX + match.group(1), text)
    text = text.replace(CURSOR_START[0], CURSOR_START[1]).replace(CURSOR_END[0], CURSOR_END[1])
    if PLACEHOLDER.search(text):
        return "calls a project function", None
    if not relations <= declared.keys():
        return "reads a relation without declared column types", None
    if SYSTEM_FUNCTION in text.upper():
        return "calls a system function", None
    if STATEMENT_SEPARATOR in text:
        return "contains a statement separator", None
    tag: str = next((tag for tag in tags if tag in (node.get("tags") or ())), "")
    return "", ProjectStatement(name=node["name"], sql=text.strip(), tag=tag, relations=relations)


def _typed(node: dict[str, Any]) -> bool:
    """Whether a relation declares columns and a type for every one."""
    columns: list[dict[str, Any]] = node.get("columns") or []
    return bool(columns) and all(column.get("type") for column in columns)
