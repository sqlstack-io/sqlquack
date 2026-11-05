"""Group a page's productions into catalogue forms and give each form a stable id."""

from __future__ import annotations

import re

from scripts.syntax_catalogue._helpers.grammar import (
    bare_words,
    call_name,
    expand_name_alternation,
    required_skeleton,
    sha256,
    slug,
    special_kind,
    split_calls,
    symbol_name,
)
from scripts.syntax_catalogue._helpers.pages import in_form_section
from scripts.syntax_catalogue.constants import (
    ACTION_NAME,
    ACTION_TAIL,
    CALL_FAMILIES,
    DECISION_ACTIONS,
    DECISION_CALL_SEQUENCE,
    DECISION_CALL_SIGNATURES,
    DECISION_DEFINITION_VARIANT,
    DECISION_NAME_ALTERNATION,
    DECISION_NAMED_AND_POSITIONAL,
    DEFAULT_ANCHOR,
    FALLBACK_NAME,
    GRAMMAR_JOINER,
    KIND_CALL,
    KIND_DEFINITION,
    KIND_PROSE,
    KIND_SPECIAL,
    KIND_STATEMENT,
    MIN_VARIANT_DEPTH,
    PROSE_KEY_PREFIX,
    SPECIAL_KIND_NAMED,
    STATUS_CATALOGUED,
    SYNTAX_HEADING,
    VARIANT_WORDS,
)
from scripts.syntax_catalogue.models import Existing, FormGroup, PageContext, Production
from scripts.syntax_catalogue.types import Row


def call_pieces(statements: list[Production]) -> tuple[list[tuple[str, tuple[str, ...]]], set[str]]:
    """Each documented call with its heading, splitting call lists and name alternations."""
    pieces: list[tuple[str, tuple[str, ...]]] = []
    decisions: set[str] = set()
    for production in statements:
        calls: list[str] = split_calls(production.grammar)
        if len(calls) > 1:
            decisions.add(DECISION_CALL_SEQUENCE)
        for call in calls:
            names: list[str] = expand_name_alternation(call)
            if len(names) > 1:
                decisions.add(DECISION_NAME_ALTERNATION)
            pieces += [(grammar, production.heading) for grammar in names]
    return pieces, decisions


def group_calls(statements: list[Production]) -> tuple[list[FormGroup], set[str]]:
    """Plain call signatures of one name form one call form; each special syntax is its own."""
    pieces, decisions = call_pieces(statements)
    plain: dict[str, list[str]] = {}
    special: dict[tuple[str, str], list[str]] = {}
    headings: dict[object, tuple[str, ...]] = {}
    for grammar, heading in pieces:
        kind: str = special_kind(grammar)
        key: tuple[str, str] = (call_name(grammar) or grammar, grammar if kind else "")
        target: list[str] = (
            plain.setdefault(key[0], []) if not kind else special.setdefault(key, [])
        )
        headings.setdefault(key[0] if not kind else key, heading)
        if grammar not in target:
            target.append(grammar)
    if any(len(grammars) > 1 for grammars in plain.values()):
        decisions.add(DECISION_CALL_SIGNATURES)
    if plain and any(special_kind(grammar) == SPECIAL_KIND_NAMED for _, grammar in special):
        decisions.add(DECISION_NAMED_AND_POSITIONAL)
    groups: list[FormGroup] = [
        FormGroup(KIND_CALL, name, tuple(grammars), headings[name])
        for name, grammars in plain.items()
    ]
    groups += [
        FormGroup(KIND_SPECIAL, "", tuple(grammars), headings[key])
        for key, grammars in special.items()
    ]
    return groups, decisions


def action_alternatives(*, grammar: str, definitions: set[str]) -> list[tuple[str, str]]:
    """`... { aAction | bAction }` where each alternative is an *Action the page defines."""
    match: re.Match[str] | None = ACTION_TAIL.search(grammar)
    if not match:
        return []
    names: list[str] = [name.strip() for name in match.group(1).split("|")]
    if not all(name in definitions and ACTION_NAME.search(name) for name in names):
        return []
    return [(name, grammar[: match.start()] + name) for name in names]


def is_definition_variant(
    *, production: Production, statement_headings: set[tuple[str, ...]]
) -> bool:
    """True for a definition under its own heading inside the syntax section."""
    heading: tuple[str, ...] = production.heading
    return (
        production.kind == KIND_DEFINITION
        and len(heading) >= MIN_VARIANT_DEPTH
        and bool(SYNTAX_HEADING.search(heading[1]))
        and in_form_section(heading=heading, syntax_sections_only=True)
        and heading not in statement_headings
    )


def group_forms(
    *, family: str, productions: list[Production], statements: list[Production]
) -> tuple[list[FormGroup], set[str]]:
    """The page's form groups and the extraction decisions that shaped them."""
    if family in CALL_FAMILIES:
        return group_calls(statements)
    decisions: set[str] = set()
    definitions: set[str] = {p.name for p in productions if p.kind == KIND_DEFINITION}
    groups: list[FormGroup] = []
    for production in statements:
        actions: list[tuple[str, str]] = action_alternatives(
            grammar=production.grammar, definitions=definitions
        )
        if actions:
            decisions.add(DECISION_ACTIONS)
        groups += [FormGroup(KIND_STATEMENT, n, (g,), production.heading) for n, g in actions]
        if not actions:
            groups.append(FormGroup(KIND_STATEMENT, "", (production.grammar,), production.heading))
    headings: set[tuple[str, ...]] = {p.heading for p in statements}
    variants: list[FormGroup] = [
        FormGroup(KIND_STATEMENT, p.name, (p.grammar,), p.heading)
        for p in productions
        if is_definition_variant(production=p, statement_headings=headings)
    ]
    if variants:
        decisions.add(DECISION_DEFINITION_VARIANT)
    return groups + variants, decisions


def skeleton_words(grammar: str) -> list[str]:
    """The upper-cased words of a grammar's required skeleton."""
    return [word.upper() for word in bare_words(required_skeleton(grammar))]


def variant_names(groups: list[FormGroup]) -> list[str]:
    """Distinguishing words of each form after the words every form on the page shares."""
    grammars: list[str] = [g.grammars[0] if g.grammars else "" for g in groups]
    words: list[list[str]] = [skeleton_words(grammar) for grammar in grammars]
    common: int = 0
    while all(len(w) > common + 1 for w in words) and len({w[common] for w in words}) == 1:
        common += 1
    names: list[str] = []
    for group, grammar, word_list in zip(groups, grammars, words, strict=True):
        if group.label and group.kind != KIND_PROSE:
            names.append(slug(group.label))
            continue
        name: str = slug("-".join(word_list[common:][:VARIANT_WORDS])) or symbol_name(grammar)
        if not name and len(group.heading) > 1:
            name = slug(group.heading[-1])
        names.append(name or FALLBACK_NAME)
    return names


def group_key(*, source: str, group: FormGroup) -> tuple[str, str]:
    """(source, grammar hash): the identity under which a form keeps its id."""
    if group.grammars:
        return source, sha256(GRAMMAR_JOINER.join(group.grammars))
    return source, PROSE_KEY_PREFIX + group.label


def new_form_id(
    *, context: PageContext, name: str, group: FormGroup, used_ids: set[str], multi: bool
) -> str:
    """A fresh id for a form: the page base, then its variant name, heading or a number."""
    form_id: str = f"{context.base}.{name}" if multi else context.base
    if form_id in used_ids and len(group.heading) > 1:
        form_id = f"{context.base}.{name}.{slug(group.heading[-1])}"
    number: int = 2
    while form_id in used_ids:
        form_id = f"{context.base}.{name}-{number}"
        number += 1
    return form_id


def form_row(*, context: PageContext, form_id: str, group: FormGroup, grammar_sha256: str) -> Row:
    """One catalogue form record."""
    return {
        "id": form_id,
        "family": context.family,
        "source": context.source,
        "anchor": " / ".join(group.heading[1:]) or DEFAULT_ANCHOR,
        "kind": group.kind,
        "grammar": list(group.grammars),
        "grammar_sha256": grammar_sha256,
        "uses": uses(definitions=context.definitions, grammars=group.grammars),
        "status": STATUS_CATALOGUED,
    }


def emit_forms(
    *, context: PageContext, groups: list[FormGroup], existing: Existing, used_ids: frozenset[str]
) -> tuple[list[Row], frozenset[str]]:
    """Form records for one page, keeping earlier ids, and the ids used once they are added."""
    keys: list[tuple[str, str]] = [group_key(source=context.source, group=g) for g in groups]
    stale: list[str] = [
        form_id
        for form_id in existing.by_source.get(context.source, [])
        if (context.source, existing.forms[form_id]["grammar_sha256"]) not in keys
    ]
    taken: set[str] = set(used_ids)
    forms: list[Row] = []
    for group, name, key in zip(groups, variant_names(groups), keys, strict=True):
        form_id: str | None = existing.by_key.get(key)
        natural: str = f"{context.base}.{name}" if len(groups) > 1 else context.base
        if form_id is None and natural in stale:
            form_id = natural
        if form_id is None:
            form_id = next(
                (old for old in stale if covers(existing=existing, old=old, group=group)), None
            )
        if form_id is not None and form_id in stale:
            stale.remove(form_id)
        if form_id is None:
            form_id = new_form_id(
                context=context, name=name, group=group, used_ids=taken, multi=len(groups) > 1
            )
        taken.add(form_id)
        forms.append(form_row(context=context, form_id=form_id, group=group, grammar_sha256=key[1]))
    return forms, frozenset(taken)


def covers(*, existing: Existing, old: str, group: FormGroup) -> bool:
    """True when every signature of an earlier form is still a signature of this form."""
    grammar: object = existing.forms[old]["grammar"]
    return isinstance(grammar, list) and bool(grammar) and set(grammar) <= set(group.grammars)


def refers_to(*, name: str, grammars: tuple[str, ...]) -> bool:
    """True when any grammar names the sub-grammar as a whole word."""
    pattern: re.Pattern[str] = re.compile(rf"\b{re.escape(name)}\b")
    return any(pattern.search(grammar) for grammar in grammars)


def uses(*, definitions: tuple[str, ...], grammars: tuple[str, ...]) -> list[str]:
    """The page's definitions a form's grammar refers to, every variant of a name it names."""
    return sorted(
        name for name in definitions if refers_to(name=name.split(" ")[0], grammars=grammars)
    )
