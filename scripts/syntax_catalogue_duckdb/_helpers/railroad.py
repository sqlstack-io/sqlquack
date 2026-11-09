"""Render evaluated railroad trees in the catalogue's grammar notation."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

from scripts.grammar_catalogue._helpers.grammar import normalize
from scripts.syntax_catalogue_duckdb.constants import (
    EMPTY_PLACEHOLDER,
    NAME_KEYWORD,
    PUNCTUATION,
    RAILROAD_EVALUATOR,
    TREE_CHOICE,
    TREE_KEYWORD,
    TREE_LEAVES,
    TREE_ONE_OR_MORE,
    TREE_OPTIONAL,
    TREE_OPTIONAL_SEQUENCE,
    TREE_PLACEHOLDER,
    TREE_SEQUENCE,
    TREE_ZERO_OR_MORE,
)
from scripts.syntax_catalogue_duckdb.exceptions import DuckDbCatalogueError
from scripts.syntax_catalogue_duckdb.types import Tree


def evaluated(*, railroad: Path, scripts: list[Path]) -> dict[str, Tree]:
    """Every page script's diagrams and definitions, by script path relative to `railroad`."""
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        ["node", str(RAILROAD_EVALUATOR), str(railroad), *map(str, scripts)],
        capture_output=True,
        text=True,
        check=False,
        cwd=railroad.parent,
    )
    if completed.returncode != 0:
        raise DuckDbCatalogueError(f"railroad evaluation failed: {completed.stderr.strip()}")
    trees: dict[str, Tree] = json.loads(completed.stdout)
    return {
        Path(script).resolve().relative_to(railroad.parent.resolve()).as_posix(): tree
        for script, tree in trees.items()
    }


def is_empty(tree: Tree) -> bool:
    """Whether a tree matches nothing: an empty sequence, or the docs' skip marker."""
    if tree["kind"] == TREE_SEQUENCE:
        return all(is_empty(item) for item in tree["items"])
    return tree["kind"] == TREE_PLACEHOLDER and tree["text"] == EMPTY_PLACEHOLDER


def separator_text(separator: Tree | None) -> str:
    """A repetition's separator as grammar text, empty when items follow each other directly."""
    return "" if separator is None else render(separator)


def render(tree: Tree) -> str:
    """The tree in catalogue notation: `[ ]` optional, `{ a | b }` choice, `x [ , x ... ]`."""
    kind: str = tree["kind"]
    text: str = tree.get("text", "")
    if (kind == TREE_KEYWORD and not NAME_KEYWORD.match(text)) or (
        kind == TREE_PLACEHOLDER and PUNCTUATION.match(text)
    ):
        return text
    if kind in TREE_LEAVES:
        return f"<{text}>"
    parts: list[str] = [render(item) for item in tree["items"] if not is_empty(item)]
    body: str = normalize(" ".join(parts))
    if kind == TREE_SEQUENCE:
        return body
    if kind == TREE_OPTIONAL:
        return f"[ {body} ]" if body else ""
    if kind == TREE_OPTIONAL_SEQUENCE:
        return " ".join(f"[ {part} ]" for part in parts)
    if kind == TREE_CHOICE:
        empty: bool = any(is_empty(item) for item in tree["items"])
        alternatives: str = " | ".join(parts)
        if len(parts) == 1:
            return f"[ {alternatives} ]" if empty else alternatives
        return f"[ {alternatives} ]" if empty else f"{{ {alternatives} }}"
    repeat: str = normalize(f"{separator_text(tree.get('separator'))} {body}")
    if kind == TREE_ONE_OR_MORE:
        return f"{body} [ {repeat} ... ]"
    if kind == TREE_ZERO_OR_MORE:
        return f"[ {body} [ {repeat} ... ] ]"
    raise DuckDbCatalogueError(f"unknown railroad node {kind}")
