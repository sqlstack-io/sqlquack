"""Build the DuckDB catalogue: sources, forms by family and definitions from the pinned pages."""

from __future__ import annotations

import json
import re
from pathlib import Path, PurePosixPath

from scripts.grammar_catalogue._helpers.existing import absent_forms, load_existing
from scripts.grammar_catalogue._helpers.forms import emit_forms
from scripts.grammar_catalogue._helpers.grammar import sha256, slug
from scripts.grammar_catalogue.constants import GRAMMAR_JOINER
from scripts.grammar_catalogue.models import Catalogue, Existing, FormGroup, PageContext
from scripts.syntax_catalogue_duckdb._helpers.groups import (
    diagram_groups,
    is_call,
    operator_groups,
    signature_groups,
    type_groups,
)
from scripts.syntax_catalogue_duckdb._helpers.hand import load_hand
from scripts.syntax_catalogue_duckdb._helpers.pages import load_pages
from scripts.syntax_catalogue_duckdb._helpers.railroad import render
from scripts.syntax_catalogue_duckdb.constants import (
    ALIAS_FOR,
    ALIAS_ROW,
    ANCHOR_KEY,
    BARE_NAME,
    CALL_SIGNATURE,
    CODE_SPAN,
    DECISION_DIAGRAMS,
    DECISION_HAND,
    DECISION_OPERATOR_TABLES,
    DECISION_OPERATORS,
    DECISION_RESTATED,
    DECISION_SIGNATURES,
    DECISION_SPLIT,
    DECISION_TYPES,
    DEFINITION_ADDITIONS,
    DEFINITION_RENAMES,
    DIAGRAM_FAMILY_OVERRIDES,
    EXPRESSIONS,
    FAMILIES,
    FORM_PREFIX,
    FUNCTIONS,
    HAND_FILE,
    OPERANDS,
    OPERATOR_PAGES,
    PAGE_BASES,
    PAGE_FAMILIES,
    RAILROAD_FILE,
    SIGNATURE_DEPTH,
    SIGNATURE_DIRS,
    SINGLE_QUOTE,
    STATEMENTS_FOLDER,
    STATUS_FORMS,
    STATUS_NO_SYNTAX,
    STATUS_OUT_OF_SCOPE,
    STATUS_RESTATED,
    TABLE_FUNCTION_DEPTH,
    TABLE_FUNCTION_PAGE,
    TABLE_SIGNATURE,
    TREE_DEFINITION,
    TYPE_BASE,
    TYPE_TABLE_PAGE,
)
from scripts.syntax_catalogue_duckdb.models import (
    DuckDbCatalogue,
    DuckDbPage,
    HandInputs,
    HandPage,
    Located,
    PageBuild,
)
from scripts.syntax_catalogue_duckdb.types import Row, Tree


def page_family(path: str) -> str:
    """The family PAGE_FAMILIES gives the page itself, else its nearest directory."""
    for candidate in (path, *(str(parent) for parent in PurePosixPath(path).parents)):
        if candidate in PAGE_FAMILIES:
            return PAGE_FAMILIES[candidate]
    return EXPRESSIONS


def page_bases(pages: list[DuckDbPage]) -> dict[str, str]:
    """Each page's id base: its file name, qualified by its directory where names collide."""
    names: dict[tuple[str, str], list[DuckDbPage]] = {}
    for page in pages:
        if page.path in PAGE_BASES:
            continue
        name: str = slug(PurePosixPath(page.path).stem)
        names.setdefault((page_family(page.path), name), []).append(page)
    bases: dict[str, str] = {
        page.id: PAGE_BASES[page.path] for page in pages if page.path in PAGE_BASES
    }
    for (_, name), same in names.items():
        for page in same:
            folder: str = PurePosixPath(page.path).parent.name
            qualified: bool = len(same) > 1 and folder != STATEMENTS_FOLDER
            bases[page.id] = f"{slug(folder)}-{name}" if qualified else name
    return bases


def referenced(tree: Tree) -> set[str]:
    """The definition names a tree refers to."""
    if tree["kind"] == TREE_DEFINITION:
        return {tree["name"]}
    found: set[str] = set()
    for item in tree.get("items", []):
        found |= referenced(item)
    return found


def diagram_owners(*, pages: list[DuckDbPage], trees: dict[str, Tree]) -> dict[str, str]:
    """For each diagram grammar, the page that owns its form: fewest diagrams, then path."""
    owners: dict[str, tuple[int, str, str]] = {}
    for page in pages:
        for located in page.diagrams:
            tree: Tree | None = trees.get(page.railroad, {}).get("diagrams", {}).get(located.text)
            if tree is None:
                continue
            for group in diagram_groups(located=located, tree=tree, family=EXPRESSIONS):
                rank: tuple[int, str, str] = (len(page.diagrams), page.path, page.id)
                key: str = group.grammars[0]
                if key not in owners or rank < owners[key]:
                    owners[key] = rank
    return {grammar: rank[2] for grammar, rank in owners.items()}


def add(*, build: PageBuild, family: str, groups: list[FormGroup]) -> PageBuild:
    """Append groups under a family."""
    if groups:
        build.groups.setdefault(family, []).extend(groups)
    return build


def diagram_build(
    *, page: DuckDbPage, trees: dict[str, Tree], owners: dict[str, str], build: PageBuild
) -> tuple[PageBuild, dict[str, set[str]]]:
    """Add the page's own diagram forms and definitions; also return the forms it restates."""
    family: str = page_family(page.path)
    family = DIAGRAM_FAMILY_OVERRIDES.get(family, family)
    script: Tree = trees.get(page.railroad, {})
    restates: dict[str, set[str]] = {}
    names: set[str] = set()
    for located in page.diagrams:
        tree: Tree | None = script.get("diagrams", {}).get(located.text)
        if tree is None:
            build.problems.append(f"{page.id}: diagram {located.text} not in {page.railroad}")
            continue
        groups: list[FormGroup] = diagram_groups(located=located, tree=tree, family=family)
        build.syntax.extend(group.grammars[0] for group in groups)
        if len(groups) > 1:
            build.decisions.add(DECISION_SPLIT)
        own: list[FormGroup] = [g for g in groups if owners[g.grammars[0]] == page.id]
        for group in groups:
            if owners[group.grammars[0]] != page.id:
                restates.setdefault(owners[group.grammars[0]], set()).add(group.grammars[0])
        if own:
            names |= referenced(tree)
        build = add(build=build, family=family, groups=own)
    if page.diagrams:
        build.decisions.add(DECISION_DIAGRAMS)
    renames: dict[str, str] = {
        old: new for (owner, old), (new, _) in DEFINITION_RENAMES.items() if owner == page.railroad
    }
    build.problems.extend(
        f"{page.id}: definition {old} to rename is not in {page.railroad}"
        for old in renames
        if old not in script.get("definitions", {})
    )
    definitions: dict[str, Tree] = {
        renames.get(key, key): tree for key, tree in script.get("definitions", {}).items()
    }
    pending: list[str] = sorted(renames.get(name, name) for name in names)
    while pending:
        name: str = pending.pop()
        if name not in definitions or any(key == name for key, _ in build.definitions):
            continue
        body: str = render(definitions[name])
        addition: tuple[str, str, str] | None = DEFINITION_ADDITIONS.get((page.railroad, name))
        if addition is not None:
            if addition[0] not in body:
                build.problems.append(f"{page.id}: {name} no longer spells {addition[0]}")
            body = body.replace(addition[0], addition[1])
        build.definitions.append((name, f"<{name}> ::= {body}"))
        pending += sorted(renames.get(item, item) for item in referenced(definitions[name]))
    if restates:
        build.decisions.add(DECISION_RESTATED)
    return build, restates


def page_signatures(
    *, page: DuckDbPage, elsewhere: frozenset[str] = frozenset()
) -> tuple[Located, ...]:
    """The page's signature headings; on the metadata page, its table-function sections."""
    if page.path == TABLE_FUNCTION_PAGE:
        return tuple(
            Located(s.heading, f"{s.text}()", s.depth)
            for s in page.signatures
            if s.depth == TABLE_FUNCTION_DEPTH
        )
    found: list[Located] = []
    for signature in page.signatures:
        if signature.depth == SIGNATURE_DEPTH:
            found += [signature, *aliases(page=page, signature=signature)]
    known: set[str] = {s.text for s in found} | elsewhere
    return tuple(found + table_signatures(page=page, known=known))


def table_signatures(*, page: DuckDbPage, known: set[str]) -> list[Located]:
    """Call signatures in the first cell of the page's tables that no heading already gives."""
    found: list[Located] = []
    seen: set[str] = set(known)
    for row in page.rows:
        match: re.Match[str] | None = TABLE_SIGNATURE.match(row.text)
        if (
            match is None
            or not CALL_SIGNATURE.match(match.group(1))
            or match.group(1) in seen
            or SINGLE_QUOTE in match.group(1)
        ):
            continue
        seen.add(match.group(1))
        alias: re.Match[str] | None = ALIAS_FOR.search(row.text)
        found.append(Located(row.heading, match.group(1), 0, alias.group(1) if alias else ""))
    return found


def aliases(*, page: DuckDbPage, signature: Located) -> list[Located]:
    """The call aliases the signature's Alias row names, as signatures owned by its function."""
    match: re.Match[str] | None = CALL_SIGNATURE.match(signature.text)
    if match is None or not is_call(signature.text):
        return []
    heading: tuple[str, ...] = (*signature.heading, f"`{signature.text}`")
    found: list[Located] = []
    for row in page.rows:
        alias_row: re.Match[str] | None = ALIAS_ROW.match(row.text)
        if row.heading != heading or alias_row is None:
            continue
        for alias in CODE_SPAN.findall(alias_row.group(1)):
            text: str = f"{alias}({match.group(2)})" if BARE_NAME.match(alias) else alias
            if CALL_SIGNATURE.match(text):
                found.append(Located(signature.heading, text, signature.depth, match.group(1)))
    return found


def operator_tokens(grammar: str) -> list[str]:
    """The operator of an operator-table grammar: its tokens other than the `<x>` operands."""
    return [" ".join(t for t in grammar.split() if t not in OPERANDS)]


def spelled_grammars(build: PageBuild) -> str:
    """Every grammar line of the build's groups, each padded with spaces, joined by a space."""
    padded: list[str] = []
    for groups in build.groups.values():
        for group in groups:
            padded.extend(f" {grammar} " for grammar in group.grammars)
    return " ".join(padded)


def page_build(
    *,
    page: DuckDbPage,
    trees: dict[str, Tree],
    owners: dict[str, str],
    hand: HandInputs,
    elsewhere: frozenset[str],
) -> tuple[PageBuild, dict[str, set[str]]]:
    """Everything one page contributes, and the forms of other pages it restates."""
    build: PageBuild = PageBuild()
    build, restates = diagram_build(page=page, trees=trees, owners=owners, build=build)
    if any(page.path.startswith(folder) for folder in SIGNATURE_DIRS):
        kept: tuple[Located, ...] = tuple(
            signature
            for signature in page_signatures(page=page, elsewhere=elsewhere)
            if (page.id, signature.text) not in hand.signatures
        )
        calls, specials = signature_groups(kept)
        build.syntax.extend(
            signature.text for signature in page_signatures(page=page, elsewhere=elsewhere)
        )
        build = add(build=build, family=FUNCTIONS, groups=calls)
        build = add(build=build, family=EXPRESSIONS, groups=specials)
        if calls:
            build.decisions.add(DECISION_SIGNATURES)
        if specials:
            build.decisions.add(DECISION_OPERATORS)
    if page.path in OPERATOR_PAGES or any(page.path.startswith(d) for d in SIGNATURE_DIRS):
        spelled: str = spelled_grammars(build)
        operators: list[FormGroup] = []
        for group in operator_groups(page.rows):
            if not any(f" {op} " in spelled for op in operator_tokens(group.grammars[0])):
                operators.append(group)
        if operators:
            build.syntax.extend(GRAMMAR_JOINER.join(group.grammars) for group in operators)
            build = add(build=build, family=EXPRESSIONS, groups=operators)
            build.decisions.add(DECISION_OPERATOR_TABLES)
    if page.path == TYPE_TABLE_PAGE:
        types: list[FormGroup] = type_groups(page.rows)
        build.syntax.extend(GRAMMAR_JOINER.join(group.grammars) for group in types)
        build = add(build=build, family=EXPRESSIONS, groups=types)
        build.decisions.add(DECISION_TYPES)
    for family, form in hand.forms.get(page.id, []):
        if form.page_sha256 != page.page_sha256:
            build.problems.append(f"{page.id}: hand form {form.label} reviewed a different page")
        group: FormGroup = FormGroup(form.kind, form.label, form.grammars, ("", form.anchor))
        build.syntax.extend(form.grammars)
        build = add(build=build, family=family, groups=[group])
        build.decisions.add(DECISION_HAND.format(basis=form.basis))
    return build, restates


def page_status(
    *, page: DuckDbPage, build: PageBuild, restates: bool, hand: HandInputs
) -> tuple[str, PageBuild]:
    """forms, restated, or the reviewed status of a page with neither; a problem if unreviewed."""
    disposition: HandPage | None = hand.pages.get(page.id)
    if build.groups:
        if disposition is not None:
            build.problems.append(f"{page.id}: has forms and a page disposition")
        return STATUS_FORMS, build
    if restates:
        return STATUS_RESTATED, build
    if disposition is None:
        build.problems.append(f"{page.id}: no diagram, signature or hand form and no disposition")
        return STATUS_NO_SYNTAX, build
    if disposition.page_sha256 != page.page_sha256:
        build.problems.append(f"{page.id}: disposition reviewed a different page")
    if disposition.status not in (STATUS_NO_SYNTAX, STATUS_OUT_OF_SCOPE):
        build.problems.append(f"{page.id}: unknown disposition status {disposition.status}")
    return disposition.status, build


def source_row(*, page: DuckDbPage, status: str, build: PageBuild, retrieved_utc: str) -> Row:
    """One sources.toml record."""
    return {
        "id": page.id,
        "url": page.url,
        "title": page.title,
        "family": "" if status == STATUS_OUT_OF_SCOPE else page_family(page.path),
        "retrieved_utc": retrieved_utc,
        "page_sha256": page.page_sha256,
        "syntax_sha256": sha256("\n\n".join(build.syntax)),
        "syntax_blocks": len(build.syntax),
        "status": status,
        "decisions": sorted(build.decisions),
    }


def emit_page(
    *, page: DuckDbPage, base: str, build: PageBuild, existing: Existing, used_ids: frozenset[str]
) -> tuple[list[tuple[str, list[Row]]], frozenset[str]]:
    """A page's forms as (family, rows), and the ids now used."""
    names: tuple[str, ...] = tuple(name for name, _ in build.definitions)
    emitted: list[tuple[str, list[Row]]] = []
    for family, groups in build.groups.items():
        page_base: str = TYPE_BASE if page.path == TYPE_TABLE_PAGE else base
        context_base: str = f"{FORM_PREFIX}.{family}.{page_base}"
        if len(groups) == 1 and groups[0].label:
            context_base = f"{context_base}.{slug(groups[0].label)}"
        context: PageContext = PageContext(context_base, family, page.id, names)
        forms, used_ids = emit_forms(
            context=context, groups=groups, existing=existing, used_ids=used_ids
        )
        emitted.append((family, forms))
    return emitted, used_ids


def with_also_sources(*, form: Row, sources: set[str]) -> Row:
    """The form with the restating pages listed after its anchor."""
    rebuilt: Row = {}
    for key, value in form.items():
        rebuilt[key] = value
        if key == ANCHOR_KEY:
            rebuilt["also_sources"] = sorted(sources - {str(form["source"])})
    return rebuilt


def signature_headings(pages: list[DuckDbPage]) -> frozenset[str]:
    """The signature heading texts of every page, for tables that restate them."""
    found: set[str] = set()
    for page in pages:
        found |= {s.text for s in page.signatures if s.depth == SIGNATURE_DEPTH}
    return frozenset(found)


def build_catalogue(*, cache: Path, out: Path, retrieved_utc: str) -> DuckDbCatalogue:
    """Sources, forms by family and definitions by family for every pinned page."""
    trees: dict[str, Tree] = json.loads((cache / RAILROAD_FILE).read_text())
    hand: HandInputs = load_hand(out / HAND_FILE)
    existing: Existing = load_existing(out)
    used_ids: frozenset[str] = frozenset(existing.ids)
    pages: list[DuckDbPage] = load_pages(cache)
    owners: dict[str, str] = diagram_owners(pages=pages, trees=trees)
    bases: dict[str, str] = page_bases(pages)
    catalogue: Catalogue = Catalogue([], {f: [] for f in FAMILIES}, {f: [] for f in FAMILIES})
    problems: list[str] = []
    restated: dict[tuple[str, str], set[str]] = {}
    known: set[str] = {page.id for page in pages}
    headings: frozenset[str] = signature_headings(pages)
    for page in pages:
        build, restates = page_build(
            page=page, trees=trees, owners=owners, hand=hand, elsewhere=headings
        )
        for owner, grammars in restates.items():
            for grammar in grammars:
                restated.setdefault((owner, sha256(grammar)), set()).add(page.id)
        status, build = page_status(page=page, build=build, restates=bool(restates), hand=hand)
        problems += build.problems
        catalogue.sources.append(
            source_row(page=page, status=status, build=build, retrieved_utc=retrieved_utc)
        )
        emitted, used_ids = emit_page(
            page=page, base=bases[page.id], build=build, existing=existing, used_ids=used_ids
        )
        for family, forms in emitted:
            catalogue.forms[family].extend(forms)
        if build.groups:
            owner_family: str = next(iter(build.groups))
            catalogue.definitions[owner_family].extend(
                {"id": f"{page.id}::{name}", "source": page.id, "name": name, "grammar": grammar}
                for name, grammar in build.definitions
            )
    problems += [f"hand forms for unknown page {s}" for s in hand.forms if s not in known]
    problems += [f"disposition for unknown page {s}" for s in hand.pages if s not in known]
    by_key: dict[tuple[str, str], Row] = {}
    for rows in catalogue.forms.values():
        by_key |= {(str(f["source"]), str(f["grammar_sha256"])): f for f in rows}
    for key, sources in sorted(restated.items()):
        form: Row | None = by_key.get(key)
        if form is None:
            problems.append(f"restated diagram of {key[0]} is not a catalogued form")
            continue
        rebuilt: Row = with_also_sources(form=form, sources=sources)
        form.clear()
        form.update(rebuilt)
    emitted_ids: set[str] = set()
    for rows in catalogue.forms.values():
        emitted_ids |= {str(f["id"]) for f in rows}
    for absent in absent_forms(existing=existing, emitted=emitted_ids):
        catalogue.forms[str(absent["family"])].append(absent)
    return DuckDbCatalogue(catalogue=catalogue, problems=problems)
