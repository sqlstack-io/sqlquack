"""Find and download the pinned Snowflake documentation pages for the private cache."""

from __future__ import annotations

import datetime
import gzip
import hashlib
import json
import time
import urllib.error
import urllib.request
from pathlib import Path

from scripts.syntax_catalogue.constants import (
    DOCS_ORIGIN,
    DOCS_PREFIX,
    FETCH_ATTEMPTS,
    FETCH_TIMEOUT_SECONDS,
    FORMAT_HTML,
    FORMAT_MARKDOWN,
    HTTP_NOT_FOUND,
    HTTP_OK,
    INDEX_MARKDOWN_URL,
    MARKDOWN_SUFFIX,
    NAV_INDEX_SUFFIX,
    REFERENCE_ROOT,
    USER_AGENT,
    UTC_FORMAT,
)
from scripts.syntax_catalogue.types import Record


def nav_paths(node: object) -> list[str]:
    """Every `/en/sql-reference...` page path in the docs navigation tree."""
    if isinstance(node, dict):
        found: list[str] = []
        for value in node.values():
            if isinstance(value, str) and value.startswith(f"/en/{REFERENCE_ROOT}"):
                found.append(value.split("#")[0].rstrip("/"))
            else:
                found += nav_paths(value)
        return found
    if isinstance(node, list):
        paths: list[str] = []
        for item in node:
            paths += nav_paths(item)
        return paths
    return []


def index_urls(index_files: list[Path]) -> list[str]:
    """Markdown page URLs listed in llms.txt indexes and navigation trees, first-seen order."""
    urls: list[str] = []
    for index in index_files:
        text: str = index.read_text()
        if index.name.endswith(NAV_INDEX_SUFFIX):
            found: list[str] = [
                DOCS_ORIGIN + p + MARKDOWN_SUFFIX for p in nav_paths(json.loads(text))
            ]
        else:
            found = INDEX_MARKDOWN_URL.findall(text)
        urls += [url for url in found if url not in urls]
    return urls


def download(url: str) -> tuple[bytes, int | str]:
    """The page body and HTTP status, retrying transient failures."""
    status: int | str = ""
    for attempt in range(FETCH_ATTEMPTS):
        try:
            request: urllib.request.Request = urllib.request.Request(
                url, headers={"User-Agent": USER_AGENT}
            )
            with urllib.request.urlopen(request, timeout=FETCH_TIMEOUT_SECONDS) as response:
                return response.read(), response.status
        except urllib.error.HTTPError as error:
            status = error.code
            if error.code == HTTP_NOT_FOUND:
                break
        except OSError as error:
            status = str(error)[:80]
        time.sleep(2 * (attempt + 1))
    return b"", status


def fetch_one(*, url: str, root: Path) -> Record:
    """Fetch one page into the cache, falling back to HTML for pages with no Markdown export."""
    body, status = download(url)
    page_format: str = FORMAT_MARKDOWN
    if status == HTTP_NOT_FOUND:
        body, status = download(url.removesuffix(MARKDOWN_SUFFIX))
        page_format = FORMAT_HTML
    path: str = url.split(DOCS_PREFIX, 1)[1]
    if status == HTTP_OK:
        out: Path = root / "raw" / (path + ".gz")
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(gzip.compress(body))
    return {
        "url": url,
        "path": path,
        "status": status,
        "retrieved_utc": datetime.datetime.now(datetime.UTC).strftime(UTC_FORMAT),
        "sha256": hashlib.sha256(body).hexdigest() if status == HTTP_OK else None,
        "bytes": len(body),
        "format": page_format,
    }


def fetched_urls(log_path: Path) -> set[str]:
    """URLs already fetched successfully."""
    if not log_path.exists():
        return set()
    records: list[Record] = [json.loads(line) for line in log_path.read_text().splitlines()]
    return {str(r["url"]) for r in records if r["status"] == HTTP_OK}
