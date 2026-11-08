"""Derive obligation drafts mechanically from one form's normalized grammar."""

from __future__ import annotations

from collections import Counter

from scripts.syntax_obligations._helpers.grammar import parse
from scripts.syntax_obligations.classes.walk import Walk
from scripts.syntax_obligations.constants import (
    ACTION_ALL_OPTIONS,
    ACTION_MIN_OPTIONS,
    IDENTIFIER_ANCHOR,
    IDENTIFIER_CLASSES,
    IDENTIFIER_QUOTED,
    IDENTIFIER_RESERVED,
    KIND_ALTERNATIVE,
    KIND_IDENTIFIER,
    KIND_INTERACTION_ALL,
    KIND_NEGATIVE,
    KIND_NESTING,
    KIND_POSITIVE,
    MIN_INTERACTION_OPTIONS,
    NESTED_CLASSES,
    NESTING_DEPTHS,
    NO_BOUND,
    QUOTED_WITHOUT_BLANK,
    QUOTED_WITHOUT_BLANK_NAMES,
)
from scripts.syntax_obligations.models import Draft, Node, Spec


def derive(*, form: str, grammar: list[str]) -> list[Draft]:
    """Drafts in a fixed order: positive, then per production its parts, then form-wide cases."""
    return derive_trees(form=form, grammar=grammar)[0]


def derive_trees(
    *, form: str, grammar: list[str]
) -> tuple[list[Draft], list[tuple[Node, ...]], Counter[str]]:
    """The drafts, parsed productions and the classes of skipped argument negatives."""
    if not grammar:
        return [], [], Counter()
    walk: Walk = Walk(form)
    note: str = "the documented form with every required part and no optional part"
    walk.add(kind=KIND_POSITIVE, target="", note=note, spec=walk.spec(action=ACTION_MIN_OPTIONS))
    trees: list[tuple[Node, ...]] = []
    for index, production in enumerate(grammar):
        walk.production = index
        trees.append(parse(production))
        walk.sequence(nodes=trees[-1], where="", top=True)
        if index and not any(d.spec and d.spec.production == index for d in walk.drafts):
            walk.add(
                kind=KIND_ALTERNATIVE,
                target=walk.target(text=production),
                note="a documented production with no option or operand of its own (D118)",
                spec=Spec(ACTION_MIN_OPTIONS, index),
            )
    for keyword, spec in walk.dropped.items():
        walk.add(
            kind=KIND_NEGATIVE,
            target="",
            note=f"the form without its required {keyword}",
            spec=spec,
            arity=keyword in walk.arity_drops,
        )
    if walk.widest >= MIN_INTERACTION_OPTIONS:
        note = "every independent option of one production together"
        walk.add(kind=KIND_INTERACTION_ALL, target="", note=note, spec=Spec(ACTION_ALL_OPTIONS))
    for placeholder in walk.identifiers:
        for label, description in IDENTIFIER_CLASSES:
            reserved: Spec = walk.identifier_nodes[placeholder]
            walk.drafts.append(
                Draft(
                    form=form,
                    kind=KIND_IDENTIFIER,
                    target=placeholder,
                    note=_class_note(label=label, description=description, placeholder=placeholder),
                    anchor=IDENTIFIER_ANCHOR,
                    spec=reserved
                    if label == IDENTIFIER_RESERVED
                    else walk.quote_specs[placeholder],
                )
            )
    for label in sorted(walk.classes & NESTED_CLASSES):
        walk.drafts.append(
            Draft(
                form=form,
                kind=KIND_NESTING,
                target="",
                note=f"{label} operands nested; {NO_BOUND}",
                depths=NESTING_DEPTHS,
                spec=walk.nest_specs[label],
            )
        )
    return walk.drafts, trees, walk.argument_classes


def _class_note(*, label: str, description: str, placeholder: str) -> str:
    """The identifier class note, blank-free for names where a blank breaks Snowflake."""
    if label == IDENTIFIER_QUOTED and placeholder.strip("<>") in QUOTED_WITHOUT_BLANK_NAMES:
        return f"{label}: {QUOTED_WITHOUT_BLANK}"
    return f"{label}: {description}"
