"""The private source cache: pinned sparse checkouts, evaluated diagrams and the engine extract."""

from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path

import duckdb

from scripts.syntax_catalogue_duckdb._helpers.railroad import evaluated
from scripts.syntax_catalogue_duckdb.constants import (
    DUCKDB_VERSION,
    ENGINE_COMMIT,
    ENGINE_DIR,
    ENGINE_EXTRACT,
    ENGINE_PATHS,
    ENGINE_QUERIES,
    ENGINE_REPOSITORY,
    FETCH_RECORD,
    RAILROAD_DIRS,
    RAILROAD_FILE,
    RAILROAD_ROOT,
    RAILROAD_SHARED,
    UTC_FORMAT,
    WEB_COMMIT,
    WEB_DIR,
    WEB_PATHS,
    WEB_REPOSITORY,
)
from scripts.syntax_catalogue_duckdb.exceptions import DuckDbCatalogueError


def git(*arguments: str, cwd: Path) -> str:
    """Run git and return its output, raising with its message on failure."""
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        ["git", *arguments], cwd=cwd, capture_output=True, text=True, check=False
    )
    if completed.returncode != 0:
        raise DuckDbCatalogueError(f"git {' '.join(arguments)}: {completed.stderr.strip()}")
    return completed.stdout.strip()


def checkout(*, target: Path, repository: str, commit: str, paths: tuple[str, ...]) -> None:
    """A sparse checkout of `paths` at `commit`, verified to be at that commit."""
    if not target.exists():
        target.mkdir(parents=True)
        git("init", "-q", cwd=target)
        git("remote", "add", "origin", repository, cwd=target)
    git("sparse-checkout", "set", "--no-cone", *[f"/{path}/" for path in paths], cwd=target)
    git("fetch", "-q", "--depth", "1", "--filter=blob:none", "origin", commit, cwd=target)
    git("checkout", "-q", "--detach", commit, cwd=target)
    if git("rev-parse", "HEAD", cwd=target) != commit:
        raise DuckDbCatalogueError(f"{target}: not at the pinned commit {commit}")


def engine_extract() -> dict[str, list[list[object]]]:
    """Functions, keywords and types of the pinned engine, from its own catalog functions."""
    if duckdb.__version__ != DUCKDB_VERSION:
        raise DuckDbCatalogueError(
            f"duckdb {duckdb.__version__} is not the pinned {DUCKDB_VERSION}"
        )
    connection: duckdb.DuckDBPyConnection = duckdb.connect()
    extract: dict[str, list[list[object]]] = {}
    for name, query in ENGINE_QUERIES.items():
        extract[name] = [list(row) for row in connection.execute(query).fetchall()]
    return extract


def railroad_scripts(railroad: Path) -> list[Path]:
    """The page diagram scripts beside the shared railroad script, in path order."""
    found: list[Path] = []
    for folder in RAILROAD_DIRS:
        found.extend((railroad.parent / folder).glob("*.js"))
    return sorted(found)


def fetch(cache: Path) -> Path:
    """Fill the cache: both checkouts, the evaluated diagrams and the engine extract."""
    cache.mkdir(parents=True, exist_ok=True)
    web: Path = cache / WEB_DIR
    checkout(target=web, repository=WEB_REPOSITORY, commit=WEB_COMMIT, paths=WEB_PATHS)
    checkout(
        target=cache / ENGINE_DIR,
        repository=ENGINE_REPOSITORY,
        commit=ENGINE_COMMIT,
        paths=ENGINE_PATHS,
    )
    railroad: Path = web / RAILROAD_ROOT / RAILROAD_SHARED
    scripts: list[Path] = railroad_scripts(railroad)
    (cache / RAILROAD_FILE).write_text(
        json.dumps(evaluated(railroad=railroad, scripts=scripts), indent=1)
    )
    (cache / ENGINE_EXTRACT).write_text(json.dumps(engine_extract(), indent=1))
    (cache / FETCH_RECORD).write_text(
        json.dumps(
            {
                "retrieved_utc": time.strftime(UTC_FORMAT, time.gmtime()),
                "web_commit": WEB_COMMIT,
                "engine_commit": ENGINE_COMMIT,
                "duckdb": DUCKDB_VERSION,
            }
        )
    )
    return cache


def pinned(cache: Path) -> tuple[Path, str]:
    """The cache after checking both checkouts are at their pins, and its retrieval time."""
    record: dict[str, str] = json.loads((cache / FETCH_RECORD).read_text())
    for folder, commit in ((WEB_DIR, WEB_COMMIT), (ENGINE_DIR, ENGINE_COMMIT)):
        if git("rev-parse", "HEAD", cwd=cache / folder) != commit:
            raise DuckDbCatalogueError(f"{cache / folder}: not at {commit}; run the fetch first")
    return cache, record["retrieved_utc"]
