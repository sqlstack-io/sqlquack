"""Fetch the pinned BigQuery sources into a private cache and verify them against the pins."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.syntax_catalogue._helpers.toml_out import write_tables
from scripts.syntax_catalogue_bigquery._helpers.fetch import (
    fetch,
    grammar_rows,
    pin_rows,
    pinned,
    verify,
)
from scripts.syntax_catalogue_bigquery.constants import (
    DEFAULT_ROOT,
    INDEXES_FILE,
    INDEXES_HEADER,
    SOURCES_FILE,
    SOURCES_HEADER,
)


def run_fetch(argv: list[str] | None = None) -> int:
    """Fetch the pages and the grammar checkout; exit 1 when they differ from the pins."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cache", type=Path)
    parser.add_argument("--root", type=Path, default=Path(DEFAULT_ROOT))
    parser.add_argument(
        "--verify-only", action="store_true", help="check an existing cache without fetching"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    if not arguments.verify_only:
        print(json.dumps(fetch(arguments.cache)))
    cache: Path = pinned(arguments.cache)
    if not (arguments.root / SOURCES_FILE).exists():
        arguments.root.mkdir(parents=True, exist_ok=True)
        write_tables(
            path=arguments.root / SOURCES_FILE,
            header=SOURCES_HEADER,
            tables=[("source", pin_rows(cache))],
        )
        write_tables(
            path=arguments.root / INDEXES_FILE,
            header=INDEXES_HEADER,
            tables=[("index", grammar_rows(cache))],
        )
        print(f"wrote the first pin, {arguments.root / SOURCES_FILE}")
        return 0
    problems: list[str] = verify(cache=cache, root=arguments.root)
    print(json.dumps({"differences_from_pins": len(problems)}))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    return 0
