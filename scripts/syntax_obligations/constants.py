"""Settings for deriving syntax-coverage obligations from the catalogue grammar."""

from __future__ import annotations

import re

KIND_ALTERNATIVE: str = "alternative"
KIND_OPTION_PRESENT: str = "option_present"
KIND_OPTION_ABSENT: str = "option_absent"
KIND_OPERAND_MISSING: str = "required_operand_missing"
KIND_OPERAND_CLASS: str = "operand_type_class"
KIND_POSITIVE: str = "positive"
KIND_NEGATIVE: str = "negative_neighbour"
KIND_IDENTIFIER: str = "identifier_class"
KIND_NESTING: str = "nesting_depth"
KIND_INTERACTION: str = "interaction"
KIND_INTERACTION_ALL: str = "interaction_all_options"
KIND_BOUNDARY_BELOW: str = "boundary_below"
KIND_BOUNDARY_AT: str = "boundary_at"
KIND_BOUNDARY_ABOVE: str = "boundary_above"

NODE_OPTIONAL: str = "optional"
NODE_CHOICE: str = "choice"
NODE_TOKEN: str = "token"
TOKEN_PLACEHOLDER: str = "placeholder"
TOKEN_KEYWORD: str = "keyword"
TOKEN_PUNCT: str = "punct"
TOKEN_ELISION: str = "elision"
TOKEN_LITERAL: str = "literal"
OPEN_OPTIONAL: str = "["
CLOSE_OPTIONAL: str = "]"
OPEN_CHOICE: str = "{"
CLOSE_CHOICE: str = "}"
BAR: str = "|"
ELISION: str = "..."
PARENS: frozenset[str] = frozenset({"(", ")"})
TARGET_WIDTH: int = 120
NO_BOUND: str = "no documented bound"
NESTING_DEPTHS: tuple[int, ...] = (2, 8)
NESTED_CLASSES: frozenset[str] = frozenset({"statement", "query"})
STATUS_CATALOGUED: str = "catalogued"
KIND_PROSE: str = "prose"
ID_WIDTH: int = 3
PAIR_SEPARATOR: str = " & "

IDENTIFIER_ANCHOR: str = "doc.identifiers-syntax"
IDENTIFIER_CLASSES: tuple[tuple[str, str], ...] = (
    ("quoted", "a double-quoted identifier with blanks and mixed case"),
    (
        "reserved",
        "a reserved keyword, unquoted (expected rejection) and double-quoted (expected acceptance)",
    ),
)

TOKEN: re.Pattern[str] = re.compile(
    r"\s*(?:(?P<placeholder><[^<>]+>)|(?P<literal>'[^']*'|\"[^\"]*\")|(?P<elision>\.\.\.)"
    r"|(?P<bracket>[\[\]{}|])|(?P<keyword>[A-Za-z_][A-Za-z0-9_$]*)"
    r"|(?P<punct>:=|::|=>|->>|<>|!=|<=|>=|\|\||\S))"
)
CLASS_RULES: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"_(declaration|assignment|definition|handler)$"), "subgrammar"),
    (re.compile(r"statement"), "statement"),
    (re.compile(r"query|subquery"), "query"),
    (re.compile(r"(^|_)(data_)?type$|_type$"), "data_type"),
    (re.compile(r"number$|^n$|^num|_count$|^count$"), "integer_literal"),
    (re.compile(r"message|string|pattern|comment"), "string_literal"),
    (re.compile(r"expr|expression|condition|predicate|value|start|end|default"), "expression"),
    (
        re.compile(r"name|label|variable|alias|cursor|resultset|identifier|^[a-z_]*col"),
        "identifier",
    ),
)
DEFAULT_CLASS: str = "operand"
STRING_CLASS: str = "string_literal"
# Wrong-class negatives only where the wrong operand is lexically distinct, so the error should be
# a syntax error (1003) rather than a semantic one (D82). Expression operands get none: nearly any
# lexical class is an expression.
LEXICAL_WRONG: dict[str, str] = {
    "identifier": "a string literal",
    "integer_literal": "a string literal",
    "string_literal": "an unquoted identifier",
    "data_type": "a string literal",
    "statement": "a bare literal",
    "query": "a bare literal",
}
SUBGRAMMAR_CLASS: str = "subgrammar"
