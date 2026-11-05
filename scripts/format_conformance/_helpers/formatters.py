"""Running `sqlquack convert`, with and without `--pretty`, and `sqlclean format`."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

from scripts.format_conformance.constants import SQLCLEAN_CONFIG


def converted(*, sqlquack: str, query: str, schema: dict) -> tuple[str, str] | None:
    """The DuckDB SQL `sqlquack convert` prints for `query`, plain and with `--pretty`."""
    with tempfile.NamedTemporaryFile("w", suffix=".json") as schema_file:
        json.dump(schema, schema_file)
        schema_file.flush()
        outputs: list[str] = []
        for flags in ([], ["--pretty"]):
            command: list[str] = [sqlquack, "convert", "--utc-session", "--format", "json"]
            command += ["--schema", schema_file.name, *flags, "-"]
            result: subprocess.CompletedProcess[str] = subprocess.run(
                command, input=query, capture_output=True, text=True, check=False
            )
            if result.returncode != 0:
                return None
            outputs.append(json.loads(result.stdout)["sql"])
    return outputs[0], outputs[1]


def sqlclean_formatted(*, sqlclean: str, statements: list[str]) -> list[str]:
    """Each statement as `sqlclean format` lays it out for DuckDB at SQLQuack's indent and width."""
    with tempfile.TemporaryDirectory() as directory:
        root: Path = Path(directory)
        (root / "sqlclean.toml").write_text(SQLCLEAN_CONFIG)
        paths: list[Path] = [root / f"{index:05}.sql" for index in range(len(statements))]
        for path, statement in zip(paths, statements, strict=True):
            path.write_text(statement)
        subprocess.run([sqlclean, "format", directory], cwd=root, capture_output=True, check=False)
        return [path.read_text().rstrip("\n") for path in paths]
