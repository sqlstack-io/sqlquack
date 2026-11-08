"""The pinned source indexes of the SQLite catalogue, each with its SHA-256."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import cast

from scripts.syntax_catalogue_sqlite._helpers.fetch import archive_path, member
from scripts.syntax_catalogue_sqlite._helpers.pages import load_pages
from scripts.syntax_catalogue_sqlite.constants import (
    AMALGAMATION,
    ARCHIVES,
    DOWNLOAD_ROOT,
    ENGINE_DIR,
    FETCH_RECORD,
    GRAMMAR_FILES,
    LIBRARY_FILE,
    SOURCE,
    SOURCE_PREFIX,
    SQLITE_VERSION,
)
from scripts.syntax_catalogue_sqlite.types import Row


def index_rows(cache: Path) -> list[Row]:
    """The three archives, the grammar files, the page list and the built library, hashed."""
    rows: list[Row] = [
        {
            "name": f"{DOWNLOAD_ROOT}{file} (SQLite {SQLITE_VERSION}; SHA3-256 {sha3})",
            "sha256": sha256,
        }
        for file, sha256, sha3 in ARCHIVES.values()
    ]
    source: Path = archive_path(cache=cache, name=SOURCE)
    rows += [
        {
            "name": f"{ARCHIVES[SOURCE][0]} {name}",
            "sha256": hashlib.sha256(member(archive=source, name=SOURCE_PREFIX + name)).hexdigest(),
        }
        for name in GRAMMAR_FILES
    ]
    pages, _ = load_pages(cache)
    listing: str = "".join(f"{page.path}\n" for page in pages)
    rows.append(
        {
            "name": f"{ARCHIVES['doc'][0]} page set (one path per line)",
            "sha256": hashlib.sha256(listing.encode()).hexdigest(),
        }
    )
    record: dict[str, object] = json.loads((cache / FETCH_RECORD).read_text())
    rows.append(
        {
            "name": f"{ENGINE_DIR}/{LIBRARY_FILE} built from {ARCHIVES[AMALGAMATION][0]} with "
            f"{' '.join(cast('list[str]', record['compile_flags']))} -lm ({record['compiler']})",
            "sha256": str(record["library_sha256"]),
        }
    )
    return rows
