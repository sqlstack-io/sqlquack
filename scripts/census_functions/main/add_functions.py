"""Writes a census project folder's function files from compiled function files."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.census_functions._helpers.project_files import write_functions


def run_add_functions(argv: list[str] | None = None) -> int:
    """Replace --project's functions/sql and functions/python files with --functions' headers."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--functions", type=Path, required=True, help="compiled project functions")
    parser.add_argument(
        "--project", type=Path, required=True, help="the project folder; its sources are kept"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    print("census functions: reading compiled function headers…", file=sys.stderr)
    written, unread = write_functions(compiled=arguments.functions, project=arguments.project)
    print(
        f"census functions: done, {written} functions written, {unread} unread",
        file=sys.stderr,
    )
    return 0
