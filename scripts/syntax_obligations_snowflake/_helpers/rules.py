"""Snowflake operand classes: the shared rules plus the pages' number, object and word names."""

from __future__ import annotations

import re

from scripts.grammar_obligations._helpers import grammar
from scripts.grammar_obligations.classes import walk
from scripts.syntax_obligations_snowflake.constants import (
    IDENTIFIER_CLASS,
    IDENTIFIER_NAMES,
    INTEGER_CLASS,
    INTEGER_NAMES,
    WORD_CLASS,
    WORD_NAMES,
)

_shared_class_rules: tuple[tuple[re.Pattern[str], str], ...] = grammar.CLASS_RULES


def install() -> None:
    """Classify operands, widen the integer rule, read quoted templates and walk list items."""
    grammar.TEMPLATE_OPERANDS = True
    walk.REPEATED_ITEMS = True
    grammar.CLASS_RULES = (
        (re.compile(IDENTIFIER_NAMES), IDENTIFIER_CLASS),
        (re.compile(WORD_NAMES), WORD_CLASS),
        *(
            (re.compile(f"{pattern.pattern}|{INTEGER_NAMES}"), label)
            if label == INTEGER_CLASS
            else (pattern, label)
            for pattern, label in _shared_class_rules
        ),
    )
