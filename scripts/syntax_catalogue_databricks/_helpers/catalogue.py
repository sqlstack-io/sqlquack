"""Build the Databricks catalogue: sources, forms and definitions from the pinned pages."""

from __future__ import annotations

from pathlib import Path

from scripts.syntax_catalogue._helpers.catalogue import absent_forms, load_existing
from scripts.syntax_catalogue._helpers.forms import emit_forms, group_calls
from scripts.syntax_catalogue.models import Existing, FormGroup, PageContext, Production
from scripts.syntax_catalogue_databricks._helpers.families import family_of, page_slug, source_id
from scripts.syntax_catalogue_databricks._helpers.hand import load_hand
from scripts.syntax_catalogue_databricks._helpers.notation import productions
from scripts.syntax_catalogue_databricks._helpers.pages import read_page
from scripts.syntax_catalogue_databricks.constants import (
    APPLIES_SQL,
    DECISION_APPLIES,
    DECISION_HAND,
    DECISION_READING,
    DECISION_RUNTIME_ONLY,
    DECISION_SYNTAX,
    DEFINITION_SEPARATOR,
    FAMILIES,
    FORM_PREFIX,
    FUNCTIONS,
    HAND_FILE,
    KIND_SPECIAL,
    KIND_STATEMENT,
    OPERATOR_TITLE,
    OPERATOR_WORDS,
    STATEMENT_FAMILIES,
    STATUS_FORMS,
)
from scripts.syntax_catalogue_databricks.models import (
    DbCatalogue,
    DbPage,
    Hand,
    HandPage,
    HandReading,
    RawProduction,
)
from scripts.syntax_catalogue_databricks.types import Row


def page_keywords(page: DbPage) -> frozenset[str]:
    """The lower-case words an operator or predicate page writes as keywords; none elsewhere."""
    if not page.path.startswith(f"{FUNCTIONS}/") or not OPERATOR_TITLE.search(page.title):
        return frozenset()
    return frozenset({*page.title_code.lower().split(), *OPERATOR_WORDS})


def page_calls(page: DbPage) -> frozenset[str]:
    """The function names a functions page documents: its path name and its title's words."""
    if not page.path.startswith(f"{FUNCTIONS}/"):
        return frozenset()
    name: str = page.path.rsplit("/", 1)[-1].lower()
    titled: set[str] = {word.lower() for word in page.title.replace(",", " ").split()}
    return frozenset({name, *titled})


def read_productions(
    *, page: DbPage, readings: dict[tuple[str, str], HandReading]
) -> tuple[list[RawProduction], set[str], set[tuple[str, str]], dict[str, str]]:
    """The page's productions with hand readings applied, and what reading them decided."""
    found: list[RawProduction] = []
    decisions: set[str] = set()
    used: set[tuple[str, str]] = set()
    later: dict[str, str] = {}
    first: str = page.blocks[0].heading if page.blocks else ""
    for block in page.blocks:
        for production in productions(
            text=block.text, calls=page_calls(page), keywords=page_keywords(page)
        ):
            written: str = (
                f"<{production.name}> ::= {production.text}" if production.name else production.text
            )
            reading: HandReading | None = readings.get((page.path, written))
            read: list[RawProduction] = [production]
            if reading is not None:
                used.add((page.path, written))
                decisions.add(DECISION_READING)
                read = [
                    RawProduction(
                        name=line.split(DEFINITION_SEPARATOR, 1)[0][1:-1],
                        text=line.split(DEFINITION_SEPARATOR, 1)[1],
                    )
                    if line.startswith("<") and DEFINITION_SEPARATOR in line
                    else RawProduction(name="", text=line)
                    for line in reading.read
                ]
            found += read
            if block.heading != first:
                for item in read:
                    later.setdefault(item.text, block.heading)
    return found, decisions, used, later


def definitions_of(found: list[RawProduction]) -> list[tuple[str, str]]:
    """Each distinct `<name> ::= body`; a second body under one name is `name #2`."""
    kept: list[tuple[str, str]] = []
    for production in found:
        if not production.name:
            continue
        grammar: str = f"<{production.name}> ::= {production.text}"
        if any(body == grammar for _, body in kept):
            continue
        key: str = production.name
        number: int = 2
        while any(name == key for name, _ in kept):
            key = f"{production.name} #{number}"
            number += 1
        kept.append((key, grammar))
    return kept


def page_groups(
    *, page: DbPage, family: str, found: list[RawProduction], later: dict[str, str]
) -> list[FormGroup]:
    """Statement forms one per production; a functions page's calls by the shared call rule."""
    heading: tuple[str, ...] = (page.title, page.blocks[0].heading if page.blocks else "")
    statements: list[str] = list(dict.fromkeys(p.text for p in found if not p.name and p.text))
    if not statements:
        statements = promoted(found)
    if family == FUNCTIONS:
        groups, _ = group_calls([Production(KIND_STATEMENT, g, heading) for g in statements])
        return groups
    kind: str = KIND_STATEMENT if family in STATEMENT_FAMILIES else KIND_SPECIAL
    return [
        FormGroup(
            kind, "", (grammar,), (page.title, later[grammar]) if grammar in later else heading
        )
        for grammar in statements
    ]


def _referenced(*, production: RawProduction, found: list[RawProduction]) -> bool:
    """True when another production of the page names the definition."""
    return any(f"<{production.name}>" in other.text for other in found if other is not production)


def promoted(found: list[RawProduction]) -> list[str]:
    """A page of definitions only: the bodies of those no other production on it refers to."""
    return list(
        dict.fromkeys(
            p.text for p in found if p.name and not _referenced(production=p, found=found)
        )
    )


def applies_decisions(page: DbPage) -> set[str]:
    """The page's first applicability banner, and whether it leaves out Databricks SQL."""
    if not page.applies:
        return set()
    found: set[str] = {DECISION_APPLIES.format(applies=page.applies[0])}
    if APPLIES_SQL not in page.applies[0]:
        found.add(DECISION_RUNTIME_ONLY)
    return found


def stale_inputs(*, hand: Hand, known: set[str], unused: set[tuple[str, str]]) -> list[str]:
    """Hand rows that name a page outside the pin, and readings that match no production."""
    problems: list[str] = [
        f"hand form on a page not in the pin: {f.page}" for f in hand.forms if f.page not in known
    ]
    problems += [f"[[page]] not in the pin: {p.page}" for p in hand.pages if p.page not in known]
    return problems + [f"[[reading]] matches no production: {p}: {w}" for p, w in sorted(unused)]


def build_catalogue(
    *, cache: Path, out: Path, urls: list[str], retrieved_utc: dict[str, str]
) -> DbCatalogue:
    """Every pinned page's source row, forms and definitions, and the problems found."""
    hand: Hand = load_hand(out / HAND_FILE)
    readings: dict[tuple[str, str], HandReading] = {(r.page, r.written): r for r in hand.readings}
    hand_pages: dict[str, HandPage] = {row.page: row for row in hand.pages}
    existing: Existing = load_existing(out)
    sources: list[Row] = []
    forms: dict[str, list[Row]] = {family: [] for family in FAMILIES}
    definitions: dict[str, list[Row]] = {family: [] for family in FAMILIES}
    problems: list[str] = []
    used_readings: set[tuple[str, str]] = set()
    used_ids: frozenset[str] = frozenset(existing.ids)
    emitted: set[str] = set()
    known: set[str] = set()
    for url in urls:
        page: DbPage = read_page(cache=cache, url=url)
        known.add(page.path)
        family: str = family_of(page.path)
        source: str = source_id(page.path.lower())
        found, decisions, used, later = read_productions(page=page, readings=readings)
        used_readings |= used
        groups: list[FormGroup] = page_groups(page=page, family=family, found=found, later=later)
        if groups:
            decisions.add(DECISION_SYNTAX)
        for form in hand.forms:
            if form.page != page.path:
                continue
            if form.article_sha256 != page.article_sha256:
                problems.append(f"hand form {form.label}: {page.path} changed since its review")
            groups.append(
                FormGroup(form.kind, form.label, form.grammars, (page.title, form.anchor))
            )
            decisions.add(DECISION_HAND.format(basis=form.basis))
        named: list[tuple[str, str]] = definitions_of(found)
        status: str = STATUS_FORMS
        disposition: HandPage | None = hand_pages.get(page.path)
        if groups and disposition is not None:
            problems.append(f"{page.path}: has forms and a [[page]] disposition")
        if not groups:
            if disposition is None:
                problems.append(f"{page.path}: no form and no reviewed [[page]] disposition")
                continue
            if disposition.article_sha256 != page.article_sha256:
                problems.append(f"[[page]] {page.path}: the page changed since its review")
            status = disposition.status
            decisions.add(disposition.reason)
        context: PageContext = PageContext(
            base=f"{FORM_PREFIX}.{family}.{page_slug(page.path)}",
            family=family,
            source=source,
            definitions=tuple(name for name, _ in named),
        )
        rows, used_ids = emit_forms(
            context=context, groups=groups, existing=existing, used_ids=used_ids
        )
        forms[family] += rows
        emitted |= {str(row["id"]) for row in rows}
        definitions[family] += [
            {"id": f"{source}::{name}", "source": source, "name": name, "grammar": grammar}
            for name, grammar in named
            if groups
        ]
        sources.append(
            {
                "id": source,
                "url": page.url,
                "title": page.title,
                "family": family,
                "retrieved_utc": retrieved_utc[url],
                "page_sha256": page.page_sha256,
                "syntax_sha256": page.syntax_sha256,
                "syntax_blocks": len(page.blocks),
                "status": status,
                "decisions": sorted(decisions | applies_decisions(page)),
            }
        )
    for form in absent_forms(existing=existing, emitted=emitted):
        forms[str(form["family"])].append(form)
    problems += stale_inputs(hand=hand, known=known, unused=set(readings) - used_readings)
    return DbCatalogue(sources=sources, forms=forms, definitions=definitions, problems=problems)
