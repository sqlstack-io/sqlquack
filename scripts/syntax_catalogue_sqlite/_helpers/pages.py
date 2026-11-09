"""Read the pinned documentation archive: its page set, exclusions and syntax diagrams."""

from __future__ import annotations

import fnmatch
import hashlib
import html
import re
import zipfile
from pathlib import Path

from scripts.grammar_catalogue._helpers.grammar import slug
from scripts.syntax_catalogue_sqlite._helpers.fetch import archive_path
from scripts.syntax_catalogue_sqlite.constants import (
    ANNOTATION_STYLE,
    DOCS,
    DOCS_PREFIX,
    HTML_SUFFIX,
    PAGE_EXCLUSIONS,
    PAGE_GLOBS,
    PIKCHR_CLASS,
    SITE_URL,
    SPACES,
    SVG,
    SVG_TEXT,
    SYNTAX_DIR,
    TAG,
    TITLE,
)
from scripts.syntax_catalogue_sqlite.models import Diagram, SqlitePage


def expand_braces(pattern: str) -> list[str]:
    """`{a,b}.html` as `a.html` and `b.html`; a pattern without braces as itself."""
    match: re.Match[str] | None = re.search(r"\{([^{}]*)\}", pattern)
    if match is None:
        return [pattern]
    expanded: list[str] = []
    for choice in match.group(1).split(","):
        expanded += expand_braces(pattern[: match.start()] + choice + pattern[match.end() :])
    return expanded


def matches(*, path: str, pattern: str) -> bool:
    """Whether a page path matches a glob whose `*` stays within one directory."""
    return any(
        fnmatch.fnmatchcase(path, one) and path.count("/") == one.count("/")
        for one in expand_braces(pattern)
    )


def html_members(cache: Path) -> dict[str, bytes]:
    """Every HTML page of the documentation archive, by its path inside the docs tree."""
    found: dict[str, bytes] = {}
    with zipfile.ZipFile(archive_path(cache=cache, name=DOCS)) as opened:
        for info in opened.infolist():
            if info.filename.startswith(DOCS_PREFIX) and info.filename.endswith(HTML_SUFFIX):
                found[info.filename.removeprefix(DOCS_PREFIX)] = opened.read(info)
    return found


def page_id(path: str) -> str:
    """`doc.` and the page's path as slugs: `doc.lang-select`, `doc.syntax.expr`."""
    parts: list[str] = path.removesuffix(HTML_SUFFIX).split("/")
    return "doc." + ".".join(slug(part) for part in parts)


def in_page_set(path: str) -> bool:
    """Whether a page is in the catalogued page set."""
    return any(matches(path=path, pattern=pattern) for pattern in PAGE_GLOBS)


def exclusion_of(path: str) -> str:
    """The first excluded page group a page belongs to, or an empty string."""
    return next(
        (pattern for pattern, _ in PAGE_EXCLUSIONS if matches(path=path, pattern=pattern)), ""
    )


def text_of(fragment: str) -> str:
    """A fragment's text: tags dropped, entities read, non-breaking and other spaces single."""
    return SPACES.sub(" ", html.unescape(TAG.sub("", fragment)).replace("\xa0", " ")).strip()


def diagrams_of(*, text: str, name: str) -> tuple[Diagram, ...]:
    """The page's pikchr syntax diagrams; a syntax/ page's own diagram takes the page's name."""
    found: list[Diagram] = []
    for svg in SVG.findall(text):
        if PIKCHR_CLASS not in svg.split(">", 1)[0]:
            continue
        labels: list[str] = []
        annotations: list[str] = []
        for attributes, label in SVG_TEXT.findall(svg):
            target: list[str] = annotations if ANNOTATION_STYLE in attributes else labels
            target.append(html.unescape(label).replace("\xa0", " "))
        digest: str = hashlib.sha256(svg.encode()).hexdigest()
        found.append(Diagram(name, digest, tuple(labels), tuple(annotations)))
    return tuple(found)


def syntax_diagrams(members: dict[str, bytes]) -> dict[str, Diagram]:
    """The diagram of every syntax/ page, by the SHA-256 of its SVG."""
    found: dict[str, Diagram] = {}
    for path, data in members.items():
        if path.startswith(SYNTAX_DIR):
            name: str = path.removeprefix(SYNTAX_DIR).removesuffix(HTML_SUFFIX)
            for diagram in diagrams_of(text=data.decode("utf-8", errors="replace"), name=name):
                found[diagram.svg_sha256] = diagram
    return found


def load_pages(cache: Path) -> tuple[list[SqlitePage], list[str]]:
    """The page set's pages (diagrams: SVGs equal to a syntax/ page's) and unplaced pages."""
    members: dict[str, bytes] = html_members(cache)
    syntax: dict[str, Diagram] = syntax_diagrams(members)
    pages: list[SqlitePage] = []
    problems: list[str] = []
    used: set[str] = set()
    for path in sorted(members):
        text: str = members[path].decode("utf-8", errors="replace")
        found: tuple[Diagram, ...] = tuple(
            syntax[d.svg_sha256] for d in diagrams_of(text=text, name="") if d.svg_sha256 in syntax
        )
        if not in_page_set(path):
            group: str = exclusion_of(path)
            used.add(group)
            if not group:
                problems.append(f"{path}: neither in the page set nor in an excluded group")
            elif found:
                problems.append(f"{path}: excluded by {group} but holds a syntax diagram")
            continue
        title_match: re.Match[str] | None = TITLE.search(text)
        pages.append(
            SqlitePage(
                path=path,
                id=page_id(path),
                url=SITE_URL + path,
                title=text_of(title_match.group(1)) if title_match else path,
                page_sha256=hashlib.sha256(members[path]).hexdigest(),
                html=text,
                diagrams=found,
            )
        )
    problems += [
        f"excluded group excludes nothing: {pattern}"
        for pattern, _ in PAGE_EXCLUSIONS
        if pattern not in used
    ]
    return pages, problems
