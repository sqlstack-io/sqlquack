"""The Snowflake operand classes: the shared rules with the pages' other numeric operand names."""

from __future__ import annotations

import re

from scripts.grammar_obligations._helpers import grammar
from scripts.syntax_obligations_snowflake.constants import INTEGER_CLASS, INTEGER_NAMES

_shared_class_rules: tuple[tuple[re.Pattern[str], str], ...] = grammar.CLASS_RULES


def install() -> None:
    """Widen the shared integer rule in place, so every earlier shared rule still wins first."""
    grammar.CLASS_RULES = tuple(
        (re.compile(f"{pattern.pattern}|{INTEGER_NAMES}"), label)
        if label == INTEGER_CLASS
        else (pattern, label)
        for pattern, label in _shared_class_rules
    )
