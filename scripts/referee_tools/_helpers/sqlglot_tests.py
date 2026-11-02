"""Query statements from sqlglot's Snowflake dialect tests, for syntax verdicts only."""

from __future__ import annotations

import ast
from pathlib import Path

from scripts.referee_tools.constants import QUERY_START, SQLGLOT_CALLS, SYNTAX_ONLY_EXCLUSIONS


def sqlglot_statements(path: Path) -> tuple[list[str], int]:
    """Distinct query statements sqlglot validates, in file order, and how many were left out."""
    tree: ast.Module = ast.parse(path.read_text(encoding="utf-8"))
    found: dict[str, None] = {}
    for node in sorted(ast.walk(tree), key=_position):
        text: str | None = _first_string(node)
        if text is not None:
            found.setdefault(text.strip(), None)
    kept: list[str] = [
        text
        for text in found
        if QUERY_START.match(text) is not None and SYNTAX_ONLY_EXCLUSIONS.search(text) is None
    ]
    return kept, len(found) - len(kept)


def _position(node: ast.AST) -> tuple[int, int]:
    return getattr(node, "lineno", 0), getattr(node, "col_offset", 0)


def _first_string(node: ast.AST) -> str | None:
    if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Attribute):
        return None
    if node.func.attr not in SQLGLOT_CALLS or not node.args:
        return None
    argument: ast.expr = node.args[0]
    if isinstance(argument, ast.Constant) and isinstance(argument.value, str):
        return argument.value
    return None
