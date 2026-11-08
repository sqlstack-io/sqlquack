"""A fetched page differs from its pin only when its article does, not when its script bundles do."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.syntax_catalogue_databricks._helpers.pages import pin_differences, read_page
from scripts.syntax_catalogue_databricks.models import DbPage
from tests.unit.scripts.syntax_catalogue_databricks._test_types import PinTestCase
from tests.unit.scripts.syntax_catalogue_databricks.helpers import ARTICLE, URL, cached, page


@pytest.mark.parametrize(
    "test_case",
    (
        PinTestCase(
            description="a redeployed script bundle",
            fetched=page(bundle="c9f95641", article=ARTICLE),
            expected_differences=[],
        ),
        PinTestCase(
            description="an edited article",
            fetched=page(bundle="31ad9c0d", article=ARTICLE.replace("Text.", "Edited text.")),
            expected_differences=[f"article_sha256 changed: {URL}"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_fetched_page_when_compared_with_its_pin_then_only_its_article_decides(
    test_case: PinTestCase, tmp_path: Path
) -> None:
    pin: DbPage = read_page(
        cache=cached(tmp_path / "pin", page(bundle="31ad9c0d", article=ARTICLE)), url=URL
    )
    root: Path = tmp_path / "root"
    root.mkdir()
    (root / "sources.toml").write_text(
        f'[[source]]\nurl = "{URL}"\nsyntax_sha256 = "{pin.syntax_sha256}"\n'
    )
    (root / "articles.toml").write_text(
        f'[[article]]\nurl = "{URL}"\narticle_sha256 = "{pin.article_sha256}"\n'
    )

    found: list[str] = pin_differences(
        cache=cached(tmp_path / "fresh", test_case.fetched), root=root, urls=[URL]
    )

    assert found == test_case.expected_differences
