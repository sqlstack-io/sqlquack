"""Read the pinned Markdown pages: headings, syntax blocks, their positions and table rows."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.syntax_catalogue_bigquery._helpers.fetch import fetched_pages, page_id
from scripts.syntax_catalogue_bigquery.constants import (
    BOLD_LABEL,
    DOCS_DIR,
    EXAMPLE_HEADING,
    EXAMPLE_LABEL,
    FENCE,
    HEADING,
    HTTP_OK,
    INDENT,
    MARKDOWN_SUFFIX,
    PAGES_DIR,
    POSITION_EXAMPLE,
    POSITION_OTHER,
    POSITION_SYNOPSIS,
    POSITION_SYNTAX,
    SYNTAX_HEADING,
    SYNTAX_LABEL,
    SYNTAX_LEAD,
    SYNTAX_LEAD_LABEL,
    TABLE_ROW,
)
from scripts.syntax_catalogue_bigquery.models import BigQueryPage, Located


def position(*, heading: tuple[str, ...], label: str, prose: bool) -> str:
    """Where a block sits: an example, a syntax section, a synopsis, or among prose."""
    if (label and EXAMPLE_LABEL.match(label)) or any(EXAMPLE_HEADING.search(h) for h in heading):
        return POSITION_EXAMPLE
    if (label and SYNTAX_LABEL.search(label)) or any(SYNTAX_HEADING.search(h) for h in heading):
        return POSITION_SYNTAX
    return POSITION_OTHER if prose else POSITION_SYNOPSIS


class _Scanner:
    """One pass over a page's lines: the heading path, the label and prose state, and output."""

    def __init__(self, text: str) -> None:
        self.lines: list[str] = text.splitlines()
        self.index: int = 0
        self.title: str = ""
        self.headings: list[str] = []
        self.label: str = ""
        self.prose: bool = False
        self.blocks: list[Located] = []
        self.rows: list[Located] = []
        self.table: int = 0
        self.in_table: bool = False

    def path(self) -> tuple[str, ...]:
        """The heading path below the title."""
        return tuple(h for h in self.headings[1:] if h)

    def block(self, *, body: str, fenced: bool) -> None:
        """Record a code block where it sits; a `Syntax:` lead covers only the next block."""
        where: str = position(heading=self.path(), label=self.label, prose=self.prose)
        self.blocks.append(Located(self.path(), body, fenced, 0, where))
        self.label = "" if self.label == SYNTAX_LEAD_LABEL else self.label

    def fence(self) -> None:
        """A fenced block from its opening fence through its closing one."""
        body: list[str] = []
        self.index += 1
        while self.index < len(self.lines) and not FENCE.match(self.lines[self.index]):
            body.append(self.lines[self.index])
            self.index += 1
        self.block(body="\n".join(body), fenced=True)
        self.index += 1

    def indented(self) -> None:
        """An indented block: indented and blank lines after a blank line."""
        body: list[str] = []
        lines: list[str] = self.lines
        while self.index < len(lines) and (
            INDENT.match(lines[self.index]) or not lines[self.index].strip()
        ):
            body.append(lines[self.index][4:])
            self.index += 1
        self.block(body="\n".join(body).strip("\n"), fenced=False)

    def marker(self, line: str) -> bool:
        """Read a heading, a bold label or a `Syntax:` lead; False for any other line."""
        heading: re.Match[str] | None = HEADING.match(line)
        bold: re.Match[str] | None = BOLD_LABEL.match(line)
        if heading:
            depth: int = len(heading.group(1))
            if depth == 1 and not self.title:
                self.title = heading.group(2).strip()
            self.headings = [
                *(self.headings + [""] * depth)[: depth - 1],
                heading.group(2).strip(),
            ]
            self.label, self.prose, self.in_table = "", False, False
        elif bold:
            self.label, self.prose = bold.group(1).strip(), False
        elif SYNTAX_LEAD.search(line):
            self.label, self.prose = SYNTAX_LEAD_LABEL, False
        else:
            return False
        self.index += 1
        return True

    def other(self, line: str) -> None:
        """A table row or a prose line."""
        if TABLE_ROW.match(line):
            if not self.in_table:
                self.table += 1
            self.rows.append(Located(self.path(), line, False, self.table, ""))
            self.in_table = True
        else:
            self.in_table = False
        if line.strip() and not line.startswith(">"):
            self.prose = True
        self.index += 1

    def scan(self) -> tuple[str, list[Located], list[Located]]:
        """The title, blocks and table rows of the whole page."""
        while self.index < len(self.lines):
            line: str = self.lines[self.index]
            after_blank: bool = self.index == 0 or not self.lines[self.index - 1].strip()
            if FENCE.match(line):
                self.fence()
            elif self.marker(line):
                continue
            elif INDENT.match(line) and after_blank and not self.in_table:
                self.indented()
            else:
                self.other(line)
        return self.title, self.blocks, self.rows


def located(text: str) -> tuple[str, list[Located], list[Located]]:
    """The page title, its code blocks (fenced or indented) and its table rows."""
    return _Scanner(text).scan()


def load_pages(cache: Path) -> list[BigQueryPage]:
    """Every fetched page of the pinned reference, in path order."""
    pages: list[BigQueryPage] = []
    for record in fetched_pages(cache):
        if record.status != HTTP_OK:
            continue
        text: str = (cache / DOCS_DIR / PAGES_DIR / f"{record.path}.md").read_text()
        title, blocks, rows = located(text)
        pages.append(
            BigQueryPage(
                path=record.path,
                id=page_id(record.path),
                url=record.url.removesuffix(MARKDOWN_SUFFIX),
                title=title or record.path,
                retrieved_utc=record.retrieved_utc,
                page_sha256=record.sha256,
                blocks=tuple(blocks),
                rows=tuple(rows),
                text=text,
            )
        )
    return sorted(pages, key=lambda page: page.path)
