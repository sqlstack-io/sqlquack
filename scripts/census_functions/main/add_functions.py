"""Adds the functions section to a census schema file from compiled function files."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.census_functions._helpers.schema_file import compiled_functions
from scripts.census_functions.constants import FUNCTIONS_KEY, RELATIONS_KEY


def run_add_functions(argv: list[str] | None = None) -> int:
    """Rewrite --schema with a functions section read from --functions; relations are kept."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--functions", type=Path, required=True)
    parser.add_argument("--schema", type=Path, required=True)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print("census functions: reading compiled function headers…", file=sys.stderr)
    loaded: dict[str, object] = json.loads(arguments.schema.read_text(encoding="utf-8"))
    relations: object = loaded.get(RELATIONS_KEY, loaded) if FUNCTIONS_KEY in loaded else loaded
    functions, unread = compiled_functions(arguments.functions)
    arguments.schema.write_text(
        json.dumps({RELATIONS_KEY: relations, FUNCTIONS_KEY: functions}), encoding="utf-8"
    )
    print(
        f"census functions: done, {len(functions)} functions written, {unread} unread",
        file=sys.stderr,
    )
    return 0
