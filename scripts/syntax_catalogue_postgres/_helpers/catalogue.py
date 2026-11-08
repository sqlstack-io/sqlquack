"""Build the PostgreSQL catalogue's sources, forms and definitions from the pinned tree."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.syntax_catalogue._helpers.catalogue import absent_forms, load_existing
from scripts.syntax_catalogue._helpers.forms import emit_forms
from scripts.syntax_catalogue._helpers.grammar import normalize, sha256, slug
from scripts.syntax_catalogue.exceptions import CatalogueError
from scripts.syntax_catalogue.models import Catalogue, Existing, FormGroup, PageContext
from scripts.syntax_catalogue_postgres._helpers.groups import (
    call_groups,
    required_without_options,
    signature_groups,
    statement_groups,
    statement_labels,
)
from scripts.syntax_catalogue_postgres._helpers.hand import load_hand
from scripts.syntax_catalogue_postgres._helpers.pages import load_pages
from scripts.syntax_catalogue_postgres._helpers.sgml import plain, synopsis_text
from scripts.syntax_catalogue_postgres._helpers.synopsis import (
    definition_grammar,
    read_synopsis,
    split_at,
)
from scripts.syntax_catalogue_postgres.constants import (
    ALIAS_SEPARATOR,
    ANCHOR_FIELD,
    CLASH_SIZE,
    DECISION_BLOCKS,
    DECISION_DEFINITIONS,
    DECISION_HAND,
    DECISION_RESTATED,
    DECISION_STATEMENTS,
    DISPOSITION_CALLS,
    DISPOSITION_DEFINITION,
    DISPOSITION_FORM,
    DISPOSITION_RESTATED,
    DOCS_URL,
    EXTRA_LEADS,
    FAMILIES,
    FUNCTIONS_FAMILY,
    FUNCTIONS_ID_PREFIX,
    ID_PREFIX,
    KIND_SPECIAL,
    RETURN_TAIL,
    STATUS_FORMS,
    STATUS_NO_SYNTAX,
    STATUS_RESTATED,
    SYNOPSIS_HEADING,
    TABLE,
    TBODY,
    TYPE_ROW,
    TYPE_TABLES,
)
from scripts.syntax_catalogue_postgres.models import (
    Definition,
    Disposition,
    HandForm,
    HandInputs,
    PageBuild,
    PgCatalogue,
    PgPage,
    Signature,
    SynopsisBlock,
)
from scripts.syntax_catalogue_postgres.types import Row


def add_groups(*, build: PageBuild, family: str, groups: list[FormGroup]) -> PageBuild:
    """Add form groups under a family."""
    if groups:
        build.groups.setdefault(family, []).extend(groups)
    return build


def page_slug(page: PgPage) -> str:
    """The id segment of a page: a command's title, or a chapter section id."""
    if page.is_command:
        return slug(page.title)
    return page.id.removeprefix(FUNCTIONS_ID_PREFIX) if page.family == FUNCTIONS_FAMILY else page.id


def productions_of(text: str) -> list[str]:
    """A narrative block's productions: each unindented line at bracket depth zero starts one."""
    lines: list[str] = [line.rstrip() for line in text.expandtabs().splitlines()]
    base: int = min((len(ln) - len(ln.lstrip()) for ln in lines if ln.strip()), default=0)
    return split_at(
        lines=lines, starts=lambda line, _current: len(line) - len(line.lstrip()) == base
    )


def strip_return(grammar: str) -> str:
    """A function synopsis without the return type written after its closing parenthesis."""
    return (
        normalize(RETURN_TAIL.sub(")", grammar))
        if grammar.rstrip().endswith(tuple("abcdefghijklmnopqrstuvwxyz]"))
        else grammar
    )


def apply_block(
    *, build: PageBuild, page: PgPage, block: SynopsisBlock, disposition: Disposition
) -> PageBuild:
    """Catalogue one narrative block as its reviewed disposition says."""
    family: str = disposition.family or page.family
    kind: str = disposition.disposition
    build.decisions.add(DECISION_BLOCKS)
    if kind == DISPOSITION_FORM:
        grammars: list[str] = (
            [normalize(block.text)] if disposition.whole else productions_of(block.text)
        )
        labels: tuple[str, ...] = disposition.labels or ("",) * len(grammars)
        if len(labels) != len(grammars):
            build.problems.append(f"{page.id} {block.sha256}: {len(grammars)} forms, labels differ")
            return build
        groups: list[FormGroup] = [
            FormGroup(disposition.kind or KIND_SPECIAL, label, (g,), block.heading)
            for g, label in zip(grammars, labels, strict=True)
        ]
        if page.is_command:
            return add_extra(build=build, family=family, groups=groups, label=disposition.label)
        return add_groups(build=build, family=family, groups=groups)
    if kind == DISPOSITION_CALLS:
        _, definitions, _ = read_synopsis(text=block.text, statement_leads=frozenset())
        calls: list[str] = [strip_return(g) for g in productions_of(statements_text(block))]
        signatures: list[Signature] = [
            Signature(block.heading, g, (), True, sha256(g)) for g in calls
        ]
        groups, decisions = call_groups(signatures)
        build = add_groups(build=build, family=family, groups=groups)
        build.decisions.update(decisions)
        return add_definitions(build=build, definitions=definitions)
    if kind == DISPOSITION_DEFINITION:
        parts: list[str] = productions_of(block.text)
        for name in disposition.names:
            definition: Definition = Definition(
                name=name,
                alternatives=tuple(parts) if disposition.listed else (normalize(" ".join(parts)),),
                listed=disposition.listed,
            )
            build = add_definitions(build=build, definitions=[definition])
    elif kind == DISPOSITION_RESTATED:
        build.restates.extend(disposition.forms)
    return build


def add_extra(*, build: PageBuild, family: str, groups: list[FormGroup], label: str) -> PageBuild:
    """Forms a command page documents beside its synopsis, each named `<page id>.<label>`."""
    if not label:
        build.problems.append(f"{groups[0].heading[0]}: a form beside the synopsis needs a label")
        return build
    suffixes: list[str] = statement_labels([g.grammars[0] for g in groups])
    for group, suffix in zip(groups, suffixes, strict=True):
        name: str = "-".join(p for p in (label, suffix) if p)
        build.extra.append((family, FormGroup(group.kind, name, group.grammars, group.heading)))
    return build


def statements_text(block: SynopsisBlock) -> str:
    """A block's text before its first `<phrase>` line."""
    return block.text.split("<phrase>", 1)[0]


def add_definitions(*, build: PageBuild, definitions: list[Definition]) -> PageBuild:
    """Add sub-grammar definitions, numbering a name the page defines twice."""
    taken: set[str] = {name for name, _ in build.definitions}
    for definition in definitions:
        key: str = definition.name
        number: int = 2
        while key in taken:
            key = f"{definition.name} #{number}"
            number += 1
        taken.add(key)
        build.definitions.append((key, definition_grammar(definition)))
    if definitions:
        build.decisions.add(DECISION_DEFINITIONS)
    return build


def command_build(*, page: PgPage, hand: HandInputs) -> PageBuild:
    """A command page: its synopsis statements and actions, then its other blocks."""
    build: PageBuild = PageBuild()
    leads: frozenset[str] = frozenset({page.title.split()[0]}) | EXTRA_LEADS.get(
        page.title.split()[0], frozenset()
    )
    statements: list[str] = []
    definitions: list[Definition] = []
    for block in page.blocks:
        if not block.in_synopsis_div:
            continue
        found, defs, _ = read_synopsis(text=block.text, statement_leads=leads)
        statements += [statement.removesuffix(";").rstrip() for statement in found]
        definitions += defs
    groups, decisions = statement_groups(
        statements=statements, definitions=definitions, heading=(page.title, SYNOPSIS_HEADING)
    )
    build = add_groups(build=build, family=page.family, groups=groups)
    build.decisions.update(decisions)
    if len(statements) > 1:
        build.decisions.add(DECISION_STATEMENTS)
    build = add_definitions(build=build, definitions=definitions)
    return narrative_blocks(build=build, page=page, hand=hand)


def narrative_keys(page: PgPage) -> set[tuple[str, str]]:
    """The (page id, block hash) keys of a page's blocks outside a command synopsis."""
    return {(page.id, block.sha256) for block in page.blocks if not block.in_synopsis_div}


def narrative_blocks(*, build: PageBuild, page: PgPage, hand: HandInputs) -> PageBuild:
    """Apply the reviewed disposition of every block outside a command synopsis."""
    for block in page.blocks:
        if block.in_synopsis_div:
            continue
        disposition: Disposition | None = hand.blocks.get((page.id, block.sha256))
        if disposition is None:
            first: str = normalize(block.text.splitlines()[0] if block.text.strip() else "")
            build.problems.append(f"{page.id} {block.sha256} undispositioned: {first[:80]}")
            continue
        build = apply_block(build=build, page=page, block=block, disposition=disposition)
    return build


def chapter_build(*, page: PgPage, hand: HandInputs, text: str) -> PageBuild:
    """A chapter page: its blocks, its signatures and, for type tables, its data types."""
    build: PageBuild = narrative_blocks(build=PageBuild(), page=page, hand=hand)
    kept: list[Signature] = []
    for signature in page.signatures:
        anchor: str = " / ".join(signature.heading[1:])
        if not any(
            source == page.id and anchor.startswith(prefix) for source, prefix in hand.sections
        ):
            kept.append(signature)
    functions, operators, decisions = signature_groups(kept)
    build = add_groups(build=build, family=page.family, groups=functions)
    build = add_groups(build=build, family="expressions", groups=operators)
    build.decisions.update(decisions)
    return add_groups(build=build, family="expressions", groups=type_groups(page=page, text=text))


def type_groups(*, page: PgPage, text: str) -> list[FormGroup]:
    """One form per row of the page's data type tables, its aliases as alternatives."""
    groups: list[FormGroup] = []
    for table_id, anchor, has_aliases in TYPE_TABLES.get(page.id, ()):
        table: re.Match[str] | None = re.search(TABLE.format(id=table_id), text, re.S)
        body: re.Match[str] | None = TBODY.search(table.group(1)) if table else None
        if body is None:
            raise CatalogueError(f"{page.id}: type table {table_id} not found")
        for name, aliases in TYPE_ROW.findall(body.group(1)):
            names: list[str] = [name, *(ALIAS_SEPARATOR.split(aliases) if has_aliases else [])]
            spellings: list[str] = [
                normalize(synopsis_text(re.sub(r"</?type>", "", n))) for n in names if plain(n)
            ]
            spellings = [s.replace("(<", "( <").replace(">)", "> )") for s in spellings]
            grammar: str = (
                spellings[0] if len(spellings) == 1 else "{ " + " | ".join(spellings) + " }"
            )
            label: str = slug(re.sub(r"<[^<>]*>", " ", required_without_options(spellings[0])))
            groups.append(FormGroup(KIND_SPECIAL, label, (grammar,), (page.title, anchor)))
    return groups


def hand_build(*, build: PageBuild, page: PgPage, forms: list[HandForm]) -> PageBuild:
    """Add a page's hand forms, refusing any read from another version of the page."""
    for form in forms:
        if form.page_sha256 != page.sha256:
            raise CatalogueError(f"{form.source} '{form.anchor}': page changed since read by hand")
        heading: tuple[str, ...] = (page.title, *form.anchor.split(" / "))
        group: FormGroup = FormGroup(form.kind, form.label, form.grammars, heading)
        if page.is_command:
            build = add_extra(
                build=build, family=form.family or page.family, groups=[group], label=form.label
            )
        else:
            build = add_groups(build=build, family=form.family or page.family, groups=[group])
        build.decisions.add(DECISION_HAND.format(basis=form.basis))
    return build


def label_groups(groups: list[FormGroup]) -> list[FormGroup]:
    """Give unlabelled groups a distinguishing label among the page's unlabelled groups."""
    unlabelled: list[int] = [i for i, g in enumerate(groups) if not g.label]
    if len(groups) < CLASH_SIZE or not unlabelled:
        return groups
    labels: list[str] = statement_labels([groups[i].grammars[0] for i in unlabelled])
    out: list[FormGroup] = list(groups)
    for index, label in zip(unlabelled, labels, strict=True):
        group: FormGroup = groups[index]
        out[index] = FormGroup(group.kind, label or "form", group.grammars, group.heading)
    return out


def source_row(*, page: PgPage, status: str, decisions: set[str], family: str) -> Row:
    """One sources.toml record."""
    syntax: list[str] = [b.text for b in page.blocks] + [s.grammar for s in page.signatures]
    return {
        "id": f"doc.{page.id}",
        "url": DOCS_URL.format(id=page.id),
        "title": page.title,
        "family": family,
        "retrieved_utc": "",
        "page_sha256": page.sha256,
        "syntax_sha256": sha256("\n\n".join(syntax)),
        "syntax_blocks": len(page.blocks) + len(page.signatures),
        "status": status,
        "decisions": sorted(decisions),
    }


def page_status(build: PageBuild) -> str:
    """A page's sources.toml status: forms, restated, or no syntax block."""
    if build.groups or build.extra:
        return STATUS_FORMS
    return STATUS_RESTATED if build.restates else STATUS_NO_SYNTAX


def page_forms(
    *, build: PageBuild, page: PgPage, existing: Existing, used_ids: frozenset[str]
) -> tuple[list[tuple[str, list[Row]]], frozenset[str]]:
    """A page's emitted forms as (family, rows) in order, and the ids now used."""
    sid: str = f"doc.{page.id}"
    names: tuple[str, ...] = tuple(name for name, _ in build.definitions)
    emitted: list[tuple[str, list[Row]]] = []
    for family, groups in build.groups.items():
        base: str = f"{ID_PREFIX}.{family}.{page_slug(page)}"
        if not page.is_command and len(groups) == 1 and groups[0].label:
            base = f"{base}.{slug(groups[0].label)}"
        context: PageContext = PageContext(base, family, sid, names)
        forms, used_ids = emit_forms(
            context=context, groups=label_groups(groups), existing=existing, used_ids=used_ids
        )
        emitted.append((family, forms))
    for family, group in build.extra:
        context = PageContext(
            f"{ID_PREFIX}.{family}.{page_slug(page)}.{group.label}", family, sid, names
        )
        forms, used_ids = emit_forms(
            context=context, groups=[group], existing=existing, used_ids=used_ids
        )
        emitted.append((family, forms))
    return emitted, used_ids


def definition_rows(*, build: PageBuild, sid: str) -> list[Row]:
    """A page's definitions as catalogue rows."""
    return [
        {"id": f"{sid}::{key}", "source": sid, "name": key, "grammar": grammar}
        for key, grammar in build.definitions
    ]


def with_also_sources(*, form: Row, sources: set[str]) -> Row:
    """A form row with the pages restating it listed after its anchor."""
    earlier: object = form.get("also_sources", [])
    also: set[str] = {str(s) for s in earlier} if isinstance(earlier, list) else set()
    also |= sources
    also.discard(str(form["source"]))
    out: Row = {}
    for key, value in form.items():
        out[key] = value
        if key == ANCHOR_FIELD:
            out["also_sources"] = sorted(also)
    return out


def build_catalogue(*, tree: Path, out: Path, retrieved_utc: str) -> PgCatalogue:
    """Sources, forms by family and definitions by family for every pinned page."""
    hand: HandInputs = load_hand(out / "hand.toml")
    existing: Existing = load_existing(out)
    used_ids: frozenset[str] = frozenset(existing.ids)
    catalogue: Catalogue = Catalogue([], {f: [] for f in FAMILIES}, {f: [] for f in FAMILIES})
    problems: list[str] = []
    restated: dict[str, set[str]] = {}
    used: set[tuple[str, str]] = set()
    pages, texts = load_pages(tree)
    for page in pages:
        build: PageBuild = (
            command_build(page=page, hand=hand)
            if page.is_command
            else chapter_build(page=page, hand=hand, text=texts[page.id])
        )
        used |= narrative_keys(page)
        build = hand_build(build=build, page=page, forms=hand.forms.pop(f"doc.{page.id}", []))
        problems += build.problems
        sid: str = f"doc.{page.id}"
        for form_id in build.restates:
            restated.setdefault(form_id, set()).add(sid)
        if build.restates:
            build.decisions.add(DECISION_RESTATED)
        catalogue.sources.append(
            source_row(
                page=page, status=page_status(build), decisions=build.decisions, family=page.family
            )
        )
        emitted, used_ids = page_forms(build=build, page=page, existing=existing, used_ids=used_ids)
        for family, forms in emitted:
            catalogue.forms[family].extend(forms)
        if build.groups or build.extra:
            owner: str = next(iter(build.groups)) if build.groups else build.extra[0][0]
            catalogue.definitions[owner].extend(definition_rows(build=build, sid=sid))
    for row in catalogue.sources:
        row["retrieved_utc"] = retrieved_utc
    problems += [f"hand forms for unknown page {source}" for source in hand.forms]
    problems += [f"disposition for no block: {key}" for key in hand.blocks if key not in used]
    by_id: dict[str, Row] = {}
    for rows in catalogue.forms.values():
        by_id |= {str(f["id"]): f for f in rows}
    for form_id, sources in sorted(restated.items()):
        form: Row | None = by_id.get(form_id)
        if form is None:
            problems.append(f"restated form {form_id} is not catalogued")
            continue
        rebuilt: Row = with_also_sources(form=form, sources=sources)
        form.clear()
        form.update(rebuilt)
    for absent in absent_forms(existing=existing, emitted=set(by_id)):
        catalogue.forms[str(absent["family"])].append(absent)
    return PgCatalogue(catalogue=catalogue, problems=problems)
