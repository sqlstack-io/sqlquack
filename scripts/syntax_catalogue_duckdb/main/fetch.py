"""Fetch the pinned DuckDB documentation, grammar and engine catalog into a private cache."""

from __future__ import annotations

import argparse
from pathlib import Path

from scripts.syntax_catalogue_duckdb._helpers.fetch import fetch


def run_fetch(argv: list[str] | None = None) -> int:
    """Check out both pinned trees, evaluate the diagrams and extract the engine catalog."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cache", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print(fetch(arguments.cache))
    return 0
