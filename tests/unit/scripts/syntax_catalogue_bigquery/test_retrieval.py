"""A rebuild keeps a page's written retrieval time while the page's sha256 is unchanged."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.syntax_catalogue_bigquery._helpers.fetch import kept_retrieval
from tests.unit.scripts.syntax_catalogue_bigquery._test_types import RetrievalTestCase

WRITTEN: str = (
    '[[source]]\nid = "doc.a"\npage_sha256 = "aa"\nretrieved_utc = "2026-11-06T17:39:55Z"\n'
)
REFETCHED: str = "2026-12-01T00:00:00Z"


@pytest.mark.parametrize(
    "test_case",
    (
        RetrievalTestCase(
            description="the same page refetched keeps its written time",
            page_id="doc.a",
            page_sha256="aa",
            expected_retrieved_utc="2026-11-06T17:39:55Z",
        ),
        RetrievalTestCase(
            description="a changed page takes the new time",
            page_id="doc.a",
            page_sha256="a2",
            expected_retrieved_utc=REFETCHED,
        ),
        RetrievalTestCase(
            description="a new page takes the new time",
            page_id="doc.b",
            page_sha256="aa",
            expected_retrieved_utc=REFETCHED,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_written_pin_when_rebuilt_then_time_kept_only_for_unchanged_page(
    tmp_path: Path, test_case: RetrievalTestCase
) -> None:
    written: Path = tmp_path / "sources.toml"
    written.write_text(WRITTEN)
    row: dict[str, object] = {
        "id": test_case.page_id,
        "page_sha256": test_case.page_sha256,
        "retrieved_utc": REFETCHED,
    }

    kept: list[dict[str, object]] = kept_retrieval(path=written, rows=[row])

    assert kept[0]["retrieved_utc"] == test_case.expected_retrieved_utc
