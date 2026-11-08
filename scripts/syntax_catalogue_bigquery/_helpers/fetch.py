"""The private source cache: the reference pages as Markdown and a sparse GoogleSQL checkout."""

from __future__ import annotations

import concurrent.futures
import datetime
import hashlib
import json
import subprocess
import time
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

from scripts.syntax_catalogue_bigquery.constants import (
    DOCS_DIR,
    DOCS_ORIGIN,
    FETCH_ATTEMPTS,
    FETCH_LOG,
    FETCH_RECORD,
    FETCH_TIMEOUT_SECONDS,
    FETCH_WORKERS,
    GOOGLESQL_COMMIT,
    GOOGLESQL_REPOSITORY,
    GOOGLESQL_TAG,
    GRAMMAR_DIR,
    GRAMMAR_FILES,
    HTTP_NOT_FOUND,
    HTTP_OK,
    INDEX_FILE,
    INDEXES_FILE,
    MARKDOWN_SUFFIX,
    NAV_LINK,
    NAV_PAGE,
    PAGE_LINK,
    PAGES_DIR,
    REFERENCE_PREFIX,
    SOURCES_FILE,
    STANDARD_SQL,
    USER_AGENT,
    UTC_FORMAT,
)
from scripts.syntax_catalogue_bigquery.exceptions import BigQueryCatalogueError
from scripts.syntax_catalogue_bigquery.models import FetchedPage
from scripts.syntax_catalogue_bigquery.types import Row


def now_utc() -> str:
    """The current time in UTC, to the second."""
    return datetime.datetime.now(datetime.UTC).strftime(UTC_FORMAT)


def download(url: str) -> tuple[bytes, int | str]:
    """The body and HTTP status of a URL, retrying transient failures; 404 is final."""
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


def page_path(link: str) -> str:
    """`standard-sql/query-syntax` from `/bigquery/docs/reference/standard-sql/query-syntax`."""
    return link.removeprefix(REFERENCE_PREFIX)


def markdown_url(path: str) -> str:
    """The Markdown export of a reference page."""
    return f"{DOCS_ORIGIN}{REFERENCE_PREFIX}{path}{MARKDOWN_SUFFIX}"


def nav_paths(html: str) -> list[str]:
    """The SQL reference pages the reference navigation links, sorted."""
    return sorted({page_path(link) for link in NAV_LINK.findall(html)})


def fetch_page(*, path: str, pages: Path) -> tuple[FetchedPage, list[str]]:
    """Fetch one page's Markdown into the cache; return its record and the pages it links."""
    url: str = markdown_url(path)
    body, status = download(url)
    retrieved: str = now_utc()
    if status != HTTP_OK:
        return FetchedPage(path, url, retrieved, status, 0, ""), []
    target: Path = pages / f"{path}.md"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(body)
    links: list[str] = [page_path(link) for link in PAGE_LINK.findall(body.decode())]
    return (
        FetchedPage(path, url, retrieved, status, len(body), hashlib.sha256(body).hexdigest()),
        links,
    )


def crawl(*, seeds: list[str], pages: Path) -> list[FetchedPage]:
    """Every page reachable from the navigation through links between reference pages."""
    seen: set[str] = set(seeds)
    frontier: list[str] = sorted(seeds)
    records: list[FetchedPage] = []
    with concurrent.futures.ThreadPoolExecutor(FETCH_WORKERS) as pool:
        while frontier:
            found: list[str] = []
            for record, links in pool.map(lambda p: fetch_page(path=p, pages=pages), frontier):
                records.append(record)
                found += [link for link in links if link not in seen]
                seen.update(links)
            frontier = sorted(set(found))
    return sorted(records, key=lambda record: record.path)


def git(*arguments: str, cwd: Path) -> str:
    """Run git and return its output, raising with its message on failure."""
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        ["git", *arguments], cwd=cwd, capture_output=True, text=True, check=False
    )
    if completed.returncode != 0:
        raise BigQueryCatalogueError(f"git {' '.join(arguments)}: {completed.stderr.strip()}")
    return completed.stdout.strip()


def checkout_grammar(target: Path) -> None:
    """A sparse, depth-1 checkout of the pinned GoogleSQL files, verified at the pinned commit."""
    target.mkdir(parents=True)
    git("init", "-q", cwd=target)
    git("remote", "add", "origin", GOOGLESQL_REPOSITORY, cwd=target)
    git("sparse-checkout", "set", "--no-cone", *[f"/{name}" for name in GRAMMAR_FILES], cwd=target)
    git("fetch", "-q", "--depth", "1", "--filter=blob:none", "origin", GOOGLESQL_COMMIT, cwd=target)
    git("checkout", "-q", "--detach", GOOGLESQL_COMMIT, cwd=target)
    tagged: str = git("ls-remote", "origin", f"refs/tags/{GOOGLESQL_TAG}", cwd=target)
    if git("rev-parse", "HEAD", cwd=target) != GOOGLESQL_COMMIT or not tagged.startswith(
        GOOGLESQL_COMMIT
    ):
        raise BigQueryCatalogueError(f"{target}: not at {GOOGLESQL_TAG} ({GOOGLESQL_COMMIT})")


def fetch(cache: Path) -> dict[str, object]:
    """Fill an empty cache: the navigation, every reachable page and the grammar checkout."""
    docs: Path = cache / DOCS_DIR
    for folder in (docs, cache / GRAMMAR_DIR):
        if folder.exists() and any(folder.iterdir()):
            raise BigQueryCatalogueError(f"{folder} is not empty; fetch into a new cache")
    started: str = now_utc()
    nav_body, nav_status = download(f"{DOCS_ORIGIN}{REFERENCE_PREFIX}{NAV_PAGE}")
    if nav_status != HTTP_OK:
        raise BigQueryCatalogueError(f"navigation page: HTTP {nav_status}")
    seeds: list[str] = nav_paths(nav_body.decode())
    records: list[FetchedPage] = crawl(seeds=seeds, pages=docs / PAGES_DIR)
    with (docs / FETCH_LOG).open("w") as log:
        for record in records:
            log.write(json.dumps(record.__dict__, sort_keys=True) + "\n")
    index: dict[str, object] = {
        "navigation": NAV_PAGE,
        "navigation_pages": seeds,
        "pages": [r.path for r in records if r.status == HTTP_OK],
        "not_found": [r.path for r in records if r.status != HTTP_OK],
    }
    (docs / INDEX_FILE).write_text(json.dumps(index, indent=1) + "\n")
    checkout_grammar(cache / GRAMMAR_DIR)
    summary: dict[str, object] = {
        "started_utc": started,
        "finished_utc": now_utc(),
        "pages": len(index["pages"]),  # type: ignore[arg-type]
        "not_found": len(index["not_found"]),  # type: ignore[arg-type]
        "googlesql_tag": GOOGLESQL_TAG,
        "googlesql_commit": GOOGLESQL_COMMIT,
    }
    (cache / FETCH_RECORD).write_text(json.dumps(summary, indent=1) + "\n")
    return summary


def fetched_pages(cache: Path) -> list[FetchedPage]:
    """The fetch log's records, by path."""
    lines: list[str] = (cache / DOCS_DIR / FETCH_LOG).read_text().splitlines()
    return [FetchedPage(**json.loads(line)) for line in lines]


def file_sha256(path: Path) -> str:
    """The hex sha256 of a file's bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def pinned(cache: Path) -> Path:
    """The cache after checking the grammar checkout is at its pin and the pages are intact."""
    grammar: Path = cache / GRAMMAR_DIR
    if git("rev-parse", "HEAD", cwd=grammar) != GOOGLESQL_COMMIT:
        raise BigQueryCatalogueError(f"{grammar}: not at {GOOGLESQL_COMMIT}; fetch again")
    if git("status", "--porcelain", cwd=grammar):
        raise BigQueryCatalogueError(f"{grammar}: has local changes")
    for record in fetched_pages(cache):
        if record.status != HTTP_OK:
            continue
        cached: Path = cache / DOCS_DIR / PAGES_DIR / f"{record.path}.md"
        if file_sha256(cached) != record.sha256:
            raise BigQueryCatalogueError(f"{cached}: changed since it was fetched")
    return cache


def page_id(path: str) -> str:
    """`doc.<page>` for a SQL reference page, `doc.reference.<page>` for the two outside it."""
    if path.startswith(STANDARD_SQL):
        return "doc." + path.removeprefix(STANDARD_SQL).replace("_", "-")
    return "doc.reference." + path.replace("_", "-")


def pin_rows(cache: Path) -> list[dict[str, object]]:
    """One pin per fetched page: id, URL, retrieval time and sha256 of the Markdown."""
    return [
        {
            "id": page_id(record.path),
            "url": record.url.removesuffix(MARKDOWN_SUFFIX),
            "retrieved_utc": record.retrieved_utc,
            "page_sha256": record.sha256,
        }
        for record in fetched_pages(cache)
        if record.status == HTTP_OK
    ]


def kept_retrieval(*, path: Path, rows: list[Row]) -> list[Row]:
    """The rows with the written retrieval time of each page whose sha256 is unchanged."""
    if not path.exists():
        return rows
    written: dict[tuple[str, str], str] = {
        (str(row["id"]), str(row["page_sha256"])): str(row["retrieved_utc"])
        for row in tomllib.loads(path.read_text()).get("source", [])
    }
    return [
        {
            **row,
            "retrieved_utc": written.get(
                (str(row["id"]), str(row["page_sha256"])), row["retrieved_utc"]
            ),
        }
        for row in rows
    ]


def grammar_rows(cache: Path) -> list[dict[str, object]]:
    """One [[index]] per GoogleSQL file read, named by repository, tag, commit and path."""
    return [
        {
            "name": f"{GOOGLESQL_REPOSITORY}@{GOOGLESQL_TAG} ({GOOGLESQL_COMMIT}) {name}",
            "sha256": file_sha256(cache / GRAMMAR_DIR / name),
        }
        for name in GRAMMAR_FILES
    ]


def verify(*, cache: Path, root: Path) -> list[str]:
    """Differences between a cache and the pins in sources.toml: pages, hashes and grammar files."""
    path: Path = root / SOURCES_FILE
    if not path.exists():
        return [f"{path} does not exist; nothing to verify against"]
    pins: dict[str, list[dict[str, object]]] = tomllib.loads(path.read_text())
    pinned_pages: dict[str, str] = {
        str(row["url"]): str(row["page_sha256"]) for row in pins.get("source", [])
    }
    fetched: dict[str, str] = {
        r.url.removesuffix(MARKDOWN_SUFFIX): r.sha256
        for r in fetched_pages(cache)
        if r.status == HTTP_OK
    }
    problems: list[str] = [
        f"page {url}: sha256 {fetched[url]}, pinned {sha}"
        for url, sha in sorted(pinned_pages.items())
        if url in fetched and fetched[url] != sha
    ]
    problems += [f"pinned page not fetched: {url}" for url in sorted(pinned_pages.keys() - fetched)]
    problems += [f"page not pinned: {url}" for url in sorted(fetched.keys() - pinned_pages)]
    indexes: Path = root / INDEXES_FILE
    pinned_files: list[dict[str, object]] = (
        tomllib.loads(indexes.read_text()).get("index", []) if indexes.exists() else []
    )
    if [row["name"] for row in pinned_files] != [row["name"] for row in grammar_rows(cache)]:
        problems.append(f"{indexes}: does not pin exactly the grammar files {GRAMMAR_FILES}")
    for row in pinned_files:
        path: str = str(row["name"]).rsplit(" ", 1)[1]
        actual: str = file_sha256(cache / GRAMMAR_DIR / path)
        if actual != row["sha256"]:
            problems.append(f"grammar file {path}: sha256 {actual}, pinned {row['sha256']}")
    return problems
