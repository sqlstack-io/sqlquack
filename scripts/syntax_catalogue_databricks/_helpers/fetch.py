"""The private source cache of the pinned reference pages and Spark grammar, and its index rows."""

from __future__ import annotations

import hashlib
import json
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import cast

from scripts.syntax_catalogue_databricks.constants import (
    DBSQL_VERSION,
    FETCH_ATTEMPTS,
    FETCH_PAUSE_SECONDS,
    FETCH_RECORD,
    FETCH_TIMEOUT_SECONDS,
    FETCH_WORKERS,
    FUNCTION_INDEX_PAGES,
    INDEX_LINK,
    INDEX_LINKED_KEY,
    INDEX_PAGE,
    MANUAL_PREFIX,
    PAGE_SUFFIX,
    PAGES_DIR,
    SITE,
    SITEMAP_FILE,
    SITEMAP_LOC,
    SITEMAP_URL,
    SPARK_COMMIT,
    SPARK_DIR,
    SPARK_FILES,
    SPARK_RAW,
    SPARK_TAG,
    USER_AGENT,
    UTC_FORMAT,
)
from scripts.syntax_catalogue_databricks.exceptions import DatabricksCatalogueError
from scripts.syntax_catalogue_databricks.types import Row


def sha256(data: bytes) -> str:
    """The SHA-256 of bytes, in hex."""
    return hashlib.sha256(data).hexdigest()


def download(url: str) -> bytes:
    """One URL's body, retried with a growing pause; a page that never answers stops the fetch."""
    request: urllib.request.Request = urllib.request.Request(
        url, headers={"User-Agent": USER_AGENT}
    )
    for attempt in range(FETCH_ATTEMPTS):
        try:
            with urllib.request.urlopen(request, timeout=FETCH_TIMEOUT_SECONDS) as response:
                return response.read()
        except (urllib.error.URLError, TimeoutError) as error:
            if attempt + 1 == FETCH_ATTEMPTS:
                raise DatabricksCatalogueError(f"{url}: {error}") from error
            time.sleep(FETCH_PAUSE_SECONDS * 2 ** (attempt + 3))
    raise DatabricksCatalogueError(f"{url}: no answer")


def page_name(url: str) -> str:
    """The cache file name of a reference page: its path under the manual, `/` as `__`."""
    rest: str = url.removeprefix(MANUAL_PREFIX).strip("/")
    return (rest.replace("/", "__") or INDEX_PAGE) + PAGE_SUFFIX


def manual_urls(sitemap: bytes) -> list[str]:
    """Every page of the SQL language reference the sitemap lists, sorted."""
    found: set[str] = {
        url for url in SITEMAP_LOC.findall(sitemap.decode()) if url.startswith(MANUAL_PREFIX)
    }
    return sorted(found)


def index_linked(*, cache: Path, pages: dict[str, str]) -> list[str]:
    """Function pages the cached alphabetical indexes link that the sitemap did not list."""
    linked: set[str] = set()
    for index in FUNCTION_INDEX_PAGES:
        text: str = (cache / PAGES_DIR / f"{index}{PAGE_SUFFIX}").read_text()
        linked |= {SITE + path.rstrip("/") for path in INDEX_LINK.findall(text)}
    return sorted(url for url in linked if url not in pages)


def fetch(cache: Path) -> Path:
    """Fill an empty cache with the reference pages and the pinned Spark files."""
    if cache.exists() and any(cache.iterdir()):
        raise DatabricksCatalogueError(f"{cache}: not empty; fetch into an empty directory")
    (cache / PAGES_DIR).mkdir(parents=True)
    (cache / SPARK_DIR).mkdir()
    retrieved: str = time.strftime(UTC_FORMAT, time.gmtime())
    sitemap: bytes = download(SITEMAP_URL)
    (cache / SITEMAP_FILE).write_bytes(sitemap)
    urls: list[str] = manual_urls(sitemap)

    def one(url: str) -> tuple[str, str]:
        body: bytes = download(url)
        (cache / PAGES_DIR / page_name(url)).write_bytes(body)
        time.sleep(FETCH_PAUSE_SECONDS)
        return url, sha256(body)

    with ThreadPoolExecutor(max_workers=FETCH_WORKERS) as pool:
        pages: dict[str, str] = dict(pool.map(one, urls))
    linked: list[str] = index_linked(cache=cache, pages=pages)
    pages |= dict(one(url) for url in linked)
    spark: dict[str, str] = {}
    for path in SPARK_FILES:
        body: bytes = download(SPARK_RAW + path)
        (cache / SPARK_DIR / Path(path).name).write_bytes(body)
        spark[path] = sha256(body)
    record: Row = {
        "retrieved_utc": retrieved,
        "sitemap_url": SITEMAP_URL,
        "sitemap_sha256": sha256(sitemap),
        "pages": pages,
        INDEX_LINKED_KEY: linked,
        "spark_tag": SPARK_TAG,
        "spark_commit": SPARK_COMMIT,
        "spark_files": spark,
    }
    (cache / FETCH_RECORD).write_text(json.dumps(record, indent=1, sort_keys=True) + "\n")
    return cache


def pinned_pages(record: Row) -> dict[str, str]:
    """The fetch record's pages, each URL with the SHA-256 of its HTML."""
    return cast(dict[str, str], record["pages"])


def pinned_files(record: Row) -> dict[str, str]:
    """The fetch record's Spark files, each path with its SHA-256."""
    return cast(dict[str, str], record["spark_files"])


def index_linked_urls(record: Row) -> list[str]:
    """The pages the fetch added from the function indexes, which the sitemap left out."""
    return cast(list[str], record.get(INDEX_LINKED_KEY, []))


def pinned(cache: Path) -> tuple[Row, str]:
    """The fetch record after checking every cached file against it, and its retrieval time."""
    path: Path = cache / FETCH_RECORD
    if not path.exists():
        raise DatabricksCatalogueError(f"{path}: missing; run the fetch first")
    record: Row = json.loads(path.read_text())
    if sha256((cache / SITEMAP_FILE).read_bytes()) != record["sitemap_sha256"]:
        raise DatabricksCatalogueError(f"{cache / SITEMAP_FILE}: changed since the fetch")
    pages: dict[str, str] = pinned_pages(record)
    for url, digest in pages.items():
        if sha256((cache / PAGES_DIR / page_name(url)).read_bytes()) != digest:
            raise DatabricksCatalogueError(f"{url}: cached page changed since the fetch")
    files: dict[str, str] = pinned_files(record)
    for name, digest in files.items():
        if sha256((cache / SPARK_DIR / Path(name).name).read_bytes()) != digest:
            raise DatabricksCatalogueError(f"{name}: cached Spark file changed since the fetch")
    return record, str(record["retrieved_utc"])


def index_rows(*, record: Row, articles: dict[str, str]) -> list[Row]:
    """Name and SHA-256 of the sitemap, of the page set and of each pinned Spark file."""
    pages: dict[str, str] = pinned_pages(record)
    page_set: str = "".join(f"{articles[url]}  {url}\n" for url in sorted(pages))
    rows: list[Row] = [
        {
            "name": f"{record['sitemap_url']} (retrieved {record['retrieved_utc']})",
            "sha256": str(record["sitemap_sha256"]),
        },
        {
            "name": f"language-manual page set: {len(pages)} pages as `<article sha256>  <url>` "
            f"lines (Databricks SQL {DBSQL_VERSION})",
            "sha256": hashlib.sha256(page_set.encode()).hexdigest(),
        },
    ]
    files: dict[str, str] = pinned_files(record)
    rows += [
        {"name": f"apache/spark {SPARK_TAG} ({SPARK_COMMIT}) {path}", "sha256": digest}
        for path, digest in sorted(files.items())
    ]
    return rows
