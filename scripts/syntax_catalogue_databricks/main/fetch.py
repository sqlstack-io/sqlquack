"""Fetch the pinned Databricks SQL language reference and Spark grammar into a private cache."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.syntax_catalogue_databricks._helpers.fetch import fetch
from scripts.syntax_catalogue_databricks._helpers.pages import pin_differences
from scripts.syntax_catalogue_databricks.constants import FETCH_RECORD


def run_fetch(argv: list[str] | None = None) -> int:
    """Download every reference page the sitemap lists and the pinned Spark parser files; with
    --root, report how the fetch differs from that root's pinned sources and exit 1 if it does."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cache", type=Path)
    parser.add_argument("--root", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    cache: Path = fetch(arguments.cache)
    print(cache)
    if arguments.root is None:
        return 0
    record: dict[str, object] = json.loads((cache / FETCH_RECORD).read_text())
    found: list[str] = pin_differences(
        cache=cache,
        root=arguments.root,
        urls=sorted(record["pages"]),  # type: ignore[arg-type]
    )
    print(json.dumps({"differences_from_pins": len(found)}))
    if found:
        print("\n".join(found), file=sys.stderr)
        return 1
    return 0
