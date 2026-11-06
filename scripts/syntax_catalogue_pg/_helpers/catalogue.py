"""Build the PostgreSQL catalogue's sources, forms and definitions from the pinned tree."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

from scripts.syntax_catalogue._helpers.catalogue import absent_forms, load_existing
from scripts.syntax_catalogue._helpers.forms import emit_forms
from scripts.syntax_catalogue._helpers.grammar import normalize, sha256, slug
from scripts.syntax_catalogue.exceptions import CatalogueError
from scripts.syntax_catalogue.models import Catalogue, Existing, FormGroup, PageContext
from scripts.syntax_catalogue_pg._helpers.groups import (
    call_groups,
    required_without_options,
    signature_groups,
    statement_groups,
    statement_labels,
)
from scripts.syntax_catalogue_pg._helpers.hand import Disposition, HandForm, HandInputs, load_hand
from scripts.syntax_catalogue_pg._helpers.pages import load_pages
from scripts.syntax_catalogue_pg._helpers.sgml import plain, synopsis_text
from scripts.syntax_catalogue_pg._helpers.synopsis import (
    definition_grammar,
    read_synopsis,
    split_at,
)
from scripts.syntax_catalogue_pg.constants import (
    DECISION_BLOCKS,
    DECISION_DEFINITIONS,
    DECISION_HAND,
    DECISION_RESTATED,
    DECISION_STATEMENTS,
    DOCS_URL,
    FAMILIES,
    ID_PREFIX,
    KIND_SPECIAL,
    STATUS_FORMS,
    STATUS_NO_SYNTAX,
    STATUS_RESTATED,
    SYNOPSIS_HEADING,
    TYPE_TABLES,
)
from scripts.syntax_catalogue_pg.models import Definition, PgPage, Signature, SynopsisBlock
from scripts.syntax_catalogue_pg.types import Row

RETURN_TAIL: re.Pattern[str] = re.compile(r"\)\s*(?:returns\s+)?[A-Za-z_][\w \[\]]*$")
TYPE_ROW: re.Pattern[str] = re.compile(r"<row>\s*<entry>(.*?)</entry>\s*<entry>(.*?)</entry>", re.S)
TABLE: str = r'<table id="{id}">(.*?)</table>'
TBODY: re.Pattern[str] = re.compile(r"<tbody>(.*?)</tbody>", re.S)


@dataclass
class PageBuild:
    """What one page contributes: groups by family, definitions, decisions and restatements."""

    groups: dict[str, list[FormGroup]] = field(default_factory=dict)
    definitions: list[tuple[str, str]] = field(default_factory=list)
    decisions: set[str] = field(default_factory=set)
    restates: list[str] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)
    extra: list[tuple[str, FormGroup]] = field(default_factory=list)

    def add(self, family: str, groups: list[FormGroup]) -> None:
        """Add form groups under a family."""
        if groups:
            self.groups.setdefault(family, []).extend(groups)


def page_slug(page: PgPage) -> str:
    """The id segment of a page: a command's title, or a chapter section id."""
    if page.is_command:
        return slug(page.title)
    return page.id.removeprefix("functions-") if page.family == "functions" else page.id


def productions_of(text: str) -> list[str]:
    """A narrative block's productions: each unindented line at bracket depth zero starts one."""
    lines: list[str] = [line.rstrip() for line in text.expandtabs().splitlines()]
    base: int = min((len(ln) - len(ln.lstrip()) for ln in lines if ln.strip()), default=0)
    return split_at(lines, starts=lambda line, _current: len(line) - len(line.lstrip()) == base)


def strip_return(grammar: str) -> str:
    """A function synopsis without the return type written after its closing parenthesis."""
    return (
        normalize(RETURN_TAIL.sub(")", grammar))
        if grammar.rstrip().endswith(tuple("abcdefghijklmnopqrstuvwxyz]"))
        else grammar
    )


def apply_block(
    *, build: PageBuild, page: PgPage, block: SynopsisBlock, disposition: Disposition
) -> None:
    """Catalogue one narrative block as its reviewed disposition says."""
    family: str = disposition.family or page.family
    kind: str = disposition.disposition
    build.decisions.add(DECISION_BLOCKS)
    if kind == "form":
        grammars: list[str] = (
            [normalize(block.text)] if disposition.whole else productions_of(block.text)
        )
        labels: tuple[str, ...] = disposition.labels or ("",) * len(grammars)
        if len(labels) != len(grammars):
            build.problems.append(f"{page.id} {block.sha256}: {len(grammars)} forms, labels differ")
            return
        groups: list[FormGroup] = [
            FormGroup(disposition.kind or KIND_SPECIAL, label, (g,), block.heading)
            for g, label in zip(grammars, labels, strict=True)
        ]
        if page.is_command:
            add_extra(build=build, family=family, groups=groups, label=disposition.label)
        else:
            build.add(family, groups)
    elif kind == "calls":
        _, definitions, _ = read_synopsis(text=block.text, statement_leads=frozenset())
        calls: list[str] = [strip_return(g) for g in productions_of(statements_text(block))]
        signatures: list[Signature] = [
            Signature(block.heading, g, (), True, sha256(g)) for g in calls
        ]
        groups, decisions = call_groups(signatures)
        build.add(family, groups)
        build.decisions |= decisions
        add_definitions(build, definitions)
    elif kind == "definition":
        parts: list[str] = productions_of(block.text)
        for name in disposition.names:
            definition: Definition = Definition(
                name=name,
                alternatives=tuple(parts) if disposition.listed else (normalize(" ".join(parts)),),
                listed=disposition.listed,
            )
            add_definitions(build, [definition])
    elif kind == "restated":
        build.restates.extend(disposition.forms)


def add_extra(*, build: PageBuild, family: str, groups: list[FormGroup], label: str) -> None:
    """Forms a command page documents beside its synopsis (older syntax still accepted), each
    named `<page id>.<label>` so the synopsis forms keep their ids."""
    if not label:
        build.problems.append(f"{groups[0].heading[0]}: a form beside the synopsis needs a label")
        return
    suffixes: list[str] = statement_labels([g.grammars[0] for g in groups])
    for group, suffix in zip(groups, suffixes, strict=True):
        name: str = "-".join(p for p in (label, suffix) if p)
        build.extra.append((family, FormGroup(group.kind, name, group.grammars, group.heading)))


def statements_text(block: SynopsisBlock) -> str:
    """A block's text before its first `<phrase>` line."""
    return block.text.split("<phrase>", 1)[0]


def add_definitions(build: PageBuild, definitions: list[Definition]) -> None:
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


def command_build(*, page: PgPage, hand: HandInputs) -> PageBuild:
    """A command page: its synopsis statements and actions, then its other blocks."""
    build: PageBuild = PageBuild()
    leads: frozenset[str] = frozenset({page.title.split()[0]}) | (
        frozenset({"TABLE", "WITH"}) if page.title.startswith("SELECT") else frozenset()
    )
    statements: list[str] = []
    definitions: list[Definition] = []
    for block in page.blocks:
        if not block.in_synopsis_div:
            continue
        found, defs, _ = read_synopsis(text=block.text, statement_leads=leads)
        statements += found
        definitions += defs
    groups, decisions = statement_groups(
        statements=statements, definitions=definitions, heading=(page.title, SYNOPSIS_HEADING)
    )
    build.add(page.family, groups)
    build.decisions |= decisions
    if len(statements) > 1:
        build.decisions.add(DECISION_STATEMENTS)
    add_definitions(build, definitions)
    narrative_blocks(build=build, page=page, hand=hand)
    return build


def narrative_blocks(*, build: PageBuild, page: PgPage, hand: HandInputs) -> None:
    """Apply the reviewed disposition of every block outside a command synopsis."""
    for block in page.blocks:
        if block.in_synopsis_div:
            continue
        disposition: Disposition | None = hand.blocks.get((page.id, block.sha256))
        hand.used.add((page.id, block.sha256))
        if disposition is None:
            first: str = normalize(block.text.splitlines()[0] if block.text.strip() else "")
            build.problems.append(f"{page.id} {block.sha256} undispositioned: {first[:80]}")
            continue
        apply_block(build=build, page=page, block=block, disposition=disposition)


def chapter_build(*, page: PgPage, hand: HandInputs, text: str) -> PageBuild:
    """A chapter page: its blocks, its signatures and, for type tables, its data types."""
    build: PageBuild = PageBuild()
    narrative_blocks(build=build, page=page, hand=hand)
    kept: list[Signature] = []
    for signature in page.signatures:
        anchor: str = " / ".join(signature.heading[1:])
        if not any(
            source == page.id and anchor.startswith(prefix) for source, prefix in hand.sections
        ):
            kept.append(signature)
    functions, operators, decisions = signature_groups(kept)
    build.add(page.family, functions)
    build.add("expressions", operators)
    build.decisions |= decisions
    build.add("expressions", type_groups(page=page, text=text))
    return build


def type_groups(*, page: PgPage, text: str) -> list[FormGroup]:
    """One form per row of the page's data type tables, its aliases as alternatives."""
    groups: list[FormGroup] = []
    for table_id, anchor, has_aliases in TYPE_TABLES.get(page.id, ()):
        table: re.Match[str] | None = re.search(TABLE.format(id=table_id), text, re.S)
        body: re.Match[str] | None = TBODY.search(table.group(1)) if table else None
        if body is None:
            raise CatalogueError(f"{page.id}: type table {table_id} not found")
        for name, aliases in TYPE_ROW.findall(body.group(1)):
            names: list[str] = [name, *(aliases.split(",") if has_aliases else [])]
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


def hand_build(*, build: PageBuild, page: PgPage, forms: list[HandForm]) -> None:
    """Add a page's hand forms, refusing any read from another version of the page."""
    for form in forms:
        if form.page_sha256 != page.sha256:
            raise CatalogueError(f"{form.source} '{form.anchor}': page changed since read by hand")
        heading: tuple[str, ...] = (page.title, *form.anchor.split(" / "))
        group: FormGroup = FormGroup(form.kind, form.label, form.grammars, heading)
        if page.is_command:
            add_extra(
                build=build, family=form.family or page.family, groups=[group], label=form.label
            )
        else:
            build.add(form.family or page.family, [group])
        build.decisions.add(DECISION_HAND.format(basis=form.basis))


def label_groups(groups: list[FormGroup]) -> list[FormGroup]:
    """Give unlabelled groups a distinguishing label among the page's unlabelled groups."""
    unlabelled: list[int] = [i for i, g in enumerate(groups) if not g.label]
    if len(groups) < 2 or not unlabelled:
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


@dataclass
class PgCatalogue:
    """The built catalogue and the problems that make it incomplete."""

    catalogue: Catalogue
    problems: list[str]


def build_catalogue(*, tree: Path, out: Path, retrieved_utc: str) -> PgCatalogue:
    """Sources, forms by family and definitions by family for every pinned page."""
    hand: HandInputs = load_hand(out / "hand.toml")
    existing: Existing = load_existing(out)
    used_ids: frozenset[str] = frozenset(existing.ids)
    catalogue: Catalogue = Catalogue([], {f: [] for f in FAMILIES}, {f: [] for f in FAMILIES})
    problems: list[str] = []
    restated: dict[str, set[str]] = {}
    texts: dict[str, str] = {}
    pages: list[PgPage] = load_pages(tree, texts=texts)
    for page in pages:
        build: PageBuild = (
            command_build(page=page, hand=hand)
            if page.is_command
            else chapter_build(page=page, hand=hand, text=texts[page.id])
        )
        hand_build(build=build, page=page, forms=hand.forms.pop(f"doc.{page.id}", []))
        problems += build.problems
        sid: str = f"doc.{page.id}"
        for form_id in build.restates:
            restated.setdefault(form_id, set()).add(sid)
        if build.restates:
            build.decisions.add(DECISION_RESTATED)
        status: str = (
            STATUS_FORMS
            if build.groups or build.extra
            else STATUS_RESTATED
            if build.restates
            else STATUS_NO_SYNTAX
        )
        catalogue.sources.append(
            source_row(page=page, status=status, decisions=build.decisions, family=page.family)
        )
        names: tuple[str, ...] = tuple(name for name, _ in build.definitions)
        for family, groups in build.groups.items():
            base: str = f"{ID_PREFIX}.{family}.{page_slug(page)}"
            if not page.is_command and len(groups) == 1 and groups[0].label:
                base = f"{base}.{slug(groups[0].label)}"
            context: PageContext = PageContext(base, family, sid, names)
            forms, used_ids = emit_forms(
                context=context, groups=label_groups(groups), existing=existing, used_ids=used_ids
            )
            catalogue.forms[family].extend(forms)
        for family, group in build.extra:
            context = PageContext(
                f"{ID_PREFIX}.{family}.{page_slug(page)}.{group.label}", family, sid, names
            )
            forms, used_ids = emit_forms(
                context=context, groups=[group], existing=existing, used_ids=used_ids
            )
            catalogue.forms[family].extend(forms)
        if build.groups or build.extra:
            family = next(iter(build.groups)) if build.groups else build.extra[0][0]
            catalogue.definitions[family].extend(
                {"id": f"{sid}::{key}", "source": sid, "name": key, "grammar": grammar}
                for key, grammar in build.definitions
            )
    for row in catalogue.sources:
        row["retrieved_utc"] = retrieved_utc
    problems += [f"hand forms for unknown page {source}" for source in hand.forms]
    problems += [f"disposition for no block: {key}" for key in hand.blocks if key not in hand.used]
    by_id: dict[str, Row] = {str(f["id"]): f for rows in catalogue.forms.values() for f in rows}
    for form_id, sources in sorted(restated.items()):
        form: Row | None = by_id.get(form_id)
        if form is None:
            problems.append(f"restated form {form_id} is not catalogued")
            continue
        earlier: object = form.get("also_sources", [])
        also: set[str] = {str(s) for s in earlier} if isinstance(earlier, list) else set()
        also |= sources
        also.discard(str(form["source"]))
        rows_order: list[tuple[str, object]] = list(form.items())
        form.clear()
        for key, value in rows_order:
            form[key] = value
            if key == "anchor":
                form["also_sources"] = sorted(also)
    emitted: set[str] = set(by_id)
    for form in absent_forms(existing=existing, emitted=emitted):
        catalogue.forms[str(form["family"])].append(form)
    return PgCatalogue(catalogue=catalogue, problems=problems)
