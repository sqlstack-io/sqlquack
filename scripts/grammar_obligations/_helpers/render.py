"""Render an obligation's case SQL from its form's grammar trees and its case spec."""

from __future__ import annotations

from dataclasses import replace

from scripts.grammar_obligations._helpers.qualify import qualified_references
from scripts.grammar_obligations.classes.renderer import Renderer
from scripts.grammar_obligations.constants import (
    ACTION_DROP_NODE,
    ACTION_HAND,
    DEFINE_FORM_FILLS,
    DEFINE_QUALIFIED_NAMES,
    EMPTY_CLAUSE,
    EMPTY_LIST,
    EMPTY_PROPERTY_LIST,
    EXPECT_ACCEPT,
    EXPECT_OBSERVE,
    EXPECT_REJECT,
    FAMILY_STYLES,
    SCHEMA_QUALIFIER,
    SCHEMA_REFERENCES,
)
from scripts.grammar_obligations.models import FormStyle, Node, Rendered, Spec


def submissions(
    *, trees: list[tuple[Node, ...]], style: FormStyle, spec: Spec, depths: tuple[int, ...]
) -> list[Rendered]:
    """A case spec's submissions; empty properties, clauses and `= ( )` lists observed if asked."""
    rendered: list[Rendered] = Renderer(trees=trees, style=style).submissions(
        spec=spec, depths=depths
    )
    if style.qualify_references:
        rendered = [replace(r, sql=qualified_references(r.sql)) for r in rendered]
    if spec.action == ACTION_HAND:
        return rendered
    return [
        Rendered(r.sql, EXPECT_OBSERVE, r.stage)
        if (
            style.observe_empty_properties
            and (
                EMPTY_PROPERTY_LIST.search(r.sql)
                or (r.expect == EXPECT_ACCEPT and EMPTY_CLAUSE.search(r.sql))
            )
        )
        or (
            r.expect == EXPECT_REJECT
            and spec.action == ACTION_DROP_NODE
            and EMPTY_LIST.search(r.sql)
        )
        else r
        for r in rendered
    ]


def style_of(*, family: str, form: str) -> FormStyle:
    """The longest matching form-id prefix style of the family."""
    styles: dict[str, FormStyle] = FAMILY_STYLES[family]
    best: str = max((prefix for prefix in styles if form.startswith(prefix)), key=len)
    if form not in DEFINE_QUALIFIED_NAMES:
        return styles[best]
    own, qualifier = DEFINE_QUALIFIED_NAMES[form]
    qualify: dict[str, str] = dict.fromkeys(SCHEMA_REFERENCES, SCHEMA_QUALIFIER)
    if own:
        qualify[own] = qualifier
    fills: dict[str, str] = {**styles[best].fills, **DEFINE_FORM_FILLS.get(form, {})}
    return replace(styles[best], qualify=qualify, qualify_references=True, fills=fills)
