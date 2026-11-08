"""Compiled functions and census relations as the files of a project folder."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.census_functions._helpers.headers import placeholder, signature
from scripts.census_functions.constants import (
    FUNCTION_SUFFIX,
    FUNCTIONS_FOLDER,
    IDENTIFIER_QUOTE,
    LANGUAGE_FOLDERS,
    LANGUAGE_SUFFIXES,
    PLACEHOLDER_BODY,
    PLAIN_IDENTIFIER,
    PYTHON_LANGUAGE,
    SQL_LANGUAGE,
)
from scripts.census_functions.models import Signature


def quoted_identifier(name: str) -> str:
    """`name` in double quotes, so a project declares exactly it: its case, dots and quotes."""
    doubled: str = name.replace(IDENTIFIER_QUOTE, IDENTIFIER_QUOTE * 2)
    return f"{IDENTIFIER_QUOTE}{doubled}{IDENTIFIER_QUOTE}"


def source_columns(columns: list[list[str]]) -> list[dict[str, str]]:
    """Each column of a source under its exact name, with its type."""
    return [{"name": quoted_identifier(column), "type": data_type} for column, data_type in columns]


def sources_text(relations: dict[str, list[list[str]]]) -> str:
    """A source file declaring each relation and its columns under their exact names."""
    sources: list[dict[str, object]] = [
        {"name": quoted_identifier(name), "columns": source_columns(columns)}
        for name, columns in relations.items()
    ]
    return json.dumps({"sources": sources}, indent=1) + "\n"


def typed_names(pairs: tuple[tuple[str, str], ...]) -> str:
    """`NAME TYPE, ...` as a SQL function header lists arguments and columns."""
    return ", ".join(f"{name} {data_type}" for name, data_type in pairs)


def sql_function_text(found: Signature) -> str:
    """A SQL function file declaring `found`'s signature, with a placeholder body."""
    returns: str = (
        found.returns if isinstance(found.returns, str) else f"table ({typed_names(found.returns)})"
    )
    return (
        "FUNCTION (\n"
        f"  arguments ({typed_names(found.arguments)}),\n"
        f"  returns {returns},\n"
        ");\n"
        f"{PLACEHOLDER_BODY}\n"
    )


def python_function_text(found: Signature) -> str:
    """A Python function file declaring `found`'s scalar signature with `@udf(...)`."""
    return (
        "from sqlquack import udf\n\n\n"
        f"@udf(arguments={json.dumps(dict(found.arguments))}, "
        f"returns={json.dumps(found.returns)})\n"
        "def main(*arguments):\n"
        "    return None\n"
    )


def declarable(found: Signature) -> bool:
    """Whether a project can declare `found`: SQL or a Python scalar, its names plain."""
    columns: tuple[tuple[str, str], ...] = () if isinstance(found.returns, str) else found.returns
    names: list[str] = [name for name, _ in (*found.arguments, *columns)]
    return (
        found.language in LANGUAGE_FOLDERS
        and (found.language != PYTHON_LANGUAGE or isinstance(found.returns, str))
        and all(PLAIN_IDENTIFIER.fullmatch(name) for name in names)
    )


def function_files(directory: Path) -> tuple[dict[Path, str], int]:
    """Each compiled function's file by its project path, and how many were not declarable."""
    files: dict[str, tuple[Path, str]] = {}
    unread: int = 0
    for path in sorted(directory.rglob(f"*{FUNCTION_SUFFIX}")):
        found: Signature | None = signature(path.read_text(encoding="utf-8"))
        if found is None or not declarable(found):
            unread += 1
            continue
        name: str = placeholder(stem=path.stem, table=not isinstance(found.returns, str))
        relative: Path = (
            Path(FUNCTIONS_FOLDER)
            / LANGUAGE_FOLDERS[found.language]
            / f"{name}{LANGUAGE_SUFFIXES[found.language]}"
        )
        text: str = (
            sql_function_text(found)
            if found.language == SQL_LANGUAGE
            else python_function_text(found)
        )
        files[name] = (relative, text)
    return dict(files.values()), unread


def write_functions(*, compiled: Path, project: Path) -> tuple[int, int]:
    """Replace `project`'s function files with `compiled`'s; how many written and not declarable."""
    for language, folder in LANGUAGE_FOLDERS.items():
        written: Path = project / FUNCTIONS_FOLDER / folder
        written.mkdir(parents=True, exist_ok=True)
        for stale in written.glob(f"*{LANGUAGE_SUFFIXES[language]}"):
            stale.unlink()
    files, unread = function_files(compiled)
    for relative, text in files.items():
        (project / relative).write_text(text, encoding="utf-8")
    return len(files), unread
