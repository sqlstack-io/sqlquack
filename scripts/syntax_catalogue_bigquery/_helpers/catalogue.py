"""Build the BigQuery catalogue: sources, forms by family and definitions from the pinned pages."""

from __future__ import annotations

import itertools
import re
from pathlib import Path, PurePosixPath

from scripts.syntax_catalogue._helpers.catalogue import absent_forms, load_existing
from scripts.syntax_catalogue._helpers.forms import emit_forms
from scripts.syntax_catalogue._helpers.grammar import bare_words, sha256, slug
from scripts.syntax_catalogue.constants import GRAMMAR_JOINER, KIND_SPECIAL, KIND_STATEMENT
from scripts.syntax_catalogue.models import Catalogue, Existing, FormGroup, PageContext
from scripts.syntax_catalogue_bigquery._helpers.groups import (
    call_groups,
    call_name,
    converted,
    operator_groups,
    page_placeholders,
    section_label,
    suspect,
    syntax_table_groups,
    type_groups,
)
from scripts.syntax_catalogue_bigquery._helpers.hand import load_hand, strings
from scripts.syntax_catalogue_bigquery._helpers.notation import grammar_like, unlinked
from scripts.syntax_catalogue_bigquery._helpers.pages import load_pages
from scripts.syntax_catalogue_bigquery.constants import (
    ANCHOR_KEY,
    BLOCK_CORRECTED,
    BLOCK_NOTATION,
    BLOCK_PLACEHOLDER,
    BLOCK_RESTATED,
    BLOCK_STATUSES,
    DATA_TYPES_PAGE,
    DECISION_BLOCKS,
    DECISION_CALLS,
    DECISION_CORRECTED,
    DECISION_DEFINITION_FORM,
    DECISION_DROPPED,
    DECISION_HAND,
    DECISION_OPERATOR_TABLE,
    DECISION_PROSE_EXAMPLES,
    DECISION_RESTATED,
    DECISION_REVIEWED_NOTATION,
    DECISION_SELECTED_CALL,
    DECISION_SYNTAX_TABLES,
    DECISION_TYPE_TABLE,
    DEFINITION_CLOSE,
    DEFINITION_FORM_PAGES,
    DEFINITION_PREFIX,
    DEFINITION_SEPARATOR,
    EXPRESSIONS,
    FAMILIES,
    FORM_PREFIX,
    FUNCTION_PAGE,
    FUNCTIONS,
    GQL_BASE_PREFIX,
    GQL_PAGES,
    HAND_BASES,
    HAND_FILE,
    LEAD_FAMILIES,
    ML_FUNCTION_BASE,
    ML_PREFIX,
    ML_STATEMENT_FAMILY,
    ML_STATEMENT_PAGES,
    OPERATORS_PAGE,
    PAGE_BASES,
    PAGE_FAMILIES,
    PAGE_STATUSES,
    POSITION_EXAMPLE,
    POSITION_OTHER,
    POSITION_SYNOPSIS,
    POSITION_SYNTAX,
    SECTION_DEPTH_TWO,
    SECTION_FAMILIES,
    SECTION_SUFFIXES,
    SELECTED_CALL,
    STANDARD_SQL,
    STATEMENT_FAMILIES,
    STATUS_FORMS,
    STATUS_NO_SYNTAX,
    STATUS_OUT_OF_SCOPE,
    STATUS_RESTATED,
    SYNTAX_HEADING,
)
from scripts.syntax_catalogue_bigquery.models import (
    BigQueryCatalogue,
    BigQueryPage,
    BlockReview,
    HandInputs,
    HandPage,
    Located,
    PageBuild,
)
from scripts.syntax_catalogue_bigquery.types import Row


def stem(path: str) -> str:
    """The page path without the `standard-sql/` folder."""
    return path.removeprefix(STANDARD_SQL)


def is_function_page(path: str) -> bool:
    """A function reference page: its call synopses become call forms."""
    return bool(FUNCTION_PAGE.match(path)) and not ML_STATEMENT_PAGES.match(stem(path))


def page_family(path: str) -> str:
    """The family of a page's statement forms: functions on function pages, else expressions."""
    if path in PAGE_FAMILIES:
        return PAGE_FAMILIES[path]
    if ML_STATEMENT_PAGES.match(stem(path)):
        return ML_STATEMENT_FAMILY
    return FUNCTIONS if is_function_page(path) else EXPRESSIONS


def page_base(path: str) -> str:
    """A page's id base: the page name, without `_functions` and the BigQuery ML prefix."""
    if path in PAGE_BASES:
        return PAGE_BASES[path]
    name: str = stem(path)
    if ML_STATEMENT_PAGES.match(name):
        rest: str = name.removeprefix(ML_PREFIX)
        return rest.replace("create", "create-model", 1) if rest.startswith("create") else rest
    if name.startswith(ML_PREFIX):
        return ML_FUNCTION_BASE
    name = re.sub(r"[_-]functions$", "", PurePosixPath(name).name)
    return slug(name)


def section_base(*, page: BigQueryPage, heading: tuple[str, ...]) -> str:
    """The id base of a block's statement: its section heading, else the page base."""
    base: str = page_base(page.path)
    if page.path not in PAGE_FAMILIES or not heading:
        return base
    title: str = heading[1] if page.path in SECTION_DEPTH_TWO and len(heading) > 1 else heading[0]
    plain: str = SECTION_SUFFIXES.sub("", title.replace("`", "").strip())
    if SYNTAX_HEADING.fullmatch(plain):
        return base
    prefix: str = GQL_BASE_PREFIX if page.path in GQL_PAGES else ""
    return f"{prefix}{slug(plain)}" if slug(plain) else base


def statement_family(*, page: BigQueryPage, heading: tuple[str, ...], grammar: str) -> str:
    """The family of one statement: its leading keywords, its section, else its page."""
    words: tuple[str, ...] = tuple(w.upper() for w in bare_words(grammar))
    for lead, family in LEAD_FAMILIES:
        if words[: len(lead)] == lead:
            return family
    if heading and (page.path, heading[0]) in SECTION_FAMILIES:
        return SECTION_FAMILIES[(page.path, heading[0])]
    return page_family(page.path)


def candidates(page: BigQueryPage) -> list[Located]:
    """The page's blocks that may be notation: in a syntax section, a synopsis, or notation-like."""
    return [
        block
        for block in page.blocks
        if block.where in (POSITION_SYNTAX, POSITION_SYNOPSIS)
        or (block.where == POSITION_OTHER and grammar_like(block.text))
    ]


class _PageReader:
    """Collects one page's form groups and definitions by (family, section base)."""

    def __init__(self, *, page: BigQueryPage, hand: HandInputs) -> None:
        self.page: BigQueryPage = page
        self.hand: HandInputs = hand
        self.placeholders: frozenset[str] = page_placeholders(page)
        self.build: PageBuild = PageBuild()
        self.calls: dict[str, list[tuple[str, tuple[str, ...]]]] = {}
        self.used_reviews: set[tuple[str, str]] = set()
        self.restated_blocks: list[tuple[str, str]] = []

    def add(self, *, family: str, base: str, group: FormGroup) -> None:
        """Append a group under a family and section base."""
        self.build.groups.setdefault((family, base), []).append(group)

    def place(
        self, *, grammars: list[tuple[str, str]], heading: tuple[str, ...], family: str
    ) -> None:
        """Put each production where it belongs: a definition, a call, a statement or special."""
        base: str = section_base(page=self.page, heading=heading)
        local: dict[str, tuple[tuple[str, str], int]] = {}
        if self.page.path in DEFINITION_FORM_PAGES and grammars and all(n for n, _ in grammars):
            self.build.decisions.add(DECISION_DEFINITION_FORM)
            first, body = grammars[0]
            group: FormGroup = FormGroup(KIND_SPECIAL, slug(first), (body,), ("", *heading))
            self.add(family=family or page_family(self.page.path), base=base, group=group)
        for name, grammar in grammars:
            self.build.syntax.append(grammar)
            if name:
                owner: str = family or statement_family(
                    page=self.page, heading=heading, grammar=grammar
                )
                section: list[tuple[str, str]] = self.build.definitions.setdefault(
                    (owner, base), []
                )
                local.setdefault(name, ((owner, base), len(section)))
                section.append((name, grammar))
                continue
            wrapped: re.Match[str] | None = SELECTED_CALL.match(grammar)
            selected: bool = is_function_page(self.page.path) and bool(wrapped) and not family
            written: str = wrapped.group(1) if selected and wrapped is not None else grammar
            if selected:
                self.build.decisions.add(DECISION_SELECTED_CALL)
            called: str = call_name(written)
            if is_function_page(self.page.path) and called and not family:
                self.calls.setdefault(called, []).append((written, heading))
                continue
            chosen: str = family or (
                EXPRESSIONS
                if is_function_page(self.page.path)
                else statement_family(page=self.page, heading=heading, grammar=written)
            )
            kind: str = KIND_STATEMENT if chosen in STATEMENT_FAMILIES else KIND_SPECIAL
            label: str = slug(section_label(heading)) if chosen == EXPRESSIONS else ""
            self.add(
                family=chosen, base=base, group=FormGroup(kind, label, (written,), ("", *heading))
            )
        for name, grammar in grammars:
            if not name:
                wrapped = SELECTED_CALL.match(grammar)
                written = (
                    wrapped.group(1) if wrapped and is_function_page(self.page.path) else grammar
                )
                self.build.block_names.setdefault(written, {}).update(local)

    def read_block(self, block: Located) -> None:
        """One candidate block: its reviewed disposition, or its productions read as notation."""
        key: tuple[str, str] = (self.page.id, sha256(block.text))
        review: BlockReview | None = self.hand.blocks.get(key)
        grammars: list[tuple[str, str]]
        if review is not None:
            self.used_reviews.add(key)
            if review.status not in BLOCK_STATUSES:
                self.build.problems.append(f"{key}: unknown block status {review.status}")
            if review.status == BLOCK_NOTATION:
                self.build.decisions.add(DECISION_REVIEWED_NOTATION)
                grammars = converted(text=block.text, placeholders=self.placeholders)
                self.place(grammars=grammars, heading=block.heading, family=review.family)
                return
            if review.status != BLOCK_CORRECTED:
                self.build.decisions.add(DECISION_DROPPED)
                if review.status == BLOCK_RESTATED:
                    self.restated_blocks.append((key[1], block.text))
                return
            self.build.decisions.add(DECISION_CORRECTED)
            grammars = [corrected_line(line) for line in review.grammar]
            self.place(grammars=grammars, heading=block.heading, family=review.family)
            return
        grammars = converted(text=block.text, placeholders=self.placeholders)
        why: str = suspect(text=unlinked(block.text), grammars=grammars)
        if not why and block.where == POSITION_OTHER:
            why = "follows prose, so it may be an example"
        if not why and stem(self.page.path).startswith(ML_PREFIX) and block.heading:
            if not SYNTAX_HEADING.search(block.heading[-1]):
                why = "sits under an argument heading of a BigQuery ML page"
        if why:
            first: str = next((ln.strip() for ln in block.text.splitlines() if ln.strip()), "")
            self.build.problems.append(
                f"{self.page.id}: unreviewed block {key[1]} [{' / '.join(block.heading)}] "
                f"({block.where}) {why}: {first[:60]}"
            )
            return
        self.build.decisions.add(DECISION_BLOCKS)
        self.place(grammars=grammars, heading=block.heading, family="")

    def read(self) -> PageBuild:
        """Every candidate block, table and hand form of the page."""
        for block in candidates(self.page):
            self.read_block(block)
        if any(b.where in (POSITION_EXAMPLE, POSITION_OTHER) for b in self.page.blocks):
            self.build.decisions.add(DECISION_PROSE_EXAMPLES)
        if self.calls:
            self.build.decisions.add(DECISION_CALLS)
            for group in call_groups(self.calls):
                self.add(family=FUNCTIONS, base=page_base(self.page.path), group=group)
        if self.page.path == OPERATORS_PAGE:
            self.build.decisions.add(DECISION_OPERATOR_TABLE)
            known: set[str] = set()
            for group in operator_groups(self.page.rows):
                self.build.syntax.append(GRAMMAR_JOINER.join(group.grammars))
                self.add(family=EXPRESSIONS, base=page_base(self.page.path), group=group)
                known |= set(group.grammars)
            self.build.decisions.add(DECISION_SYNTAX_TABLES)
            for group in syntax_table_groups(rows=self.page.rows, known=known):
                self.build.syntax.append(GRAMMAR_JOINER.join(group.grammars))
                self.add(family=EXPRESSIONS, base=page_base(self.page.path), group=group)
        if self.page.path == DATA_TYPES_PAGE:
            self.build.decisions.add(DECISION_TYPE_TABLE)
            for group in type_groups(self.page.rows):
                self.build.syntax.append(GRAMMAR_JOINER.join(group.grammars))
                self.add(family=EXPRESSIONS, base=page_base(self.page.path), group=group)
        spelled: set[str] = spelled_words(self.build.syntax)
        for sha, text in self.restated_blocks:
            converted_lines: list[tuple[str, str]] = converted(
                text=text, placeholders=self.placeholders
            )
            words: set[str] = spelled_words([g for _, g in converted_lines])
            if not words <= spelled:
                self.build.problems.append(
                    f"{self.page.id}: block {sha} reviewed as restated spells "
                    f"{sorted(words - spelled)[:5]}, which no form of the page does"
                )
        for family, form in self.hand.forms.get(self.page.id, []):
            if form.page_sha256 != self.page.page_sha256:
                self.build.problems.append(
                    f"{self.page.id}: hand form {form.label} reviewed a different page"
                )
            if form.basis not in HAND_BASES:
                self.build.problems.append(f"{self.page.id}: hand form basis {form.basis}")
            hand_group: FormGroup = FormGroup(
                form.kind, form.label, form.grammars, ("", form.anchor)
            )
            self.build.syntax.extend(form.grammars)
            self.add(family=family, base=page_base(self.page.path), group=hand_group)
            self.build.decisions.add(DECISION_HAND.format(basis=form.basis))
        return self.build


def corrected_line(line: str) -> tuple[str, str]:
    """A corrected grammar line as (definition name, grammar); the name is "" for a form."""
    if line.startswith(DEFINITION_PREFIX) and DEFINITION_SEPARATOR in line:
        name: str = line[len(DEFINITION_PREFIX) :].split(DEFINITION_CLOSE, 1)[0]
        return name, line.split(DEFINITION_SEPARATOR, 1)[1].strip()
    return "", line


def spelled_words(grammars: list[str]) -> set[str]:
    """Every bare word the grammars spell, upper-cased."""
    words: set[str] = set()
    for grammar in grammars:
        words |= {w.upper() for w in bare_words(grammar)}
    return words


def page_status(*, page: BigQueryPage, build: PageBuild, hand: HandInputs) -> tuple[str, list[str]]:
    """forms, or the reviewed status of a page with none; and its problems when unreviewed."""
    disposition: HandPage | None = hand.pages.get(page.id)
    if build.groups:
        if disposition is not None:
            return STATUS_FORMS, [f"{page.id}: has forms and a page disposition"]
        return STATUS_FORMS, []
    if build.restated:
        return STATUS_RESTATED, []
    if disposition is None:
        return STATUS_NO_SYNTAX, [f"{page.id}: no form and no page disposition"]
    problems: list[str] = []
    if disposition.page_sha256 != page.page_sha256:
        problems.append(f"{page.id}: disposition reviewed a different page")
    if disposition.status not in PAGE_STATUSES:
        problems.append(f"{page.id}: unknown disposition status {disposition.status}")
    return disposition.status, problems


def source_row(*, page: BigQueryPage, status: str, build: PageBuild) -> Row:
    """One sources.toml record: the pin and what the build read from the page."""
    return {
        "id": page.id,
        "url": page.url,
        "title": page.title,
        "family": "" if status == STATUS_OUT_OF_SCOPE else page_family(page.path),
        "retrieved_utc": page.retrieved_utc,
        "page_sha256": page.page_sha256,
        "syntax_sha256": sha256("\n\n".join(build.syntax)),
        "syntax_blocks": len(build.syntax),
        "status": status,
        "decisions": sorted(build.decisions),
    }


def with_also_sources(*, form: Row, sources: set[str]) -> Row:
    """The form with the restating pages listed after its anchor."""
    rebuilt: Row = {}
    for key, value in form.items():
        rebuilt[key] = value
        if key == ANCHOR_KEY:
            rebuilt["also_sources"] = sorted(sources - {str(form["source"])})
    return rebuilt


def definition_rows(*, page: BigQueryPage, build: PageBuild) -> list[tuple[str, Row]]:
    """(family, row) for each definition of a page, a repeated name numbered from its second."""
    found: list[tuple[str, Row]] = []
    seen: dict[str, int] = {}
    for (family, _), definitions in sorted(build.definitions.items()):
        for name, grammar in definitions:
            seen[name] = seen.get(name, 0) + 1
            suffix: str = "" if seen[name] == 1 else f"-{seen[name]}"
            row: Row = {
                "id": f"{page.id}::{name}{suffix}",
                "source": page.id,
                "name": f"{name}{suffix}",
                "grammar": f"<{name}> ::= {grammar}",
            }
            found.append((family, row))
    return found


def block_uses(*, build: PageBuild) -> dict[str, dict[str, tuple[str, str]]]:
    """Each form grammar's own block: name to its numbered definition row and grammar."""
    numbered: dict[tuple[tuple[str, str], int], tuple[str, str]] = {}
    seen: dict[str, int] = {}
    for key, definitions in sorted(build.definitions.items()):
        for position, (name, grammar) in enumerate(definitions):
            seen[name] = seen.get(name, 0) + 1
            row: str = name if seen[name] == 1 else f"{name}-{seen[name]}"
            numbered[(key, position)] = (row, grammar)
    return {
        grammar: _numbered_block(names=names, numbered=numbered)
        for grammar, names in build.block_names.items()
    }


def _numbered_block(
    *,
    names: dict[str, tuple[tuple[str, str], int]],
    numbered: dict[tuple[tuple[str, str], int], tuple[str, str]],
) -> dict[str, tuple[str, str]]:
    """One block's names, each with its numbered definition row and grammar."""
    return {name: numbered[spot] for name, spot in names.items()}


def own_uses(*, uses: list[str], own: dict[str, tuple[str, str]]) -> list[str]:
    """`uses` with its block's names numbered, plus the block definitions they reach."""
    found: list[str] = []
    pending: list[str] = list(uses)
    while pending:
        name: str = pending.pop(0)
        row, grammar = own.get(name, (name, ""))
        if row in found:
            continue
        found.append(row)
        pending.extend(n for n in BLOCK_PLACEHOLDER.findall(grammar) if n in own)
    return sorted(found)


class _Builder:
    """The catalogue across pages: owners of each grammar, the pages restating it, used ids."""

    def __init__(self, *, out: Path) -> None:
        self.hand: HandInputs = load_hand(out / HAND_FILE)
        self.existing: Existing = load_existing(out)
        self.used_ids: frozenset[str] = frozenset(self.existing.ids)
        self.catalogue: Catalogue = Catalogue(
            [], {f: [] for f in FAMILIES}, {f: [] for f in FAMILIES}
        )
        self.owners: dict[str, Row] = {}
        self.owner_pages: dict[str, str] = {}
        self.restated: dict[str, set[str]] = {}
        self.used_reviews: set[tuple[str, str]] = set()

    def read(self, page: BigQueryPage) -> PageBuild:
        """The page's build; an out_of_scope page contributes nothing."""
        disposition: HandPage | None = self.hand.pages.get(page.id)
        if disposition is not None and disposition.status == STATUS_OUT_OF_SCOPE:
            return PageBuild()
        reader: _PageReader = _PageReader(page=page, hand=self.hand)
        build: PageBuild = reader.read()
        self.used_reviews |= reader.used_reviews
        return build

    def kept(
        self, *, page: BigQueryPage, build: PageBuild, key: tuple[str, str]
    ) -> tuple[list[FormGroup], set[str]]:
        """The groups of one section no earlier page or section owns, and the restated shas."""
        kept: list[FormGroup] = []
        restated: set[str] = set()
        own_definitions: bool = bool(build.definitions.get(key))
        for group in build.groups[key]:
            sha: str = sha256(GRAMMAR_JOINER.join(group.grammars))
            if sha in self.owners and (self.owner_pages[sha] == page.id or not own_definitions):
                self.restated.setdefault(sha, set()).add(page.id)
                restated.add(sha)
                continue
            self.owners.setdefault(sha, {})
            self.owner_pages.setdefault(sha, page.id)
            kept.append(group)
        return kept, restated

    def add_page(self, *, page: BigQueryPage, build: PageBuild) -> set[str]:
        """Emit the page's forms and definitions with stable ids; the restated grammar shas."""
        restated: set[str] = set()
        own: dict[str, dict[str, tuple[str, str]]] = block_uses(build=build)
        for family, base in sorted(build.groups):
            kept, section_restated = self.kept(page=page, build=build, key=(family, base))
            restated |= section_restated
            if not kept:
                continue
            definitions: list[tuple[str, str]] = build.definitions.get((family, base), [])
            context_base: str = f"{FORM_PREFIX}.{family}.{base}"
            if len(kept) == 1 and kept[0].label:
                context_base = f"{context_base}.{slug(kept[0].label)}"
            context: PageContext = PageContext(
                context_base, family, page.id, tuple(dict.fromkeys(n for n, _ in definitions))
            )
            forms, self.used_ids = emit_forms(
                context=context, groups=kept, existing=self.existing, used_ids=self.used_ids
            )
            for form in forms:
                local: dict[str, tuple[str, str]] = {}
                for grammar in strings(form["grammar"]):
                    local.update(own.get(grammar, {}))
                written: list[str] = list(strings(form["uses"]))
                form["uses"] = own_uses(uses=written, own=local)
                if not self.owners.get(str(form["grammar_sha256"])):
                    self.owners[str(form["grammar_sha256"])] = form
            self.catalogue.forms[family].extend(forms)
        for family, row in definition_rows(page=page, build=build):
            self.catalogue.definitions[family].append(row)
        return restated

    def finish(self, known: set[str]) -> list[str]:
        """List restating pages, keep absent forms, and report stale hand inputs."""
        for sha, sources in sorted(self.restated.items()):
            form: Row = self.owners[sha]
            rebuilt: Row = with_also_sources(form=form, sources=sources)
            form.clear()
            form.update(rebuilt)
        emitted: set[str] = {
            str(f["id"]) for f in itertools.chain.from_iterable(self.catalogue.forms.values())
        }
        for absent in absent_forms(existing=self.existing, emitted=emitted):
            self.catalogue.forms[str(absent["family"])].append(absent)
        hand: HandInputs = self.hand
        return (
            [f"hand forms for unknown page {s}" for s in hand.forms if s not in known]
            + [f"disposition for unknown page {s}" for s in hand.pages if s not in known]
            + [
                f"block review names no current candidate block: {source} {sha}"
                for source, sha in sorted(hand.blocks.keys() - self.used_reviews)
            ]
        )


def build_catalogue(*, cache: Path, out: Path) -> BigQueryCatalogue:
    """Sources, forms by family and definitions by family for every pinned page."""
    builder: _Builder = _Builder(out=out)
    pages: list[BigQueryPage] = load_pages(cache)
    problems: list[str] = []
    for page in pages:
        build: PageBuild = builder.read(page)
        build.restated.update(builder.add_page(page=page, build=build))
        if build.restated:
            build.decisions.add(DECISION_RESTATED)
        status, status_problems = page_status(page=page, build=build, hand=builder.hand)
        problems += build.problems + status_problems
        builder.catalogue.sources.append(source_row(page=page, status=status, build=build))
    problems += builder.finish({page.id for page in pages})
    return BigQueryCatalogue(catalogue=builder.catalogue, problems=problems)
