"""Read the private page cache and build the catalogue's sources, forms and definitions."""

from __future__ import annotations

import gzip
import json
import re
from pathlib import Path

from scripts.grammar_catalogue._helpers.existing import absent_forms, load_existing
from scripts.grammar_catalogue._helpers.forms import emit_forms, group_forms
from scripts.grammar_catalogue._helpers.grammar import sha256
from scripts.grammar_catalogue._helpers.pages import html_title, parse_page, productions
from scripts.grammar_catalogue.constants import (
    DECISION_HAND,
    FAMILIES,
    HTTP_OK,
    KIND_DEFINITION,
    KIND_PROSE,
    KIND_STATEMENT,
    MARKDOWN_SUFFIX,
    TOP_LEVEL_PARTS,
)
from scripts.grammar_catalogue.exceptions import CatalogueError
from scripts.grammar_catalogue.models import (
    Catalogue,
    Existing,
    FormGroup,
    HandForm,
    Page,
    PageContext,
    PageResult,
    PageSource,
    Production,
)
from scripts.grammar_catalogue.types import Row
from scripts.syntax_catalogue_snowflake._helpers.family import (
    family_of,
    is_view_page,
    leading_words,
    page_slug,
    source_id,
)
from scripts.syntax_catalogue_snowflake._helpers.fold import fold_restatements, load_folds
from scripts.syntax_catalogue_snowflake._helpers.hand import hand_groups, load_hand_forms
from scripts.syntax_catalogue_snowflake.constants import (
    COMMANDS_DIRECTORY,
    CONSTRUCTS_DIRECTORY,
    DECISION_MERGED_HEADINGS,
    DECISION_PROSE,
    DEFINITION_QUALIFIER,
    FORMAT_HTML,
    FORMAT_MARKDOWN,
    PAGE_FORMS,
    PAGE_NO_SYNTAX,
    PAGE_OUT_OF_SCOPE,
    REFERENCE_ROOT,
)


def latest_pages(cache: Path) -> list[PageSource]:
    """The last successful fetch of every page in the cache, by path."""
    latest: dict[str, PageSource] = {}
    for line in (cache / "fetch-log.jsonl").read_text().splitlines():
        record: dict[str, object] = json.loads(line)
        if record["status"] == HTTP_OK:
            path: str = str(record["path"])
            latest[path] = PageSource(
                path,
                str(record["url"]),
                str(record["retrieved_utc"]),
                str(record["sha256"]),
                str(record.get("format", FORMAT_MARKDOWN)),
            )
    return [latest[path] for path in sorted(latest)]


def read_page(*, cache: Path, record: PageSource) -> Page:
    """The parsed page from the private cache; an HTML-only page has a title and no blocks."""
    text: str = gzip.decompress((cache / "raw" / (record.path + ".gz")).read_bytes()).decode()
    if record.page_format == FORMAT_HTML:
        return Page(html_title(text), (), ())
    return parse_page(text)


def unique_statements(statements: list[Production]) -> tuple[list[Production], set[str]]:
    """Statements with grammar documented under several headings kept once."""
    seen: dict[str, Production] = {}
    for production in statements:
        seen.setdefault(production.grammar, production)
    merged: bool = len(seen) < len(statements)
    return list(seen.values()), {DECISION_MERGED_HEADINGS} if merged else set()


def page_productions(*, page: Page, path: str, lead: set[str]) -> list[Production]:
    """The page's productions, read with the rules for its directory."""
    parts: list[str] = path.split("/")
    nested: bool = len(parts) > TOP_LEVEL_PARTS
    return productions(
        page=page,
        leading_words=lead if nested and parts[1] == COMMANDS_DIRECTORY else None,
        syntax_sections_only=nested and parts[1] == CONSTRUCTS_DIRECTORY,
    )


def prose_groups(*, page: Page, hand_list: list[FormGroup]) -> tuple[list[FormGroup], set[str]]:
    """Prose forms for syntax headings with no block that no hand form covers."""
    covered: set[str] = {g.heading[-1] for g in hand_list}
    missing: list[str] = [h for h in page.syntax_sections_without_block if h not in covered]
    groups: list[FormGroup] = [FormGroup(KIND_PROSE, h, (), (page.title, h)) for h in missing]
    return groups, {DECISION_PROSE.format(heading=h) for h in missing}


def page_groups(
    *, page: Page, family: str, record: PageSource, hand: list[HandForm], lead: set[str]
) -> PageResult:
    """The page's form groups, its sub-grammar definitions and the decisions taken."""
    prods: list[Production] = page_productions(page=page, path=record.path, lead=lead)
    statements, decisions = unique_statements([p for p in prods if p.kind == KIND_STATEMENT])
    groups, grouping = group_forms(family=family, productions=prods, statements=statements)
    hand_list: list[FormGroup] = hand_groups(
        forms=hand, title=page.title, page_sha256=record.sha256
    )
    prose, prose_decisions = prose_groups(page=page, hand_list=hand_list)
    hand_decisions: set[str] = {DECISION_HAND.format(basis=form.basis) for form in hand}
    definitions: dict[str, str] = {}
    for prod in prods:
        if prod.kind == KIND_DEFINITION:
            definitions[definition_key(name=prod.name, grammar=prod.grammar, taken=definitions)] = (
                prod.grammar
            )
    return PageResult(
        groups=groups + prose + hand_list,
        definitions=list(definitions.items()),
        decisions=decisions | grouping | prose_decisions | hand_decisions,
    )


def definition_key(*, name: str, grammar: str, taken: dict[str, str]) -> str:
    """The name, with its `(for ...)` qualifier when the page gives one, unique on the page."""
    qualifier: re.Match[str] | None = DEFINITION_QUALIFIER.match(grammar)
    key: str = f"{name} {qualifier[1]}" if qualifier else name
    number: int = 1
    unique: str = key
    while unique in taken:
        number += 1
        unique = f"{key} #{number}"
    return unique


def source_row(
    *, record: PageSource, page: Page, family: str | None, status: str, decisions: set[str]
) -> Row:
    """One sources.toml record."""
    return {
        "id": source_id(record.path),
        "url": record.url.removesuffix(MARKDOWN_SUFFIX),
        "title": page.title,
        "family": family or "",
        "retrieved_utc": record.retrieved_utc,
        "page_sha256": record.sha256,
        "syntax_sha256": sha256("\n\n".join(block.text for block in page.blocks)),
        "syntax_blocks": len(page.blocks),
        "status": status,
        "decisions": sorted(decisions),
    }


def build_catalogue(*, cache: Path, out: Path, hand_path: Path) -> Catalogue:
    """Sources, forms by family and definitions by family for every cached page."""
    pages: list[tuple[PageSource, Page]] = [
        (record, read_page(cache=cache, record=record)) for record in latest_pages(cache)
    ]
    prefix: str = f"{REFERENCE_ROOT}/{COMMANDS_DIRECTORY}/"
    lead: set[str] = leading_words([p.title for s, p in pages if s.path.startswith(prefix)])
    hand: dict[str, list[HandForm]] = load_hand_forms(hand_path)
    existing: Existing = load_existing(out)
    used_ids: frozenset[str] = frozenset(existing.ids)
    catalogue: Catalogue = Catalogue([], {f: [] for f in FAMILIES}, {f: [] for f in FAMILIES})
    for record, page in pages:
        sid: str = source_id(record.path)
        family: str | None = family_of(path=record.path, title=page.title)
        result: PageResult = PageResult([], [], set())
        if family is not None:
            result = page_groups(
                page=page, family=family, record=record, hand=hand.pop(sid, []), lead=lead
            )
        if family is not None and not result.groups and is_view_page(record.path):
            family = None
        status: str = (
            PAGE_OUT_OF_SCOPE if family is None else PAGE_FORMS if result.groups else PAGE_NO_SYNTAX
        )
        catalogue.sources.append(
            source_row(
                record=record, page=page, family=family, status=status, decisions=result.decisions
            )
        )
        if family is None or not result.groups:
            continue
        catalogue.definitions[family].extend(
            {"id": f"{sid}::{key}", "source": sid, "name": key, "grammar": grammar}
            for key, grammar in result.definitions
        )
        names: tuple[str, ...] = tuple(k for k, _ in result.definitions)
        context: PageContext = PageContext(
            f"sf.{family}.{page_slug(record.path)}", family, sid, names
        )
        forms, used_ids = emit_forms(
            context=context, groups=result.groups, existing=existing, used_ids=used_ids
        )
        catalogue.forms[family].extend(forms)
    if hand:
        raise CatalogueError(f"hand forms for pages not in the cache: {sorted(hand)}")
    catalogue = fold_restatements(catalogue=catalogue, hand_folds=load_folds(hand_path))
    emitted: set[str] = set()
    for rows in catalogue.forms.values():
        emitted |= {str(row["id"]) for row in rows}
    for form in absent_forms(existing=existing, emitted=emitted):
        catalogue.forms[str(form["family"])].append(form)
    return catalogue
