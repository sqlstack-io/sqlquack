"""Running the sqlquack command line next to the Python API."""

from __future__ import annotations

import json
import os
import subprocess
from operator import attrgetter
from pathlib import Path
from typing import Any

from sqlquack import CheckResult

#: The release command line `make python-api` builds next to the module.
COMMAND_ENV: str = "SQLQUACK_BIN"

#: A schema of one relation, as the `schema` option holds it in memory.
SCHEMA: dict[str, list[list[str]]] = {
    "ORDERS": [["ORDER_ID", "NUMBER(38,0)"], ["PRICE", "NUMBER(10,2)"]]
}

#: The same relation as a project's source file declares it.
SOURCES: str = (
    "sources:\n  - name: orders\n    columns:\n"
    "      - name: order_id\n        type: NUMBER(38,0)\n"
    "      - name: price\n        type: NUMBER(10,2)\n"
)


def source_project(directory: Path) -> Path:
    """`directory` made a project declaring the test relation in `sources/`."""
    (directory / "sources").mkdir(parents=True)
    (directory / "sources" / "orders.yml").write_text(SOURCES)
    return directory


def command_report(sql: str, arguments: tuple[str, ...], directory: Path) -> dict[str, Any]:
    """What `sqlquack convert --json` prints for `sql`, run in `directory`."""
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        [os.environ[COMMAND_ENV], "convert", "--json", *arguments],
        input=sql,
        capture_output=True,
        text=True,
        cwd=directory,
        check=False,
    )
    return json.loads(completed.stdout)


def positions(result: CheckResult) -> tuple[tuple[int, int], ...]:
    """The line and column of each diagnostic of `result`."""
    return tuple(attrgetter("span.line", "span.column")(found) for found in result.diagnostics)
