"""Build the SQLite catalogue: sources, and forms by family, from the pinned pages and hand.toml."""

from __future__ import annotations

from pathlib import Path, PurePosixPath
from typing import cast

from scripts.grammar_catalogue._helpers.existing import absent_forms, load_existing
from scripts.grammar_catalogue._helpers.forms import emit_forms
from scripts.grammar_catalogue._helpers.grammar import sha256, slug
from scripts.grammar_catalogue.constants import GRAMMAR_JOINER
from scripts.grammar_catalogue.models import Catalogue, Existing, FormGroup, PageContext
from scripts.syntax_catalogue_sqlite._helpers.hand import load_hand
from scripts.syntax_catalogue_sqlite._helpers.pages import load_pages
from scripts.syntax_catalogue_sqlite._helpers.signatures import (
    call_grammar,
    pragma_grammar,
    pragma_lines,
    pragma_sections,
    signatures,
)
from scripts.syntax_catalogue_sqlite.constants import (
    ANCHOR_KEY,
    DECISION_DIAGRAM,
    DECISION_HAND,
    DECISION_OVERRIDE,
    DECISION_PRAGMA_SCOPE,
    DECISION_PRAGMAS,
    DECISION_RESTATED,
    DECISION_SIGNATURE_OWNER,
    DECISION_SIGNATURES,
    DIAGRAM_FAMILIES,
    EXPRESSIONS,
    FAMILIES,
    FORM_PREFIX,
    FUNCTIONS,
    HAND_BASES,
    HAND_FILE,
    HAND_KINDS,
    HTML_SUFFIX,
    KIND_CALL,
    KIND_SPECIAL,
    KIND_STATEMENT,
    OPERATOR_LABELS,
    PAGE_FAMILIES,
    PAGE_STATUSES,
    PLACEHOLDERS,
    PRAGMA_BASE,
    PRAGMA_OUT_OF_SCOPE,
    PRAGMA_PAGE,
    SIGNATURE_BASES,
    SIGNATURE_PAGES,
    STATUS_FORMS,
    STATUS_NO_SYNTAX,
    STATUS_OUT_OF_SCOPE,
    STATUS_RESTATED,
    SYNTAX_DIR,
    UTILITY,
)
from scripts.syntax_catalogue_sqlite.models import (
    HandInputs,
    HandPage,
    Override,
    PageBuild,
    Signature,
    SqliteCatalogue,
    SqlitePage,
    Transcription,
)
from scripts.syntax_catalogue_sqlite.types import Row


def page_family(path: str) -> str:
    """The family of a page: its diagram's on a syntax/ page, else PAGE_FAMILIES'."""
    if path.startswith(SYNTAX_DIR):
        return DIAGRAM_FAMILIES[diagram_name(path)]
    return PAGE_FAMILIES[path]


def diagram_name(path: str) -> str:
    """`syntax/expr.html` as `expr`."""
    return path.removeprefix(SYNTAX_DIR).removesuffix(HTML_SUFFIX)


def page_base(path: str) -> str:
    """A page's id base: its signature base, else its file name without `lang_`, as a slug."""
    if path in SIGNATURE_BASES:
        return SIGNATURE_BASES[path]
    return slug(PurePosixPath(path).stem.removeprefix("lang_"))


def shape(grammar: str) -> str:
    """A signature with its argument names dropped, for comparing two pages' signatures."""
    return PLACEHOLDERS.sub("<>", grammar)


def add(*, build: PageBuild, family: str, base: str, groups: list[FormGroup]) -> PageBuild:
    """Record groups as the page's forms under a family and id base."""
    build.forms.extend((family, base, group) for group in groups)
    for group in groups:
        build.syntax.append(GRAMMAR_JOINER.join(group.grammars))
    return build


def diagram_forms(*, page: SqlitePage, hand: HandInputs, build: PageBuild) -> PageBuild:
    """A syntax/ page's diagram, as the forms of its reviewed transcription."""
    name: str = diagram_name(page.path)
    transcription: Transcription | None = hand.diagrams.get(name)
    if transcription is None:
        build.problems.append(f"{page.id}: diagram {name} has no transcription")
        return build
    if len(page.diagrams) != 1 or page.diagrams[0].svg_sha256 != transcription.svg_sha256:
        build.problems.append(f"{page.id}: transcription of {name} read a different diagram")
    if transcription.labels and len(transcription.labels) != len(transcription.grammar):
        build.problems.append(f"{page.id}: transcription of {name} labels some lines only")
    kind: str = KIND_SPECIAL if DIAGRAM_FAMILIES[name] == EXPRESSIONS else KIND_STATEMENT
    labels: tuple[str, ...] = transcription.labels or ("",) * len(transcription.grammar)
    groups: list[FormGroup] = [
        FormGroup(kind, label, (grammar,), ("", "Syntax"))
        for grammar, label in zip(transcription.grammar, labels, strict=True)
    ]
    build = add(build=build, family=DIAGRAM_FAMILIES[name], base=slug(name), groups=groups)
    build.decisions.add(DECISION_DIAGRAM)
    return build


def diagram_restates(*, page: SqlitePage, hand: HandInputs, build: PageBuild) -> PageBuild:
    """The diagrams a page other than its syntax/ page shows: each restates that page's forms."""
    for diagram in page.diagrams:
        transcription: Transcription | None = hand.diagrams.get(diagram.name)
        if transcription is None:
            continue
        owner: str = f"doc.syntax.{slug(diagram.name)}"
        build.restates.setdefault(owner, set()).update(transcription.grammar)
    if page.diagrams:
        build.decisions.add(DECISION_RESTATED)
    return build


def signature_grammar(
    *, page: SqlitePage, signature: Signature, hand: HandInputs, build: PageBuild
) -> tuple[tuple[str, ...], PageBuild]:
    """The grammar of one documented signature, read by hand where an override gives it."""
    override: Override | None = hand.overrides.get((page.id, signature.text))
    if override is not None:
        build.decisions.add(DECISION_OVERRIDE)
        return override.grammar, build
    grammar: str = call_grammar(signature.text)
    if not grammar:
        build.problems.append(f"{page.id}: signature {signature.text!r} is not a plain call")
        return (), build
    return (grammar,), build


def page_calls(
    *, page: SqlitePage, hand: HandInputs, build: PageBuild
) -> tuple[dict[str, tuple[str, list[str]]], PageBuild]:
    """Each name the page documents, in page order, with its anchor and grammars."""
    calls: dict[str, tuple[str, list[str]]] = {}
    for signature in signatures(text=page.html):
        grammars: list[str] = calls.setdefault(signature.name, (signature.anchor, []))[1]
        read: tuple[tuple[str, ...], PageBuild] = signature_grammar(
            page=page, signature=signature, hand=hand, build=build
        )
        build = read[1]
        grammars.extend(grammar for grammar in read[0] if grammar not in grammars)
    return calls, build


def function_owners(
    *, pages: dict[str, SqlitePage], hand: HandInputs
) -> dict[str, tuple[str, tuple[str, ...]]]:
    """For each function name, the first signature page in SIGNATURE_PAGES order and grammars."""
    owners: dict[str, tuple[str, tuple[str, ...]]] = {}
    for path in SIGNATURE_PAGES:
        scratch: PageBuild = PageBuild()
        calls: dict[str, tuple[str, list[str]]] = page_calls(
            page=pages[path], hand=hand, build=scratch
        )[0]
        for name, (_, grammars) in calls.items():
            owners.setdefault(name, (pages[path].id, tuple(grammars)))
    return owners


def signature_forms(
    *,
    page: SqlitePage,
    hand: HandInputs,
    owners: dict[str, tuple[str, tuple[str, ...]]],
    build: PageBuild,
) -> PageBuild:
    """One call form per function name; a name an earlier page documents alike is restated."""
    calls: list[FormGroup] = []
    operators: list[FormGroup] = []
    read: tuple[dict[str, tuple[str, list[str]]], PageBuild] = page_calls(
        page=page, hand=hand, build=build
    )
    build = read[1]
    for name, (anchor, grammars) in read[0].items():
        owner, owned = owners[name]
        if owner != page.id and {shape(g) for g in grammars} <= {shape(g) for g in owned}:
            build.restates.setdefault(owner, set()).add(GRAMMAR_JOINER.join(owned))
            build.decisions.add(DECISION_SIGNATURE_OWNER)
            continue
        if name in OPERATOR_LABELS:
            label: str = OPERATOR_LABELS[name]
            operators.append(FormGroup(KIND_SPECIAL, label, tuple(grammars), ("", anchor)))
            continue
        calls.append(FormGroup(KIND_CALL, slug(name), tuple(grammars), ("", anchor)))
    base: str = SIGNATURE_BASES[page.path]
    build = add(build=build, family=FUNCTIONS, base=base, groups=calls)
    build = add(build=build, family=EXPRESSIONS, base=base, groups=operators)
    if calls or operators:
        build.decisions.add(DECISION_SIGNATURES)
    return build


def pragma_forms(*, page: SqlitePage, hand: HandInputs, build: PageBuild) -> PageBuild:
    """One statement form per documented PRAGMA, its spellings as the form's lines."""
    groups: list[FormGroup] = []
    sections: list[tuple[str, str]] = list(pragma_sections(page.html))
    documented: set[str] = {section_name for section_name, _ in sections}
    build.problems.extend(
        f"{page.id}: out-of-scope PRAGMA {name} is not documented"
        for name in PRAGMA_OUT_OF_SCOPE
        if name not in documented
    )
    for name, section in sections:
        if name in PRAGMA_OUT_OF_SCOPE:
            build.decisions.add(DECISION_PRAGMA_SCOPE)
            continue
        override: Override | None = hand.overrides.get((page.id, name))
        grammars: tuple[str, ...]
        if override is not None:
            grammars = override.grammar
            build.decisions.add(DECISION_OVERRIDE)
        else:
            grammars = tuple(dict.fromkeys(pragma_grammar(line) for line in pragma_lines(section)))
        if not grammars:
            build.problems.append(f"{page.id}: PRAGMA {name} has no syntax line and no override")
            continue
        groups.append(FormGroup(KIND_STATEMENT, slug(name), grammars, ("", f"PRAGMA {name}")))
    build = add(build=build, family=UTILITY, base=PRAGMA_BASE, groups=groups)
    build.decisions.add(DECISION_PRAGMAS)
    return build


def hand_forms(*, page: SqlitePage, hand: HandInputs, build: PageBuild) -> PageBuild:
    """The page's hand-catalogued forms, each bound to the page it was read from."""
    for family, form in hand.forms.get(page.id, []):
        if form.page_sha256 != page.page_sha256:
            build.problems.append(f"{page.id}: hand form {form.label} reviewed a different page")
        if form.basis not in HAND_BASES or form.kind not in HAND_KINDS or family not in FAMILIES:
            build.problems.append(f"{page.id}: hand form {form.label} has an unknown field value")
        group: FormGroup = FormGroup(form.kind, form.label, form.grammars, ("", form.anchor))
        build = add(build=build, family=family, base=page_base(page.path), groups=[group])
        build.decisions.add(DECISION_HAND.format(basis=form.basis))
    return build


def page_build(
    *, page: SqlitePage, hand: HandInputs, owners: dict[str, tuple[str, tuple[str, ...]]]
) -> PageBuild:
    """Everything one page contributes and the forms of other pages it restates."""
    build: PageBuild = PageBuild()
    disposition: HandPage | None = hand.pages.get(page.id)
    if disposition is not None and disposition.status == STATUS_OUT_OF_SCOPE:
        return build
    if page.path.startswith(SYNTAX_DIR):
        build = diagram_forms(page=page, hand=hand, build=build)
    else:
        build = diagram_restates(page=page, hand=hand, build=build)
    if page.path in SIGNATURE_BASES:
        build = signature_forms(page=page, hand=hand, owners=owners, build=build)
    if page.path == PRAGMA_PAGE:
        build = pragma_forms(page=page, hand=hand, build=build)
    return hand_forms(page=page, hand=hand, build=build)


def page_status(*, page: SqlitePage, build: PageBuild, hand: HandInputs) -> tuple[str, PageBuild]:
    """forms, restated, or the reviewed status of a page with neither; a problem if unreviewed."""
    disposition: HandPage | None = hand.pages.get(page.id)
    if build.forms or build.restates:
        if disposition is not None:
            build.problems.append(f"{page.id}: has forms or restates and has a page status")
        return (STATUS_FORMS if build.forms else STATUS_RESTATED), build
    if disposition is None:
        build.problems.append(f"{page.id}: no diagram, signature or hand form and no status")
        return STATUS_NO_SYNTAX, build
    if disposition.page_sha256 != page.page_sha256:
        build.problems.append(f"{page.id}: status reviewed a different page")
    if disposition.status not in PAGE_STATUSES:
        build.problems.append(f"{page.id}: unknown status {disposition.status}")
    return disposition.status, build


def source_row(*, page: SqlitePage, status: str, build: PageBuild, retrieved_utc: str) -> Row:
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
    *, page: SqlitePage, build: PageBuild, existing: Existing, used_ids: frozenset[str]
) -> tuple[list[tuple[str, list[Row]]], frozenset[str]]:
    """A page's forms as (family, rows) per id base, and the ids now used."""
    grouped: dict[tuple[str, str], list[FormGroup]] = {}
    for family, base, group in build.forms:
        grouped.setdefault((family, base), []).append(group)
    emitted: list[tuple[str, list[Row]]] = []
    for (family, base), groups in grouped.items():
        context_base: str = f"{FORM_PREFIX}.{family}.{base}"
        if len(groups) == 1 and groups[0].label:
            context_base = f"{context_base}.{slug(groups[0].label)}"
        context: PageContext = PageContext(context_base, family, page.id, ())
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


def restated_sources(
    *, catalogue: Catalogue, restated: dict[tuple[str, str], set[str]]
) -> list[str]:
    """List each restating page in the also_sources of the form it restates."""
    problems: list[str] = []
    by_key: dict[tuple[str, str], Row] = {}
    for rows in catalogue.forms.values():
        for form in rows:
            for grammar in cast("list[object]", form["grammar"]):
                by_key[(str(form["source"]), sha256(str(grammar)))] = form
            by_key[(str(form["source"]), str(form["grammar_sha256"]))] = form
    collected: dict[str, tuple[Row, set[str]]] = {}
    for key, sources in sorted(restated.items()):
        form: Row | None = by_key.get(key)
        if form is None:
            problems.append(f"a restated grammar of {key[0]} is not a catalogued form")
            continue
        collected.setdefault(str(form["id"]), (form, set()))[1].update(sources)
    for form, sources in collected.values():
        rebuilt: Row = with_also_sources(form=form, sources=sources)
        form.clear()
        form.update(rebuilt)
    return problems


def build_catalogue(*, cache: Path, out: Path, retrieved_utc: str) -> SqliteCatalogue:
    """Sources and forms by family for every page of the page set."""
    hand: HandInputs = load_hand(out / HAND_FILE)
    existing: Existing = load_existing(out)
    used_ids: frozenset[str] = frozenset(existing.ids)
    loaded: tuple[list[SqlitePage], list[str]] = load_pages(cache)
    pages: list[SqlitePage] = loaded[0]
    problems: list[str] = loaded[1]
    by_path: dict[str, SqlitePage] = {page.path: page for page in pages}
    owners: dict[str, tuple[str, tuple[str, ...]]] = function_owners(pages=by_path, hand=hand)
    catalogue: Catalogue = Catalogue([], {f: [] for f in FAMILIES}, {f: [] for f in FAMILIES})
    restated: dict[tuple[str, str], set[str]] = {}
    for page in pages:
        build: PageBuild = page_build(page=page, hand=hand, owners=owners)
        for owner, grammars in build.restates.items():
            for grammar in grammars:
                restated.setdefault((owner, sha256(grammar)), set()).add(page.id)
        status, build = page_status(page=page, build=build, hand=hand)
        problems += build.problems
        catalogue.sources.append(
            source_row(page=page, status=status, build=build, retrieved_utc=retrieved_utc)
        )
        emitted, used_ids = emit_page(page=page, build=build, existing=existing, used_ids=used_ids)
        for family, forms in emitted:
            catalogue.forms[family].extend(forms)
    known: set[str] = {page.id for page in pages}
    names: set[str] = {diagram_name(p.path) for p in pages if p.path.startswith(SYNTAX_DIR)}
    problems += [f"hand forms for unknown page {s}" for s in hand.forms if s not in known]
    problems += [f"status for unknown page {s}" for s in hand.pages if s not in known]
    problems += [f"transcription of unknown diagram {n}" for n in hand.diagrams if n not in names]
    problems += restated_sources(catalogue=catalogue, restated=restated)
    used_overrides: set[tuple[str, str]] = set()
    for page in pages:
        if page.path in SIGNATURE_BASES:
            used_overrides |= {(page.id, s.text) for s in signatures(text=page.html)}
        if page.path == PRAGMA_PAGE:
            used_overrides |= {(page.id, name) for name, _ in pragma_sections(page.html)}
    problems += [
        f"override of {key[1]!r} matches no signature of {key[0]}"
        for key in hand.overrides
        if key not in used_overrides
    ]
    emitted_ids: set[str] = set()
    for rows in catalogue.forms.values():
        emitted_ids |= {str(f["id"]) for f in rows}
    for absent in absent_forms(existing=existing, emitted=emitted_ids):
        catalogue.forms[str(absent["family"])].append(absent)
    return SqliteCatalogue(catalogue=catalogue, problems=problems)
