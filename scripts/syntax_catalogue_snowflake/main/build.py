"""Build coverage/syntax/sources.toml and coverage/syntax/catalogue/<family>.toml."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from scripts.grammar_catalogue._helpers.grammar import sha256
from scripts.grammar_catalogue._helpers.toml_out import write_tables
from scripts.grammar_catalogue.constants import (
    FAMILIES,
)
from scripts.grammar_catalogue.models import Catalogue
from scripts.syntax_catalogue_snowflake._helpers.catalogue import (
    build_catalogue,
    refresh_catalogue,
)
from scripts.syntax_catalogue_snowflake._helpers.fold import add_fold_aliases
from scripts.syntax_catalogue_snowflake._helpers.output import write_catalogue


def run_build(argv: list[str] | None = None) -> int:
    """Regenerate the catalogue from the private page cache."""
    arguments: argparse.Namespace = _arguments(argv)
    cache: Path = arguments.cache
    out: Path = arguments.out
    if arguments.refresh:
        return _refresh(cache=cache, out=out)
    catalogue: Catalogue = build_catalogue(cache=cache, out=out, hand_path=out / "hand.toml")
    (out / "catalogue").mkdir(parents=True, exist_ok=True)
    write_tables(
        path=out / "indexes.toml",
        header="# Documentation indexes the pinned page list was read from (llms.txt files and "
        "the docs navigation tree).",
        tables=[
            (
                "index",
                [
                    {"name": f.name, "sha256": sha256(f.read_text())}
                    for f in sorted((cache / "idx").glob("*"))
                ],
            )
        ],
    )
    write_catalogue(out=out, catalogue=catalogue)
    catalogued: set[str] = set()
    for rows in catalogue.forms.values():
        catalogued |= {str(f["id"]) for f in rows}
    add_fold_aliases(
        path=out / "aliases.toml",
        folds={k: v for k, v in catalogue.folds.items() if k in catalogued},
    )
    print(json.dumps({family: len(catalogue.forms[family]) for family in FAMILIES}))
    return 0


def _arguments(argv: list[str] | None) -> argparse.Namespace:
    """The parsed command line: the page cache, the output root and whether to refresh."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=Path("coverage/syntax"))
    parser.add_argument(
        "--refresh",
        action="store_true",
        help="rebuild only the pages the cache holds and keep every other page as committed",
    )
    return parser.parse_args(argv)


def _refresh(*, cache: Path, out: Path) -> int:
    """Rebuild only the cached pages and write the catalogue; the exit code."""
    write_catalogue(
        out=out, catalogue=refresh_catalogue(cache=cache, out=out, hand_path=out / "hand.toml")
    )
    return 0
