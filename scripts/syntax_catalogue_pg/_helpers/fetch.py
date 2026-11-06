"""The private source cache: the pinned release tarball, verified and unpacked."""

from __future__ import annotations

import hashlib
import json
import tarfile
import time
import urllib.request
from pathlib import Path

from scripts.syntax_catalogue.exceptions import CatalogueError
from scripts.syntax_catalogue_pg.constants import (
    TARBALL_NAME,
    TARBALL_SHA256,
    TARBALL_URL,
    TREE_ROOT,
    USER_AGENT,
    UTC_FORMAT,
)

FETCH_RECORD: str = "fetch.json"


def file_sha256(path: Path) -> str:
    """The hex SHA-256 of a file's bytes."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fetch(cache: Path) -> Path:
    """Download the pinned tarball into the cache, verify it and unpack it once."""
    cache.mkdir(parents=True, exist_ok=True)
    tarball: Path = cache / TARBALL_NAME
    if not tarball.exists():
        request = urllib.request.Request(TARBALL_URL, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(request, timeout=120) as response:
            tarball.write_bytes(response.read())
        (cache / FETCH_RECORD).write_text(
            json.dumps(
                {"url": TARBALL_URL, "retrieved_utc": time.strftime(UTC_FORMAT, time.gmtime())}
            )
        )
    if file_sha256(tarball) != TARBALL_SHA256:
        raise CatalogueError(f"{tarball}: sha256 is not the pinned {TARBALL_SHA256}")
    tree: Path = cache / TREE_ROOT
    if not tree.exists():
        with tarfile.open(tarball) as archive:
            archive.extractall(cache, filter="data")
    return tree


def pinned(cache: Path) -> tuple[Path, str]:
    """The unpacked tree of the verified tarball and when the tarball was retrieved."""
    tarball: Path = cache / TARBALL_NAME
    if not tarball.exists() or file_sha256(tarball) != TARBALL_SHA256:
        raise CatalogueError(f"{tarball}: missing or not the pinned release; run the fetch first")
    record: dict[str, str] = json.loads((cache / FETCH_RECORD).read_text())
    return cache / TREE_ROOT, record["retrieved_utc"]
