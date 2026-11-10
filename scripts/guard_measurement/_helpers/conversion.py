"""Probe bodies converted by the sqlquack binary's `function` command."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

from scripts.census_functions._helpers.project_files import sources_text, sql_function_text
from scripts.census_functions.constants import (
    FUNCTION_SUFFIX,
    FUNCTIONS_FOLDER,
    LANGUAGE_FOLDERS,
    SOURCES_FOLDER,
)
from scripts.census_functions.models import Signature
from scripts.guard_measurement.constants import (
    BOOLEAN_TYPE,
    PLAIN_BODY,
    PROBE_FUNCTION,
    PROBE_PROJECT,
    REFUSED_FALLBACK,
    RESULT_COLUMN,
    SAMPLE_RELATION,
    SAMPLE_SOURCES,
    SEMI_STRUCTURED_BODY,
    SQL_LANGUAGE,
    UNUSED_ARGUMENT,
)
from scripts.guard_measurement.models import ConvertedProbe
from scripts.guard_measurement.types import Converter


def probe_bodies(*, expression: str, semi_structured: bool | None) -> list[str]:
    """Bodies to try in order: a semi-structured result is written as JSON, as output would be."""
    semi: str = SEMI_STRUCTURED_BODY.format(expression=expression)
    plain: str = PLAIN_BODY.format(expression=expression)
    choices: dict[bool | None, list[str]] = {True: [semi], False: [plain], None: [semi, plain]}
    return choices[semi_structured]


def write_probe_project(*, directory: Path, columns: tuple[tuple[str, str], ...]) -> None:
    """A project folder in `directory`: the sample relation, and the probe as a table function."""
    sources: Path = directory / SOURCES_FOLDER / SAMPLE_SOURCES
    probe: Path = (
        directory
        / FUNCTIONS_FOLDER
        / LANGUAGE_FOLDERS[SQL_LANGUAGE]
        / f"{PROBE_FUNCTION}{FUNCTION_SUFFIX}"
    )
    sources.parent.mkdir(parents=True)
    probe.parent.mkdir(parents=True)
    sources.write_text(
        sources_text({SAMPLE_RELATION: [list(column) for column in columns]}), encoding="utf-8"
    )
    probe.write_text(
        sql_function_text(
            Signature(
                language=SQL_LANGUAGE,
                arguments=(UNUSED_ARGUMENT,),
                returns=((RESULT_COLUMN, BOOLEAN_TYPE),),
            )
        ),
        encoding="utf-8",
    )


def binary_converter(*, binary: Path, policy: Path) -> Converter:
    """A converter running `<binary> function` with the policy's known divergences."""

    def convert(*, body: str, columns: tuple[tuple[str, str], ...]) -> ConvertedProbe | str:
        """The emitted macro, or the last line of the refusal."""
        with tempfile.TemporaryDirectory() as directory:
            project: Path = Path(directory) / PROBE_PROJECT
            body_path: Path = Path(directory) / "body.sql"
            write_probe_project(directory=project, columns=columns)
            body_path.write_text(body, encoding="utf-8")
            completed: subprocess.CompletedProcess[str] = subprocess.run(
                [
                    str(binary),
                    "function",
                    "--name",
                    PROBE_FUNCTION,
                    "--project",
                    str(project),
                    "--config",
                    str(policy),
                    str(body_path),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
        if completed.returncode != 0:
            return (completed.stderr.strip().splitlines() or [REFUSED_FALLBACK])[-1]
        emitted: dict[str, object] = json.loads(completed.stdout)
        parameters: object = emitted.get("parameters")
        return ConvertedProbe(
            parameters=tuple(str(name) for name in parameters)
            if isinstance(parameters, list)
            else (),
            body=str(emitted["body"]),
        )

    return convert
