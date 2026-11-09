"""Read the pinned reference pages: title, applicability, headings and syntax blocks."""

from __future__ import annotations

import hashlib
import html
import re
import tomllib
from pathlib import Path

from scripts.syntax_catalogue_databricks._helpers.fetch import page_name
from scripts.syntax_catalogue_databricks.constants import (
    APPLIES_TO,
    ARTICLE,
    ARTICLES_FILE,
    BLOCK_HEADINGS,
    BREAK,
    HEADING_OR_BLOCK,
    MANUAL_PREFIX,
    PAGES_DIR,
    SOURCES_FILE,
    TAG,
    TITLE,
    TITLE_CODE,
)
from scripts.syntax_catalogue_databricks.exceptions import DatabricksCatalogueError
from scripts.syntax_catalogue_databricks.models import DbPage, SyntaxBlock


def plain(fragment: str) -> str:
    """HTML as text: line breaks kept, tags dropped, entities decoded."""
    return html.unescape(TAG.sub("", BREAK.sub("\n", fragment)))


def page_path(url: str) -> str:
    """A page's path under the manual: `functions/abs`, `sql-ref-syntax-qry-select`."""
    return url.removeprefix(MANUAL_PREFIX).strip("/")


def read_page(*, cache: Path, url: str) -> DbPage:
    """One cached page: its article's headings and the code blocks under syntax headings."""
    raw: bytes = (cache / PAGES_DIR / page_name(url)).read_bytes()
    text: str = raw.decode()
    article: re.Match[str] | None = ARTICLE.search(text)
    if article is None:
        raise DatabricksCatalogueError(f"{url}: no article")
    title_match: re.Match[str] | None = TITLE.search(article.group(0))
    title: str = plain(title_match.group(1)).strip() if title_match else ""
    codes: list[str] = TITLE_CODE.findall(title_match.group(1)) if title_match else []
    heading: str = ""
    headings: list[str] = []
    blocks: list[SyntaxBlock] = []
    others: list[SyntaxBlock] = []
    for match in HEADING_OR_BLOCK.finditer(article.group(0)):
        if match.group(1):
            heading = plain(match.group(3)).replace("\u200b", "").strip()
            headings.append(heading)
            anchor: str = match.group(2)
            continue
        body: str = plain(match.group(4)).rstrip()
        if headings and re.sub(r"-\d+$", "", anchor) in BLOCK_HEADINGS:
            blocks.append(SyntaxBlock(heading=heading, text=body))
        else:
            others.append(SyntaxBlock(heading=heading, text=body))
    applies: list[str] = [
        " ".join(plain(found).split()) for found in APPLIES_TO.findall(article.group(0))
    ]
    syntax: str = "\n\n".join(block.text for block in blocks)
    return DbPage(
        url=url,
        path=page_path(url),
        title=title,
        title_code=" ".join(plain(code) for code in codes),
        page_sha256=hashlib.sha256(raw).hexdigest(),
        article_sha256=hashlib.sha256(article.group(0).encode()).hexdigest(),
        syntax_sha256=hashlib.sha256(syntax.encode()).hexdigest(),
        blocks=tuple(blocks),
        applies=tuple(applies),
        headings=tuple(headings),
        others=tuple(others),
    )


def pin_differences(*, cache: Path, root: Path, urls: list[str]) -> list[str]:
    """Pages added or gone, and pages whose article or Syntax blocks changed, against the pins."""
    pinned: dict[str, dict[str, object]] = {
        str(row["url"]): dict(row)
        for row in tomllib.loads((root / SOURCES_FILE).read_text()).get("source", [])
    }
    for row in tomllib.loads((root / ARTICLES_FILE).read_text()).get("article", []):
        pinned.setdefault(str(row["url"]), {})["article_sha256"] = row["article_sha256"]
    found: list[str] = [f"page not pinned: {url}" for url in sorted(set(urls) - set(pinned))]
    found += [f"pinned page gone: {url}" for url in sorted(set(pinned) - set(urls))]
    for url in sorted(set(urls) & set(pinned)):
        page: DbPage = read_page(cache=cache, url=url)
        for key, value in (
            ("article_sha256", page.article_sha256),
            ("syntax_sha256", page.syntax_sha256),
        ):
            if pinned[url].get(key) != value:
                found.append(f"{key} changed: {url}")
    return found
