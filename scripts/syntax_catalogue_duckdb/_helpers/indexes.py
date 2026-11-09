"""The pinned source indexes of the DuckDB catalogue, each with its sha256."""

from __future__ import annotations

import hashlib
from pathlib import Path

from scripts.syntax_catalogue_duckdb._helpers.pages import load_pages
from scripts.syntax_catalogue_duckdb._helpers.review import source_hashes
from scripts.syntax_catalogue_duckdb.constants import (
    DOCS_VERSION,
    ENGINE_COMMIT,
    ENGINE_DIR,
    ENGINE_REPOSITORY,
    ENGINE_TAG,
    GRAMMAR_STATEMENTS,
    WEB_COMMIT,
    WEB_REPOSITORY,
)
from scripts.syntax_catalogue_duckdb.types import Row


def index_rows(cache: Path) -> list[Row]:
    """The page list read from the docs tree and the PEG grammar files, each with its sha256."""
    pages: str = "".join(f"{page.path}\n" for page in load_pages(cache))
    grammar: Path = cache / ENGINE_DIR / GRAMMAR_STATEMENTS
    return [
        {
            "name": f"{WEB_REPOSITORY}@{WEB_COMMIT} docs/{DOCS_VERSION} page list",
            "sha256": hashlib.sha256(pages.encode()).hexdigest(),
        },
        {
            "name": f"{ENGINE_REPOSITORY}@{ENGINE_TAG} ({ENGINE_COMMIT}) "
            f"{GRAMMAR_STATEMENTS}/*.gram",
            "sha256": source_hashes(cache=cache, grammar=grammar)["grammar_sha256"],
        },
    ]
