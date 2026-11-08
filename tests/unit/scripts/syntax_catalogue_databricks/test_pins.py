"""A fetched page differs from its pin only when its article does, not when its script bundles do."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.syntax_catalogue_databricks._helpers.pages import pin_differences, read_page
from scripts.syntax_catalogue_databricks.constants import MANUAL_PREFIX

URL: str = f"{MANUAL_PREFIX}sql-ref-example"
ARTICLE: str = "<article><h1>EXAMPLE</h1><p>Text.</p></article>"


def page(*, bundle: str, article: str) -> str:
    """A reference page with one script bundle and one article."""
    return f"<html><script src=/assets/js/main.{bundle}.js></script>{article}</html>"


def cached(root: Path, html: str) -> Path:
    """A cache holding the one page."""
    (root / "pages").mkdir(parents=True)
    (root / "pages" / "sql-ref-example.html").write_text(html)
    return root


@pytest.mark.parametrize(
    ("fetched", "expected"),
    (
        (page(bundle="c9f95641", article=ARTICLE), []),
        (
            page(bundle="31ad9c0d", article=ARTICLE.replace("Text.", "Edited text.")),
            [f"article_sha256 changed: {URL}"],
        ),
    ),
    ids=("a redeployed script bundle", "an edited article"),
)
def test_given_a_fetched_page_when_compared_with_its_pin_then_only_its_article_decides(
    tmp_path: Path, fetched: str, expected: list[str]
) -> None:
    pin = read_page(
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
        cache=cached(tmp_path / "fresh", fetched), root=root, urls=[URL]
    )

    assert found == expected
