"""Read the pinned Markdown pages: front matter, headings, diagrams, signatures and tables."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.grammar_catalogue._helpers.grammar import sha256, slug
from scripts.syntax_catalogue_duckdb.constants import (
    DIAGRAM_DIV,
    DOCS_ROOT,
    FENCE,
    FRONT_MATTER,
    HEADING,
    MARKDOWN_GLOB,
    PAGE_EXCLUSIONS,
    PAGE_GLOBS,
    SIGNATURE_HEADING,
    SITE_URL,
    TABLE_ROW,
    WEB_DIR,
)
from scripts.syntax_catalogue_duckdb.models import DuckDbPage, Located


def front_matter(text: str) -> dict[str, str]:
    """The page's scalar front-matter fields; lists such as redirect_from are left out."""
    match: re.Match[str] | None = FRONT_MATTER.match(text)
    fields: dict[str, str] = {}
    for line in match.group(1).splitlines() if match else []:
        key, separator, value = line.partition(":")
        if separator and not line.startswith((" ", "-")) and value.strip():
            fields[key.strip()] = value.strip().strip("'\"")
    return fields


def located(text: str) -> tuple[list[Located], list[Located], list[Located], list[Located]]:
    """Diagrams, signature headings, table rows and code blocks, each with its heading path."""
    headings: list[str] = []
    diagrams: list[Located] = []
    signatures: list[Located] = []
    rows: list[Located] = []
    blocks: list[Located] = []
    fence: list[str] | None = None
    table: int = 0
    in_table: bool = False
    body: str = FRONT_MATTER.sub("", text, count=1)
    for line in body.splitlines():
        if fence is not None:
            if FENCE.match(line):
                blocks.append(Located(tuple(headings), "\n".join(fence)))
                fence = None
            else:
                fence.append(line)
            continue
        if FENCE.match(line):
            fence = []
            continue
        heading: re.Match[str] | None = HEADING.match(line)
        if heading:
            depth: int = len(heading.group(1))
            headings = [*(headings + [""] * depth)[: depth - 1], heading.group(2).strip()]
            signature: re.Match[str] | None = SIGNATURE_HEADING.match(heading.group(2).strip())
            if signature:
                parents: tuple[str, ...] = tuple(part for part in headings[:-1] if part)
                signatures.append(Located(parents, signature.group(1), depth))
            in_table = False
            continue
        path: tuple[str, ...] = tuple(part for part in headings if part)
        diagram: re.Match[str] | None = DIAGRAM_DIV.search(line)
        if diagram:
            diagrams.append(Located(path, diagram.group(1)))
        elif TABLE_ROW.match(line):
            if not in_table:
                table += 1
            rows.append(Located(path, line, table=table))
        in_table = bool(TABLE_ROW.match(line))
    return diagrams, signatures, rows, blocks


def page_id(path: str) -> str:
    """`doc.<segment>.<segment>…` from the page path under the docs root, without `.md`."""
    return "doc." + ".".join(slug(part) for part in Path(path).with_suffix("").parts)


def in_scope(root: Path) -> set[Path]:
    """The pages PAGE_GLOBS selects under the docs root."""
    paths: set[Path] = set()
    for pattern in PAGE_GLOBS:
        paths.update(root.glob(pattern))
    return paths


def undecided_pages(cache: Path) -> list[str]:
    """Pages neither in the page set nor in a reviewed excluded group, and stale exclusions."""
    root: Path = cache / WEB_DIR / DOCS_ROOT
    rest: set[Path] = set(root.glob(MARKDOWN_GLOB)) - in_scope(root)
    decided: set[Path] = set()
    problems: list[str] = []
    for pattern, _ in PAGE_EXCLUSIONS:
        matched: set[Path] = rest & set(root.glob(pattern))
        if not matched:
            problems.append(f"excluded page group {pattern} matches no page outside the page set")
        decided |= matched
    problems += [
        f"page {path.relative_to(root).as_posix()} is neither in the page set "
        "nor in a reviewed excluded page group"
        for path in sorted(rest - decided)
    ]
    return problems


def load_pages(cache: Path) -> list[DuckDbPage]:
    """Every in-scope page of the pinned docs tree, in path order."""
    root: Path = cache / WEB_DIR / DOCS_ROOT
    paths: set[Path] = in_scope(root)
    pages: list[DuckDbPage] = []
    for path in sorted(paths):
        text: str = path.read_text(encoding="utf-8")
        relative: str = path.relative_to(root).as_posix()
        fields: dict[str, str] = front_matter(text)
        diagrams, signatures, rows, blocks = located(text)
        pages.append(
            DuckDbPage(
                path=relative,
                id=page_id(relative),
                url=SITE_URL + str(Path(relative).with_suffix("")),
                title=fields.get("title", relative),
                railroad=fields.get("railroad", ""),
                page_sha256=sha256(text),
                diagrams=tuple(diagrams),
                signatures=tuple(signatures),
                rows=tuple(rows),
                blocks=tuple(blocks),
            )
        )
    return pages
