"""Load the pinned pages from the source tree: command refentries and chapter sect1 pages."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.syntax_catalogue._helpers.grammar import sha256
from scripts.syntax_catalogue_pg._helpers.sgml import (
    literals,
    plain,
    section_at,
    sections_of,
    signature_text,
    synopsis_text,
)
from scripts.syntax_catalogue_pg.constants import (
    ADMIN_TITLES,
    CHAPTER_FAMILIES,
    CHAPTERS,
    COMMANDS_PART,
    DML_TITLES,
    ENTITY_FILE,
    ENTITY_REF,
    FUNC_SIGNATURE,
    FUNCTION_TAG,
    HEADER_SIGNATURES,
    QUERY_TITLES,
    REF_DIR,
    REFENTRY_ID,
    REFENTRY_TITLE,
    SECT1,
    SESSION_TITLES,
    SGML_DIR,
    SYNOPSIS,
    SYNOPSIS_DIV,
    UTILITY_TITLES,
)
from scripts.syntax_catalogue_pg.models import PgPage, Section, Signature, SynopsisBlock


def command_family(title: str) -> str:
    """The family of a SQL command page, from its title."""
    for pattern, family in (
        (QUERY_TITLES, "query"),
        (DML_TITLES, "dml"),
        (SESSION_TITLES, "transaction_session"),
        (UTILITY_TITLES, "utility"),
        (ADMIN_TITLES, "security_admin"),
    ):
        if pattern.match(title):
            return family
    return "ddl"


def command_files(tree: Path) -> list[Path]:
    """The refentry files of the SQL Commands reference, in reference order."""
    sgml: Path = tree / SGML_DIR
    files: dict[str, str] = dict(ENTITY_FILE.findall((sgml / "ref" / "allfiles.sgml").read_text()))
    part: re.Match[str] | None = COMMANDS_PART.search((sgml / "reference.sgml").read_text())
    if part is None:
        raise ValueError("reference.sgml has no sql-commands part")
    return [tree / REF_DIR / files[name] for name in ENTITY_REF.findall(part.group(1))]


def heading_of(title: str, stack: tuple[Section, ...], skip: str) -> tuple[str, ...]:
    """The page title followed by the titles of the sections below the page's own."""
    return (title, *(s.title for s in stack if s.tag != skip and s.title))


def blocks_in(*, text: str, title: str, page_tag: str) -> list[SynopsisBlock]:
    """Every synopsis block of a page with its heading path."""
    spans = section_at(text)
    blocks: list[SynopsisBlock] = []
    for match in SYNOPSIS.finditer(text):
        stack: tuple[Section, ...] = sections_of(spans, match.start())
        grammar: str = synopsis_text(match.group(1))
        blocks.append(
            SynopsisBlock(
                heading=heading_of(title, stack, page_tag),
                text=grammar,
                sha256=sha256(grammar),
                in_synopsis_div=any(s.tag == SYNOPSIS_DIV for s in stack),
            )
        )
    return blocks


def signatures_in(*, text: str, title: str) -> list[Signature]:
    """Every function or operator signature of a page, without the table header rows."""
    spans = section_at(text)
    found: list[Signature] = []
    for match in FUNC_SIGNATURE.finditer(text):
        raw: str = match.group(1)
        grammar: str = signature_text(raw)
        if plain(raw) in HEADER_SIGNATURES or not grammar:
            continue
        found.append(
            Signature(
                heading=heading_of(title, sections_of(spans, match.start()), "sect1"),
                grammar=grammar,
                literals=tuple(literals(raw)),
                is_function=bool(FUNCTION_TAG.search(raw)),
                raw_sha256=sha256(raw),
            )
        )
    return found


def command_page(path: Path) -> PgPage:
    """One SQL command page."""
    text: str = path.read_text()
    page_id: re.Match[str] | None = REFENTRY_ID.search(text)
    title_match: re.Match[str] | None = REFENTRY_TITLE.search(text)
    if page_id is None or title_match is None:
        raise ValueError(f"{path}: no refentry id or title")
    title: str = plain(title_match.group(1))
    return PgPage(
        id=page_id.group(1),
        title=title,
        family=command_family(title),
        file=str(path),
        sha256=sha256(text),
        blocks=blocks_in(text=text, title=title, page_tag="refentry"),
        is_command=True,
    )


def chapter_pages(path: Path, family: str, texts: dict[str, str] | None) -> list[PgPage]:
    """Each sect1 of a chapter file as a page, and the chapter's own text when it has one."""
    text: str = path.read_text()
    pages: list[PgPage] = []
    intro_end: int = len(text)
    for match in SECT1.finditer(text):
        intro_end = min(intro_end, match.start())
        body: str = match.group(0)
        title: re.Match[str] | None = re.search(r"<title>(.*?)</title>", body, re.S)
        name: str = plain(title.group(1)) if title else match.group(1)
        if texts is not None:
            texts[match.group(1)] = body
        pages.append(
            PgPage(
                id=match.group(1),
                title=name,
                family=family,
                file=str(path),
                sha256=sha256(body),
                blocks=blocks_in(text=body, title=name, page_tag="sect1"),
                signatures=signatures_in(text=body, title=name),
            )
        )
    chapter: re.Match[str] | None = re.search(r'<chapter\s+id="([^"]+)"', text)
    if chapter is not None:
        intro: str = text[:intro_end]
        if texts is not None:
            texts[chapter.group(1)] = intro
        title = re.search(r"<title>(.*?)</title>", intro, re.S)
        name = plain(title.group(1)) if title else chapter.group(1)
        pages.insert(
            0,
            PgPage(
                id=chapter.group(1),
                title=name,
                family=family,
                file=str(path),
                sha256=sha256(intro),
                blocks=blocks_in(text=intro, title=name, page_tag="chapter"),
                signatures=signatures_in(text=intro, title=name),
            ),
        )
    return pages


def load_pages(tree: Path, texts: dict[str, str] | None = None) -> list[PgPage]:
    """Every pinned page: the SQL command pages, then the SQL-language chapter pages."""
    pages: list[PgPage] = [command_page(path) for path in command_files(tree)]
    for chapter in CHAPTERS:
        pages += chapter_pages(tree / SGML_DIR / chapter, CHAPTER_FAMILIES[chapter], texts)
    ids: list[str] = [page.id for page in pages]
    duplicated: set[str] = {page_id for page_id in ids if ids.count(page_id) > 1}
    if duplicated:
        raise ValueError(f"page ids declared twice: {sorted(duplicated)}")
    return pages
