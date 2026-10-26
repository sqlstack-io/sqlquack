"""Probe bodies converted by the sqlquack binary's `function` command."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

from scripts.guard_measurement.constants import (
    BOOLEAN_TYPE,
    PLAIN_BODY,
    PROBE_FUNCTION,
    REFUSED_FALLBACK,
    RESULT_COLUMN,
    SAMPLE_RELATION,
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


def binary_converter(*, binary: Path, policy: Path) -> Converter:
    """A converter running `<binary> function` with the policy's known divergences."""

    def convert(*, body: str, columns: tuple[tuple[str, str], ...]) -> ConvertedProbe | str:
        """The emitted macro, or the last line of the refusal."""
        with tempfile.TemporaryDirectory() as directory:
            schema_path: Path = Path(directory) / "schema.json"
            body_path: Path = Path(directory) / "body.sql"
            schema_path.write_text(json.dumps(_schema(columns=columns)), encoding="utf-8")
            body_path.write_text(body, encoding="utf-8")
            completed: subprocess.CompletedProcess[str] = subprocess.run(
                [
                    str(binary),
                    "function",
                    "--name",
                    PROBE_FUNCTION,
                    "--schema",
                    str(schema_path),
                    "--policy",
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


def _schema(*, columns: tuple[tuple[str, str], ...]) -> dict[str, object]:
    """A schema with the sample relation and the probe as a table function."""
    return {
        "relations": {SAMPLE_RELATION: [list(column) for column in columns]},
        "functions": {
            PROBE_FUNCTION: {
                "language": SQL_LANGUAGE,
                "arguments": [list(UNUSED_ARGUMENT)],
                "returns": {"table": [[RESULT_COLUMN, BOOLEAN_TYPE]]},
            }
        },
    }
