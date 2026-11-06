"""Fetch the pinned PostgreSQL release into a private, untracked cache."""

from __future__ import annotations

import argparse
from pathlib import Path

from scripts.syntax_catalogue_pg._helpers.fetch import fetch


def run_fetch(argv: list[str] | None = None) -> int:
    """Download, verify and unpack the pinned release tarball."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cache", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print(fetch(arguments.cache))
    return 0
