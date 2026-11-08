"""Project functions: SQL ones converted by `sqlquack function` and created in DuckDB."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

import duckdb

from scripts.census_functions._helpers.headers import placeholder, signature
from scripts.census_functions.constants import FUNCTION_SUFFIX, SQL_LANGUAGE
from scripts.census_functions.models import Signature
from scripts.duckshop_measure._helpers.running import refusal_names
from scripts.duckshop_measure.constants import BODY_DELIMITER, PYTHON_LANGUAGE, UNRESOLVED_UDF
from scripts.duckshop_measure.models import FunctionMacro, ProjectFunction
from scripts.duckshop_measure.types import FunctionConverter


def function_body(text: str) -> str:
    """The body between a compiled function's first and last `$$`."""
    first: int = text.find(BODY_DELIMITER)
    last: int = text.rfind(BODY_DELIMITER)
    return text[first + len(BODY_DELIMITER) : last].strip() if last > first >= 0 else ""


def project_functions(directory: Path) -> list[ProjectFunction]:
    """Every compiled function whose header reads, by the census's placeholder name."""
    functions: list[ProjectFunction] = []
    for path in sorted(directory.rglob(f"*{FUNCTION_SUFFIX}")):
        text: str = path.read_text(encoding="utf-8")
        found: Signature | None = signature(text)
        if found is None:
            continue
        name: str = placeholder(stem=path.stem, table=not isinstance(found.returns, str))
        functions.append(
            ProjectFunction(name=name, language=found.language, body=function_body(text))
        )
    return functions


def binary_function_converter(
    *, binary: Path, library: Path, policy: Path, project: Path
) -> FunctionConverter:
    """A converter running `<binary> function - --name … --project <project>` under `policy`."""
    environment: dict[str, str] = {**os.environ, "LD_LIBRARY_PATH": str(library)}

    def convert(function: ProjectFunction) -> FunctionMacro | tuple[str, ...]:
        """The macro of `function`, or its refusals; a crash or unreadable output names it."""
        completed: subprocess.CompletedProcess[str] = subprocess.run(
            (str(binary), "function", "-", "--name", function.name)
            + ("--project", str(project), "--policy", str(policy)),
            input=function.body,
            capture_output=True,
            text=True,
            check=False,
            env=environment,
        )
        try:
            report: dict[str, object] = json.loads(completed.stdout)
        except json.JSONDecodeError:
            return (f"no report: exit {completed.returncode}",)
        body: object = report.get("body")
        if not isinstance(body, str):
            return refusal_names(report.get("refusals")) or ("no macro body",)
        parameters: object = report.get("parameters")
        names: tuple[str, ...] = tuple(map(str, parameters)) if isinstance(parameters, list) else ()
        return FunctionMacro(name=function.name, parameters=names, body=body)

    return convert


def converted_macros(
    *, functions: list[ProjectFunction], convert: FunctionConverter
) -> tuple[list[FunctionMacro], dict[str, tuple[str, ...]]]:
    """Each SQL function's macro, and the refusals of those conversion refused, by name."""
    macros: list[FunctionMacro] = []
    refused: dict[str, tuple[str, ...]] = {}
    for function in functions:
        if function.language.lower() != SQL_LANGUAGE:
            continue
        converted: FunctionMacro | tuple[str, ...] = convert(function)
        if isinstance(converted, FunctionMacro):
            macros.append(converted)
        else:
            refused[function.name] = converted
    return macros, refused


def python_function_names(functions: list[ProjectFunction]) -> frozenset[str]:
    """The placeholder names of the project's Python functions."""
    return frozenset(
        function.name for function in functions if function.language.lower() == PYTHON_LANGUAGE
    )


def create_macros(*, database: Path, macros: list[FunctionMacro]) -> dict[str, str]:
    """Create each macro in `database` for models run there; DuckDB's refusals by name."""
    failed: dict[str, str] = {}
    connection: duckdb.DuckDBPyConnection = duckdb.connect(str(database))
    try:
        for macro in macros:
            try:
                connection.execute(
                    f'CREATE OR REPLACE MACRO "{macro.name}"({", ".join(macro.parameters)}) '
                    f"AS {macro.body}"
                )
            except duckdb.Error as error:
                failed[macro.name] = f"{type(error).__name__}: {error}"
    finally:
        connection.close()
    return failed


def python_calls(*, texts: tuple[str, ...], python: frozenset[str]) -> tuple[str, ...]:
    """The Python functions any of `texts` names or calls as `__udf("stem")`, sorted."""
    joined: str = UNRESOLVED_UDF.sub(
        lambda call: placeholder(stem=call.group(1), table=False), "\n".join(texts)
    )
    return tuple(sorted(name for name in python if re.search(rf"\b{name}\b", joined)))


def project_macros(
    *, arguments: argparse.Namespace, function_converter: FunctionConverter | None
) -> tuple[tuple[FunctionMacro, ...], frozenset[str], dict[str, tuple[str, ...]]]:
    """The project's SQL functions as macros, its Python functions' names, and the refused."""
    if arguments.functions is None:
        return (), frozenset(), {}
    functions: list[ProjectFunction] = project_functions(arguments.functions)
    convert: FunctionConverter = function_converter or binary_function_converter(
        binary=arguments.sqlquack,
        library=arguments.library,
        policy=arguments.policy,
        project=arguments.project,
    )
    macros, refused = converted_macros(functions=functions, convert=convert)
    print(
        f"duckshop measure: {len(macros)} SQL functions created, {len(refused)} refused"
        + "".join(f"\n  {name}: {'; '.join(why)}" for name, why in sorted(refused.items())),
        file=sys.stderr,
    )
    return tuple(macros), python_function_names(functions), refused
