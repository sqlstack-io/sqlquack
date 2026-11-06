"""Group a page's statements, actions, calls and operators into catalogue forms with labels."""

from __future__ import annotations

import re

from scripts.syntax_catalogue._helpers.grammar import bare_words, required_skeleton, slug
from scripts.syntax_catalogue.models import FormGroup
from scripts.syntax_catalogue_pg.constants import (
    ACTION_PLACEHOLDER,
    CALL_NAME,
    DECISION_ACTIONS,
    DECISION_OPERATORS,
    DECISION_SIGNATURES,
    DECISION_SPECIAL,
    KIND_CALL,
    KIND_SPECIAL,
    KIND_STATEMENT,
    OPERATOR_CHARS,
)
from scripts.syntax_catalogue_pg.models import Definition, Signature

LABEL_WORDS: int = 4
PLACEHOLDER: re.Pattern[str] = re.compile(r"<([^<>]+)>")


def without_choices(text: str) -> str:
    """The text without `{ a | b }` choices, or with each choice's first branch when that
    leaves no keyword."""
    out: list[str] = []
    depth: int = 0
    for ch in text:
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth = max(0, depth - 1)
        elif depth == 0:
            out.append(ch)
    stripped: str = "".join(out)
    if any(w.isupper() for w in bare_words(stripped)):
        return stripped
    return CHOICE.sub(lambda m: m.group(1).split("|")[0], text)


CHOICE: re.Pattern[str] = re.compile(r"\{([^{}]*)\}")


def skeleton_tokens(grammar: str, *, placeholders: bool) -> list[str]:
    """Required keywords of a grammar, and its required placeholders' names when asked."""
    text: str = without_choices(required_without_options(grammar))
    if not placeholders:
        return [w.upper() for w in bare_words(required_skeleton(text)) if w.isupper()]
    return [
        w.lower() if not w.isupper() else w
        for w in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", text.replace("<", " ").replace(">", " "))
    ]


def required_without_options(grammar: str) -> str:
    """The grammar without `[ ... ]` parts."""
    out: list[str] = []
    depth: int = 0
    for ch in grammar:
        if ch == "[":
            depth += 1
        elif ch == "]":
            depth = max(0, depth - 1)
        elif depth == 0:
            out.append(ch)
    return "".join(out)


def common_prefix(word_lists: list[list[str]]) -> int:
    """How many leading words every list shares."""
    common: int = 0
    while all(len(w) > common for w in word_lists) and len({w[common] for w in word_lists}) == 1:
        common += 1
    return common


def keywords(grammar: str) -> list[str]:
    """Every upper-case keyword of a grammar, optional parts included, in order."""
    return [w for w in bare_words(grammar) if w.isupper()]


def statement_labels(grammars: list[str]) -> list[str]:
    """A distinguishing label per statement: its required keywords after the shared ones,
    then, for labels that still clash, the keywords only that statement has."""
    if len(grammars) == 1:
        return [""]
    keyword_lists: list[list[str]] = [skeleton_tokens(g, placeholders=False) for g in grammars]
    common: int = common_prefix(keyword_lists)
    labels: list[str] = [slug("-".join(w[common:][:LABEL_WORDS])) for w in keyword_lists]
    for label in sorted(set(labels)):
        clashing: list[int] = [i for i, other in enumerate(labels) if other == label]
        if len(clashing) < 2 and label:
            continue
        shared: set[str] = set.intersection(*(set(keywords(grammars[i])) for i in clashing))
        for index in clashing:
            own: list[str] = [w for w in keywords(grammars[index]) if w not in shared]
            extra: str = slug("-".join(dict.fromkeys(own)))
            if not label and not extra:
                full: list[str] = skeleton_tokens(grammars[index], placeholders=True)
                extra = slug("-".join(full[common:][:LABEL_WORDS]))
            labels[index] = "-".join(p for p in (label, extra) if p)
    for label in sorted(set(labels)):
        clashing = [i for i, other in enumerate(labels) if other == label]
        if len(clashing) < 2:
            continue
        for index in clashing:
            names: list[str] = PLACEHOLDER.findall(required_without_options(grammars[index]))
            extra = slug("-".join(names[-2:]))
            labels[index] = "-".join(p for p in (label, extra) if p)
    return labels


def statement_groups(
    *, statements: list[str], definitions: list[Definition], heading: tuple[str, ...]
) -> tuple[list[FormGroup], set[str]]:
    """One form per statement, and per action for a statement taking `<action> [, ...]`."""
    actions: dict[str, Definition] = {
        d.name: d for d in definitions if d.name == ACTION_PLACEHOLDER and d.listed
    }
    expanded: list[str] = []
    decisions: set[str] = set()
    for grammar in statements:
        action: Definition | None = actions.get(ACTION_PLACEHOLDER)
        marker: str = f"<{ACTION_PLACEHOLDER}>"
        if action is not None and marker in grammar:
            decisions.add(DECISION_ACTIONS)
            expanded += [grammar.replace(marker, alt, 1) for alt in action.alternatives]
            continue
        expanded.append(grammar)
    unique: list[str] = list(dict.fromkeys(expanded))
    labels: list[str] = statement_labels(unique)
    return [
        FormGroup(KIND_STATEMENT, label, (grammar,), heading)
        for label, grammar in zip(labels, unique, strict=True)
    ], decisions


def operator_label(signature: Signature) -> str:
    """`op-<char names>` for a symbol operator, or the slug of a keyword operator's words."""
    text: str = " ".join(signature.literals)
    if re.search(r"[A-Za-z]", text):
        return slug(" ".join(text.split()[: LABEL_WORDS + 2]))
    symbol: str = "".join(text.split())
    return "op-" + "-".join(OPERATOR_CHARS.get(ch, f"u{ord(ch):04x}") for ch in symbol)


def operator_groups(signatures: list[Signature]) -> list[FormGroup]:
    """One special form per operator (its literal text) with every documented signature."""
    by_key: dict[str, list[Signature]] = {}
    for signature in signatures:
        key: str = " ".join(signature.literals) or signature.grammar
        by_key.setdefault(key, []).append(signature)
    groups: list[FormGroup] = []
    for found in by_key.values():
        grammars: tuple[str, ...] = tuple(dict.fromkeys(s.grammar for s in found))
        groups.append(FormGroup(KIND_SPECIAL, operator_label(found[0]), grammars, found[0].heading))
    return groups


def is_special(grammar: str) -> bool:
    """True for a signature with its own grammar: no call parentheses, a keyword inside them
    other than VARIADIC, or named-argument or cast syntax."""
    match: re.Match[str] | None = CALL_NAME.match(grammar)
    if match is None or "=>" in grammar or "::" in grammar:
        return True
    words: list[str] = [w for w in bare_words(grammar[match.end() :]) if w.isupper()]
    return any(w not in PLAIN_CALL_WORDS for w in words)


PLAIN_CALL_WORDS: frozenset[str] = frozenset({"VARIADIC", "DEFAULT"})


def call_groups(signatures: list[Signature]) -> tuple[list[FormGroup], set[str]]:
    """Call forms per function name, and special forms per function name and keyword skeleton
    (rule 7), merging type overloads of one special syntax into one form."""
    plain: dict[str, FormGroup] = {}
    merged: dict[tuple[str, tuple[str, ...]], FormGroup] = {}
    for signature in signatures:
        grammar: str = signature.grammar
        name: str = function_name(grammar)
        if is_special(grammar):
            key: tuple[str, tuple[str, ...]] = (
                name,
                tuple(skeleton_tokens(grammar, placeholders=False)),
            )
            found: FormGroup | None = merged.get(key)
            grammars: tuple[str, ...] = (found.grammars if found else ()) + (grammar,)
            heading: tuple[str, ...] = found.heading if found else signature.heading
            merged[key] = FormGroup(KIND_SPECIAL, "", tuple(dict.fromkeys(grammars)), heading)
            continue
        call: FormGroup | None = plain.get(name)
        grammars = (call.grammars if call else ()) + (grammar,)
        heading = call.heading if call else signature.heading
        plain[name] = FormGroup(KIND_CALL, name, tuple(dict.fromkeys(grammars)), heading)
    calls: list[FormGroup] = list(plain.values())
    specials: list[FormGroup] = list(merged.values())
    labels: list[str] = special_labels(specials, taken={slug(g.label) for g in calls})
    specials = [
        FormGroup(g.kind, label, g.grammars, g.heading)
        for g, label in zip(specials, labels, strict=True)
    ]
    decisions: set[str] = set()
    if any(len(g.grammars) > 1 for g in calls):
        decisions.add(DECISION_SIGNATURES)
    if specials:
        decisions.add(DECISION_SPECIAL)
    return calls + specials, decisions


def function_name(grammar: str) -> str:
    """The lower-cased name a call or keyword function grammar starts with."""
    match: re.Match[str] | None = re.match(r"^\s*([A-Za-z_][\w$.]*)", grammar)
    return match.group(1).lower() if match else ""


def special_labels(specials: list[FormGroup], *, taken: set[str]) -> list[str]:
    """Labels for special forms: the name and its required keywords, then the keywords only
    that form has where two would share a label or one would take a call form's label."""
    bases: list[str] = []
    for group in specials:
        grammar: str = group.grammars[0]
        name: str = function_name(grammar)
        tail: list[str] = [
            w for w in skeleton_tokens(grammar, placeholders=False) if w.lower() != name
        ][:LABEL_WORDS]
        noparens: bool = CALL_NAME.match(grammar) is None and not tail
        base: str = slug("-".join([name, *tail])) or "form"
        bases.append(f"{base}-noparens" if noparens and name else base)
    labels: list[str] = list(bases)
    for label in sorted(set(bases)):
        clashing: list[int] = [i for i, b in enumerate(bases) if b == label]
        if len(clashing) < 2 and label not in taken:
            continue
        sets: list[set[str]] = [set(keywords_of(specials[i])) for i in clashing]
        shared: set[str] = set.intersection(*sets) if len(sets) > 1 else set()
        for index in clashing:
            own: list[str] = [w for w in keywords_of(specials[index]) if w not in shared]
            extra: str = slug("-".join(list(dict.fromkeys(own))[:LABEL_WORDS]))
            labels[index] = "-".join(p for p in (label, extra) if p)
    return labels


def keywords_of(group: FormGroup) -> list[str]:
    """The keywords of a form's first grammar after its function name."""
    grammar: str = group.grammars[0]
    name: str = function_name(grammar)
    return [w for w in keywords(grammar) if w.lower() != name]


def signature_groups(
    signatures: list[Signature],
) -> tuple[list[FormGroup], list[FormGroup], set[str]]:
    """(function forms, operator forms, decisions) for one page's signatures."""
    calls: list[Signature] = [s for s in signatures if s.is_function]
    operators: list[Signature] = [s for s in signatures if not s.is_function]
    groups, decisions = call_groups(calls) if calls else ([], set())
    operator_list: list[FormGroup] = operator_groups(operators)
    if operator_list:
        decisions.add(DECISION_OPERATORS)
    return groups, operator_list, decisions
