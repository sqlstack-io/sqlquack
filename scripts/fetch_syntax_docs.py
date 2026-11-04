"""Fetch the Snowflake documentation pages the syntax catalogue pins into a private cache.

Usage: fetch_syntax_docs.py CACHE INDEX_FILE [INDEX_FILE ...]
"""

from __future__ import annotations

import sys
from pathlib import Path

from scripts.syntax_catalogue.main.fetch import run_fetch

MIN_ARGUMENTS: int = 2


def main(argv: list[str] | None = None) -> int:
    """Fetch every page listed in the llms.txt indexes and navigation trees given."""
    args: list[str] = sys.argv[1:] if argv is None else argv
    if len(args) < MIN_ARGUMENTS:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    return run_fetch(Path(args[0]), [Path(arg) for arg in args[1:]])


if __name__ == "__main__":
    raise SystemExit(main())
