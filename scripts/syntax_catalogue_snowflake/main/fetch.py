"""Fetch the pinned Snowflake documentation pages into a private, untracked cache."""

from __future__ import annotations

import concurrent.futures
import json
import re
import sys
from pathlib import Path

from scripts.grammar_catalogue.constants import (
    DOCS_PREFIX,
    FETCH_WORKERS,
)
from scripts.syntax_catalogue_snowflake._helpers.fetch import fetch_one, fetched_urls, index_urls
from scripts.syntax_catalogue_snowflake.constants import (
    FETCH_MIN_ARGUMENTS,
    FETCH_USAGE,
    LOG_EVERY,
)


def run_fetch(argv: list[str] | None = None) -> int:
    """Fetch every indexed page not yet in the cache, appending to fetch-log.jsonl."""
    args: list[str] = sys.argv[1:] if argv is None else argv
    if len(args) < FETCH_MIN_ARGUMENTS:
        print(FETCH_USAGE, file=sys.stderr)
        return 2
    root: Path = Path(args[0])
    urls: list[str] = index_urls([Path(arg) for arg in args[1:]])
    log_path: Path = root / "fetch-log.jsonl"
    done: set[str] = fetched_urls(log_path)
    todo: list[str] = [url for url in urls if url not in done and re.match(DOCS_PREFIX, url)]
    print(len(urls), "urls", len(todo), "to fetch", flush=True)
    with log_path.open("a") as log, concurrent.futures.ThreadPoolExecutor(FETCH_WORKERS) as pool:
        for count, record in enumerate(pool.map(lambda u: fetch_one(url=u, root=root), todo), 1):
            log.write(json.dumps(record) + "\n")
            if count % LOG_EVERY == 0:
                log.flush()
                print(count, flush=True)
    print("done", flush=True)
    return 0
