"""The indexes.toml rows: the sitemap, the page set and the pinned Spark files."""

from __future__ import annotations

import hashlib

from scripts.syntax_catalogue_databricks.constants import (
    DBSQL_VERSION,
    SPARK_COMMIT,
    SPARK_TAG,
)


def index_rows(*, record: dict[str, object], articles: dict[str, str]) -> list[dict[str, str]]:
    """Name and SHA-256 of the sitemap, of the page set and of each pinned Spark file.

    The page set lists each page's article hash, which a redeploy of the docs site leaves alone.
    """
    pages: dict[str, str] = record["pages"]  # type: ignore[assignment]
    page_set: str = "".join(f"{articles[url]}  {url}\n" for url in sorted(pages))
    rows: list[dict[str, str]] = [
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
    files: dict[str, str] = record["spark_files"]  # type: ignore[assignment]
    rows += [
        {"name": f"apache/spark {SPARK_TAG} ({SPARK_COMMIT}) {path}", "sha256": digest}
        for path, digest in sorted(files.items())
    ]
    return rows
