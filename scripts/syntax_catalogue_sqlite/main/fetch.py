"""Fetch the pinned SQLite release archives into a private cache and build the engine."""

from __future__ import annotations

import argparse
from pathlib import Path

from scripts.syntax_catalogue_sqlite._helpers.fetch import fetch


def run_fetch(argv: list[str] | None = None) -> int:
    """Download and verify the three archives, compile the amalgamation and extract its lists."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cache", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print(fetch(arguments.cache))
    return 0
