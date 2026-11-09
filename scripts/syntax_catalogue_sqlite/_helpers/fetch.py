"""The private source cache: the three pinned release archives and the engine built from them."""

from __future__ import annotations

import ctypes
import hashlib
import json
import subprocess
import time
import urllib.request
import zipfile
from pathlib import Path

from scripts.sqlite_receipts.classes.library import Library
from scripts.syntax_catalogue_sqlite.constants import (
    AMALGAMATION,
    AMALGAMATION_FILES,
    AMALGAMATION_PREFIX,
    ARCHIVES,
    COMPILE_FLAGS,
    COMPILER,
    DOWNLOAD_DIR,
    DOWNLOAD_ROOT,
    ENGINE_DIR,
    ENGINE_EXTRACT,
    ENGINE_QUERIES,
    FETCH_RECORD,
    FETCH_TIMEOUT_SECONDS,
    LIBRARY_FILE,
    LINK_FLAGS,
    SOURCE_ID,
    SQLITE_VERSION,
    USER_AGENT,
    UTC_FORMAT,
)
from scripts.syntax_catalogue_sqlite.exceptions import SqliteCatalogueError


def file_sha256(path: Path) -> str:
    """The SHA-256 of a file's bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def archive_path(*, cache: Path, name: str) -> Path:
    """Where the named archive is kept: alone in its own directory of the cache."""
    return cache / DOWNLOAD_DIR.format(name=name) / ARCHIVES[name][0]


def verified(*, path: Path, name: str) -> Path:
    """The archive, after checking its SHA-256 and the SHA3-256 sqlite.org publishes."""
    data: bytes = path.read_bytes()
    _, sha256, sha3 = ARCHIVES[name]
    if hashlib.sha256(data).hexdigest() != sha256 or hashlib.sha3_256(data).hexdigest() != sha3:
        raise SqliteCatalogueError(f"{path}: not the pinned {ARCHIVES[name][0]}")
    return path


def download(*, cache: Path, name: str) -> Path:
    """Download one archive into an empty directory of its own, unless it is already there."""
    path: Path = archive_path(cache=cache, name=name)
    if path.exists():
        return verified(path=path, name=name)
    path.parent.mkdir(parents=True, exist_ok=True)
    if any(path.parent.iterdir()):
        raise SqliteCatalogueError(f"{path.parent}: not empty; download into an empty directory")
    request: urllib.request.Request = urllib.request.Request(
        DOWNLOAD_ROOT + ARCHIVES[name][0], headers={"User-Agent": USER_AGENT}
    )
    with urllib.request.urlopen(request, timeout=FETCH_TIMEOUT_SECONDS) as response:
        path.write_bytes(response.read())
    return verified(path=path, name=name)


def member(*, archive: Path, name: str) -> bytes:
    """One member of a pinned archive, by its path inside the archive."""
    with zipfile.ZipFile(archive) as opened:
        return opened.read(name)


def build_engine(cache: Path) -> Path:
    """Compile the amalgamation into a shared library in the cache's engine directory."""
    engine: Path = cache / ENGINE_DIR
    engine.mkdir(parents=True, exist_ok=True)
    archive: Path = archive_path(cache=cache, name=AMALGAMATION)
    for file in AMALGAMATION_FILES:
        (engine / file).write_bytes(member(archive=archive, name=AMALGAMATION_PREFIX + file))
    library: Path = engine / LIBRARY_FILE
    command: list[str] = [
        COMPILER,
        *COMPILE_FLAGS,
        "-o",
        str(library),
        str(engine / AMALGAMATION_FILES[0]),
        *LINK_FLAGS,
    ]
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        command, capture_output=True, text=True, check=False
    )
    if completed.returncode != 0:
        raise SqliteCatalogueError(f"compiling the amalgamation failed: {completed.stderr}")
    return library


def compiler_version() -> str:
    """The first line of the compiler's --version output."""
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        [COMPILER, "--version"], capture_output=True, text=True, check=True
    )
    return completed.stdout.splitlines()[0]


def engine_extract(library: Path) -> dict[str, object]:
    """The built engine's version, keywords, functions, pragmas, modules and options."""
    engine: Library = Library(library)
    version, source = engine.version()
    if (version, source) != (SQLITE_VERSION, SOURCE_ID):
        raise SqliteCatalogueError(f"{library}: SQLite {version} {source}, not the pin")
    handle: ctypes.c_void_p = engine.open()
    try:
        extract: dict[str, object] = {
            "version": version,
            "source_id": source,
            "keywords": engine.keywords(),
        }
        for key, query in ENGINE_QUERIES.items():
            extract[key] = engine.rows(handle=handle, sql=query)
    finally:
        engine.close(handle)
    return extract


def fetch(cache: Path) -> Path:
    """Fill the cache: the three verified archives, the built library and its extract."""
    cache.mkdir(parents=True, exist_ok=True)
    for name in ARCHIVES:
        download(cache=cache, name=name)
    library: Path = build_engine(cache)
    (cache / ENGINE_EXTRACT).write_text(json.dumps(engine_extract(library), indent=1) + "\n")
    record: dict[str, object] = {
        "retrieved_utc": time.strftime(UTC_FORMAT, time.gmtime()),
        "archives": {name: ARCHIVES[name][1] for name in ARCHIVES},
        "library_sha256": file_sha256(library),
        "compiler": compiler_version(),
        "compile_flags": list(COMPILE_FLAGS),
    }
    (cache / FETCH_RECORD).write_text(json.dumps(record, indent=1) + "\n")
    return cache


def pinned(cache: Path) -> tuple[Path, str]:
    """The cache after checking every archive and the built library, and its retrieval time."""
    record_path: Path = cache / FETCH_RECORD
    if not record_path.exists():
        raise SqliteCatalogueError(f"{record_path}: missing; run the fetch first")
    record: dict[str, object] = json.loads(record_path.read_text())
    for name in ARCHIVES:
        verified(path=archive_path(cache=cache, name=name), name=name)
    if file_sha256(cache / ENGINE_DIR / LIBRARY_FILE) != record["library_sha256"]:
        raise SqliteCatalogueError(f"{cache / ENGINE_DIR / LIBRARY_FILE}: changed since the fetch")
    return cache, str(record["retrieved_utc"])
