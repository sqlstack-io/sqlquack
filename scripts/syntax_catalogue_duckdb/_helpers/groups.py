"""Turn a page's diagrams, signatures and type rows into catalogue form groups."""

from __future__ import annotations

import re

from scripts.grammar_catalogue._helpers.forms import skeleton_words
from scripts.grammar_catalogue._helpers.grammar import normalize, slug, symbol_name
from scripts.grammar_catalogue.constants import KIND_CALL, KIND_SPECIAL, KIND_STATEMENT
from scripts.grammar_catalogue.models import FormGroup
from scripts.syntax_catalogue_duckdb._helpers.railroad import is_empty, render
from scripts.syntax_catalogue_duckdb.constants import (
    BARE_NAME,
    CALL_KEYWORDS,
    CALL_SIGNATURE,
    CELL_SPLIT,
    CODE_SPAN,
    EXAMPLE_COLUMN,
    EXPRESSIONS,
    LABEL_WORDS,
    MIN_ACTION_ITEMS,
    OPERATOR_HEADER,
    OPERATOR_NAMES,
    TREE_CHOICE,
    TREE_KEYWORD,
    TREE_SEQUENCE,
    TYPE_TABLE_HEADINGS,
    WORD_KEYWORD,
)
from scripts.syntax_catalogue_duckdb.models import Located
from scripts.syntax_catalogue_duckdb.types import Tree


def parts(tree: Tree) -> list[Tree]:
    """A sequence's non-empty items, with single-item sequences opened up."""
    found: list[Tree] = []
    for item in tree["items"]:
        if is_empty(item):
            continue
        if item["kind"] == TREE_SEQUENCE and len(parts(item)) == 1:
            found.extend(parts(item))
        elif item["kind"] == TREE_SEQUENCE:
            found.extend(parts(item))
        else:
            found.append(item)
    return found


def alternative_parts(alternative: Tree) -> list[Tree]:
    """An alternative's items, whether it is a sequence or a single node."""
    return parts(alternative) if alternative["kind"] == TREE_SEQUENCE else [alternative]


def is_action(alternative: Tree) -> bool:
    """An alternative led by a word keyword and holding more than that keyword."""
    items: list[Tree] = alternative_parts(alternative)
    return (
        len(items) >= MIN_ACTION_ITEMS
        and items[0]["kind"] == TREE_KEYWORD
        and bool(WORD_KEYWORD.match(items[0]["text"]))
    )


def label_of(grammar: str) -> str:
    """The first keywords of a grammar's required skeleton, as a form variant label."""
    words: list[str] = [word for word in skeleton_words(grammar) if WORD_KEYWORD.match(word)]
    return slug(" ".join(words[:LABEL_WORDS])) or symbol_name(grammar)


def is_split_choice(*, item: Tree, items: list[Tree]) -> bool:
    """A choice of non-empty alternatives: the whole diagram, or alternatives that are actions."""
    return (
        item["kind"] == TREE_CHOICE
        and len(item["items"]) > 1
        and all(not is_empty(alternative) for alternative in item["items"])
        and (len(items) == 1 or all(is_action(alternative) for alternative in item["items"]))
    )


def diagram_grammars(tree: Tree) -> list[tuple[str, str]]:
    """A diagram as (label, grammar) forms: one, or one per alternative of its split choice."""
    items: list[Tree] = parts(tree)
    choices: list[int] = [
        index for index, item in enumerate(items) if is_split_choice(item=item, items=items)
    ]
    if not choices:
        return [("", render(tree))]
    index: int = choices[0]
    before: str = " ".join(render(item) for item in items[:index])
    after: str = " ".join(render(item) for item in items[index + 1 :])
    forms: list[tuple[str, str]] = []
    for alternative in items[index]["items"]:
        grammar: str = normalize(f"{before} {render(alternative)} {after}")
        forms.append((label_of(render(alternative)), grammar))
    return forms


def diagram_groups(*, located: Located, tree: Tree, family: str) -> list[FormGroup]:
    """The form groups of one placed diagram."""
    kind: str = KIND_SPECIAL if family == EXPRESSIONS else KIND_STATEMENT
    forms: list[tuple[str, str]] = diagram_grammars(tree)
    return [
        FormGroup(kind, label if len(forms) > 1 else "", (grammar,), ("", *located.heading))
        for label, grammar in forms
    ]


def is_call(signature: str) -> bool:
    """A plain call signature: a name and a parenthesised argument list with no keyword grammar."""
    match: re.Match[str] | None = CALL_SIGNATURE.match(signature)
    return match is not None and not CALL_KEYWORDS.search(match.group(2))


def unique_texts(items: list[Located]) -> tuple[str, ...]:
    """The items' texts in first-seen order, each once."""
    return tuple(dict.fromkeys(s.text for s in items))


def signature_groups(signatures: tuple[Located, ...]) -> tuple[list[FormGroup], list[FormGroup]]:
    """(call groups, special groups): one call form per function name, one special per operator."""
    calls: dict[str, list[Located]] = {}
    specials: dict[str, list[Located]] = {}
    bare: list[Located] = []
    for located in signatures:
        signature: str = normalize(located.text)
        match: re.Match[str] | None = CALL_SIGNATURE.match(signature)
        if match and is_call(signature):
            owner: str = (located.owner or match.group(1)).lower()
            calls.setdefault(owner, []).append(Located(located.heading, signature))
        elif BARE_NAME.match(signature):
            bare.append(Located(located.heading, signature))
        else:
            name: str = label_of(signature.upper()) if match is None else slug(match.group(1))
            key: str = slug(f"op {symbol_name(signature) or name}") if match is None else name
            specials.setdefault(key, []).append(Located(located.heading, signature))
    for located in bare:
        if located.text.lower() not in calls:
            specials.setdefault(slug(located.text), []).append(located)
    call_groups: list[FormGroup] = [
        FormGroup(KIND_CALL, name, unique_texts(items), ("", *items[0].heading))
        for name, items in sorted(calls.items())
    ]
    special_groups: list[FormGroup] = [
        FormGroup(KIND_SPECIAL, key, unique_texts(items), ("", *items[0].heading))
        for key, items in sorted(specials.items())
    ]
    return call_groups, special_groups


def type_groups(rows: tuple[Located, ...]) -> list[FormGroup]:
    """One special form per data type table row, its name and aliases as alternatives."""
    groups: list[FormGroup] = []
    for row in rows:
        if not row.heading or row.heading[-1] not in TYPE_TABLE_HEADINGS:
            continue
        cells: list[str] = [cell.strip() for cell in row.text.strip().strip("|").split("|")]
        names: list[str] = CODE_SPAN.findall(cells[0])
        if not names:
            continue
        aliases: list[str] = CODE_SPAN.findall(cells[1]) if len(cells) > 1 else []
        groups.append(
            FormGroup(KIND_SPECIAL, slug(names[0]), tuple([*names, *aliases]), ("", *row.heading))
        )
    return groups


def cells_of(row: str) -> list[str]:
    """A table row's cells, a `|` inside a code span kept in its cell."""
    return [cell.strip() for cell in CELL_SPLIT.split(row.strip().strip("|"))]


def operator_shape(*, operator: str, example: str) -> str | None:
    """`<x> op <y>`, `op <x>` or `<x> op` from the row's example; None for a function call."""
    text: str = example.strip()
    if operator.isalpha() and text.lower().startswith(f"{operator.lower()}("):
        return None
    if text.upper().startswith(operator.upper()):
        return f"{operator} <x>"
    if text.endswith(operator):
        return f"<x> {operator}"
    return f"<x> {operator} <y>"


def unique_grammars(items: list[tuple[str, tuple[str, ...]]]) -> tuple[str, ...]:
    """The items' grammars in first-seen order, each once."""
    return tuple(dict.fromkeys(grammar for grammar, _ in items))


def operator_groups(rows: tuple[Located, ...]) -> list[FormGroup]:
    """One special form per operator in the page's operator tables, shaped by the row's example."""
    tables: set[int] = {row.table for row in rows if OPERATOR_HEADER.match(row.text)}
    found: dict[str, list[tuple[str, tuple[str, ...]]]] = {}
    for row in rows:
        if row.table not in tables or OPERATOR_HEADER.match(row.text):
            continue
        cells: list[str] = cells_of(row.text)
        if not cells or set(cells[0]) <= set(":- "):
            continue
        example: str = (
            (CODE_SPAN.findall(cells[EXAMPLE_COLUMN]) or [""])[0]
            if len(cells) > EXAMPLE_COLUMN
            else ""
        )
        for operator in CODE_SPAN.findall(cells[0]):
            grammar: str | None = operator_shape(operator=operator, example=example)
            if grammar is not None:
                found.setdefault(operator, []).append((grammar, ("", *row.heading)))
    return [
        FormGroup(
            KIND_SPECIAL,
            slug(f"op {OPERATOR_NAMES.get(operator, operator)}"),
            unique_grammars(items),
            items[0][1],
        )
        for operator, items in sorted(found.items())
    ]
