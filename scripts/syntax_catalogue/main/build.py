"""Build coverage/syntax/sources.toml and coverage/syntax/catalogue/<family>.toml."""

from __future__ import annotations

import argparse
import gzip
import json
import tomllib
from pathlib import Path

from scripts.syntax_catalogue._helpers.family import (
    family_of,
    is_view_page,
    leading_words,
    page_slug,
    source_id,
)
from scripts.syntax_catalogue._helpers.forms import emit_forms, group_forms
from scripts.syntax_catalogue._helpers.grammar import sha256
from scripts.syntax_catalogue._helpers.hand import hand_groups, load_hand_forms
from scripts.syntax_catalogue._helpers.pages import html_title, parse_page, productions
from scripts.syntax_catalogue._helpers.toml_out import write_tables
from scripts.syntax_catalogue.constants import (
    COMMANDS_DIRECTORY,
    CONSTRUCTS_DIRECTORY,
    DECISION_HAND,
    DECISION_MERGED_HEADINGS,
    DECISION_PROSE,
    FAMILIES,
    FORMAT_HTML,
    FORMAT_MARKDOWN,
    HTTP_OK,
    KIND_DEFINITION,
    KIND_PROSE,
    KIND_STATEMENT,
    MARKDOWN_SUFFIX,
    PAGE_FORMS,
    PAGE_NO_SYNTAX,
    PAGE_OUT_OF_SCOPE,
    REFERENCE_ROOT,
    STATUS_ABSENT,
    TOP_LEVEL_PARTS,
)
from scripts.syntax_catalogue.models import (
    Existing,
    FormGroup,
    HandForm,
    Page,
    PageContext,
    PageSource,
    Production,
)

type Row = dict[str, object]


def load_existing(out: Path) -> Existing:
    """The ids the current catalogue and aliases.toml have already used."""
    existing: Existing = Existing(forms={}, by_key={}, by_source={}, ids=set())
    catalogue: Path = out / "catalogue"
    for path in sorted(catalogue.glob("*.toml")) if catalogue.exists() else []:
        for form in tomllib.loads(path.read_text()).get("form", []):
            existing.forms[form["id"]] = form
            existing.by_key[(form["source"], form["grammar_sha256"])] = form["id"]
            existing.by_source.setdefault(form["source"], []).append(form["id"])
    existing.ids = set(existing.forms)
    aliases: Path = out / "aliases.toml"
    if aliases.exists():
        existing.ids |= {a["from"] for a in tomllib.loads(aliases.read_text()).get("alias", [])}
    return existing


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


def read_page(cache: Path, record: PageSource) -> Page:
    """The parsed page from the private cache; an HTML-only page has a title and no blocks."""
    text: str = gzip.decompress((cache / "raw" / (record.path + ".gz")).read_bytes()).decode()
    if record.page_format == FORMAT_HTML:
        return Page(html_title(text), (), ())
    return parse_page(text)


def unique_statements(statements: list[Production], decisions: set[str]) -> list[Production]:
    """Statements with grammar documented under several headings kept once."""
    seen: dict[str, Production] = {}
    for production in statements:
        if production.grammar in seen:
            seen[production.grammar].notes.append(" / ".join(production.heading[1:]))
            decisions.add(DECISION_MERGED_HEADINGS)
            continue
        seen[production.grammar] = production
    return list(seen.values())


def page_groups(
    page: Page, *, family: str, record: PageSource, hand: list[HandForm], lead: set[str]
) -> tuple[list[FormGroup], list[tuple[str, str]], set[str]]:
    """The page's form groups, its sub-grammar definitions and the decisions taken."""
    parts: list[str] = record.path.split("/")
    nested: bool = len(parts) > TOP_LEVEL_PARTS
    prods: list[Production] = productions(
        page,
        leading_words=lead if nested and parts[1] == COMMANDS_DIRECTORY else None,
        syntax_sections_only=nested and parts[1] == CONSTRUCTS_DIRECTORY,
    )
    decisions: set[str] = set()
    statements: list[Production] = unique_statements(
        [p for p in prods if p.kind == KIND_STATEMENT], decisions
    )
    groups, more = group_forms(family, prods, statements)
    decisions |= more
    hand_list: list[FormGroup] = hand_groups(hand, title=page.title, page_sha256=record.sha256)
    decisions |= {DECISION_HAND.format(basis=form.basis) for form in hand}
    hand_anchors: set[str] = {g.heading[-1] for g in hand_list}
    for missing in page.syntax_sections_without_block:
        if missing not in hand_anchors:
            decisions.add(DECISION_PROSE.format(heading=missing))
            groups.append(FormGroup(KIND_PROSE, missing, (), (page.title, missing)))
    definitions: dict[str, str] = {p.name: p.grammar for p in prods if p.kind == KIND_DEFINITION}
    return groups + hand_list, list(definitions.items()), decisions


def build(cache: Path, out: Path, hand_path: Path) -> dict[str, object]:
    """Sources, forms by family and definitions by family for every cached page."""
    pages: list[tuple[PageSource, Page]] = [(s, read_page(cache, s)) for s in latest_pages(cache)]
    commands: list[str] = [
        page.title
        for s, page in pages
        if s.path.startswith(f"{REFERENCE_ROOT}/{COMMANDS_DIRECTORY}/")
    ]
    lead: set[str] = leading_words(commands)
    hand: dict[str, list[HandForm]] = load_hand_forms(hand_path)
    existing: Existing = load_existing(out)
    used_ids: set[str] = set(existing.ids)
    sources: list[Row] = []
    forms: dict[str, list[Row]] = {family: [] for family in FAMILIES}
    definitions: dict[str, list[Row]] = {family: [] for family in FAMILIES}
    for record, page in pages:
        sid: str = source_id(record.path)
        family: str | None = family_of(record.path, page.title)
        groups: list[FormGroup] = []
        defined: list[tuple[str, str]] = []
        decisions: set[str] = set()
        if family is not None:
            groups, defined, decisions = page_groups(
                page, family=family, record=record, hand=hand.pop(sid, []), lead=lead
            )
        if family is not None and not groups and is_view_page(record.path):
            family = None
        status: str = (
            PAGE_OUT_OF_SCOPE if family is None else PAGE_FORMS if groups else PAGE_NO_SYNTAX
        )
        sources.append(
            source_row(record, page=page, family=family, status=status, decisions=decisions)
        )
        if family is None or not groups:
            continue
        definitions[family] += [
            {"id": f"{sid}::{name}", "source": sid, "name": name, "grammar": grammar}
            for name, grammar in defined
        ]
        context: PageContext = PageContext(
            f"sf.{family}.{page_slug(record.path)}", family, sid, tuple(n for n, _ in defined)
        )
        forms[family] += emit_forms(context, groups, existing, used_ids)
    if hand:
        raise ValueError(f"hand forms for pages not in the cache: {sorted(hand)}")
    emitted: set[str] = {str(f["id"]) for rows in forms.values() for f in rows}
    for form_id, form in existing.forms.items():
        if form_id not in emitted:
            forms[str(form["family"])].append(dict(form, status=STATUS_ABSENT))
    return {"sources": sources, "forms": forms, "definitions": definitions}


def source_row(
    record: PageSource, *, page: Page, family: str | None, status: str, decisions: set[str]
) -> Row:
    """One sources.toml record."""
    syntax_text: str = "\n\n".join(block.text for block in page.blocks)
    return {
        "id": source_id(record.path),
        "url": record.url.removesuffix(MARKDOWN_SUFFIX),
        "title": page.title,
        "family": family or "",
        "retrieved_utc": record.retrieved_utc,
        "page_sha256": record.sha256,
        "syntax_sha256": sha256(syntax_text),
        "syntax_blocks": len(page.blocks),
        "status": status,
        "decisions": sorted(decisions),
    }


def write_catalogue(result: dict[str, object], *, cache: Path, out: Path) -> dict[str, int]:
    """Write indexes.toml, sources.toml and one catalogue file per family; counts by family."""
    (out / "catalogue").mkdir(parents=True, exist_ok=True)
    indexes: list[Row] = [
        {"name": f.name, "sha256": sha256(f.read_text())} for f in sorted((cache / "idx").glob("*"))
    ]
    write_tables(
        out / "indexes.toml",
        header="# Documentation indexes the pinned page list was read from (llms.txt files and the "
        "docs navigation tree).",
        tables=[("index", indexes)],
    )
    sources: list[Row] = result["sources"]  # ty: ignore[invalid-assignment]
    write_tables(
        out / "sources.toml",
        header="# Pinned Snowflake documentation pages; generated by scripts/build_syntax_catalogue.py.\n"
        "# Hashes are of the fetched Markdown and of its sqlsyntax blocks; the pages stay private.",
        tables=[("source", sources)],
    )
    forms: dict[str, list[Row]] = result["forms"]  # ty: ignore[invalid-assignment]
    definitions: dict[str, list[Row]] = result["definitions"]  # ty: ignore[invalid-assignment]
    for family in FAMILIES:
        write_tables(
            out / "catalogue" / f"{family}.toml",
            header=f"# Documented Snowflake syntax forms, family {family}; generated by "
            "scripts/build_syntax_catalogue.py.",
            tables=[
                ("form", sorted(forms[family], key=lambda f: str(f["id"]))),
                ("definition", sorted(definitions[family], key=lambda d: str(d["id"]))),
            ],
        )
    return {family: len(forms[family]) for family in FAMILIES}


def run_build(argv: list[str] | None = None) -> int:
    """Regenerate the catalogue from the private page cache."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=Path("coverage/syntax"))
    arguments: argparse.Namespace = parser.parse_args(argv)
    hand_path: Path = arguments.out / "hand.toml"
    result: dict[str, object] = build(arguments.cache, arguments.out, hand_path)
    print(json.dumps(write_catalogue(result, cache=arguments.cache, out=arguments.out)))
    return 0
