"""Settings for building the documented Snowflake syntax catalogue."""

from __future__ import annotations

import re

FAMILIES: tuple[str, ...] = (
    "query",
    "dml",
    "ddl",
    "transaction_session",
    "security_admin",
    "utility",
    "scripting",
    "functions",
    "expressions",
    "classes",
)

CALL_FAMILIES: frozenset[str] = frozenset({"functions", "classes"})

DOCS_ORIGIN: str = "https://docs.snowflake.com"

DOCS_PREFIX: str = "https://docs.snowflake.com/en/"

TOP_LEVEL_PARTS: int = 2

USER_AGENT: str = "sqlquack-syntax-catalogue/1"

FETCH_WORKERS: int = 6

FETCH_ATTEMPTS: int = 4

FETCH_TIMEOUT_SECONDS: int = 40

HTTP_OK: int = 200

HTTP_NOT_FOUND: int = 404

MARKDOWN_SUFFIX: str = ".md"

UTC_FORMAT: str = "%Y-%m-%dT%H:%M:%SZ"

KIND_STATEMENT: str = "statement"

KIND_DEFINITION: str = "definition"

KIND_FRAGMENT: str = "fragment"

KIND_CALL: str = "call"

KIND_SPECIAL: str = "special"

KIND_PROSE: str = "prose"

STATUS_CATALOGUED: str = "catalogued"

SYNTAX_FENCE: str = "sqlsyntax"

PLAIN_FENCE: str = "plain"

DEFAULT_ANCHOR: str = "Syntax"

PROSE_KEY_PREFIX: str = "prose:"

GRAMMAR_JOINER: str = " || "

FALLBACK_NAME: str = "form"

VARIANT_WORDS: int = 4

LEAD_WORDS: int = 2

MIN_VARIANT_DEPTH: int = 3

DECISION_CALL_SIGNATURES: str = "several call signatures of one function name form one call form"

DECISION_CALL_SEQUENCE: str = "block listing several calls split into one signature per call"

DECISION_NAME_ALTERNATION: str = "function names written as alternatives split into one per name"

DECISION_NAMED_AND_POSITIONAL: str = "function documents both positional and named-argument syntax"

DECISION_ACTIONS: str = "statement whose tail alternates named *Action productions split per action"

DECISION_DEFINITION_VARIANT: str = (
    "definition under its own heading in the syntax section catalogued as a variant"
)

DECISION_HAND: str = "catalogued by hand from {basis}"

DECISION_RESTATED: str = (
    "every form restates syntax catalogued on another page; listed in its also_sources"
)

HAND_BASES: frozenset[str] = frozenset(
    {"html_parsed_literal", "table", "prose", "heading", "inline_code", "example", "code_block"}
)

SPECIAL_WORDS: frozenset[str] = frozenset(
    {
        "AS",
        "FROM",
        "IN",
        "FOR",
        "OVER",
        "WITHIN",
        "IGNORE",
        "RESPECT",
        "BOTH",
        "LEADING",
        "TRAILING",
        "USING",
        "ORDER",
        "PARTITION",
        "AT",
        "BEFORE",
        "BY",
        "ON",
        "INTO",
        "WHEN",
        "THEN",
        "ELSE",
        "END",
        "IS",
        "NOT",
        "NULL",
        "CASE",
        "PLACING",
        "SEPARATOR",
        "RETURNING",
        "WITH",
        "KEEP",
        "FILTER",
    }
)

SPECIAL_KIND_OVER: str = "OVER"

SPECIAL_KIND_WITHIN: str = "WITHIN"

SPECIAL_KIND_NAMED: str = "named"

SPECIAL_KIND_CAST: str = "cast"

SPECIAL_KIND_NONCALL: str = "noncall"

NAMED_ARROW: str = "=>"

CAST_OPERATOR: str = "::"

SYMBOLS: tuple[tuple[str, str], ...] = (
    ("::", "double-colon"),
    ("||", "concat"),
    ("->", "arrow"),
    ("=>", "named-arg"),
    (":", "colon"),
    ("[", "subscript"),
    (".", "dot"),
    ("!", "bang"),
)

ELISION: str = "..."

OPEN_PAREN: str = "("

CLOSE_PAREN: str = ")"

OPEN_OPTIONAL: str = "["

CLOSE_OPTIONAL: str = "]"

QUOTES: frozenset[str] = frozenset({"'", '"'})

HAND_KINDS: frozenset[str] = frozenset({"statement", "call", "special"})

GRAMMAR_MARKS: frozenset[str] = frozenset("<[{")

OPENERS: dict[str, str] = {"[": "]", "{": "}", "(": ")"}

CLOSERS: dict[str, str] = {"]": "[", "}": "{", ")": "("}

FENCE: re.Pattern[str] = re.compile(r"^\s*(```+|~~~+)\s*([\w+-]*)\s*(.*?)\s*$")

SYNTAX_TITLE: re.Pattern[str] = re.compile(r'title="Syntax"', re.I)

HEADING: re.Pattern[str] = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")

DEFINITION: re.Pattern[str] = re.compile(r"^\s*<?([A-Za-z_][\w ]*?)>?\s*(\([^)]*\))?\s*::=")

WORD: re.Pattern[str] = re.compile(r"[A-Za-z_][A-Za-z0-9_$]*")

ELISION_LINE: re.Pattern[str] = re.compile(r"^\s*(\[\s*)?\.\.\.(\s*\])?\s*$")

QUOTED_OR_PLACEHOLDER: re.Pattern[str] = re.compile(r"<[^<>]*>|'[^']*'|\"[^\"]*\"")

QUOTED_OR_ANGLE: re.Pattern[str] = re.compile(r"'[^']*'|<[^<>]*>")

SQL_LINE_COMMENT: re.Pattern[str] = re.compile(r"(^|\s)--(?=(?:[^']*'[^']*')*[^']*$).*$")

CALL_START: re.Pattern[str] = re.compile(r"^\s*([A-Za-z_][\w$.]*)\s*\(")

NAME_ALTERNATION: re.Pattern[str] = re.compile(
    r"^\s*\{\s*([A-Za-z_][\w$.]*(?:\s*\|\s*[A-Za-z_][\w$.]*)+)\s*\}\s*(?=\()"
)

ACTION_TAIL: re.Pattern[str] = re.compile(r"\{\s*([A-Za-z_]\w*(?:\s*\|\s*[A-Za-z_]\w*)+)\s*\}\s*$")

ACTION_NAME: re.Pattern[str] = re.compile(r"Actions?$", re.I)

LABEL_CHUNK: re.Pattern[str] = re.compile(r"^[A-Za-z][\w ]*:$")

BULLET_CHUNK: re.Pattern[str] = re.compile(r"^\s*[-*]\s")

SYNTAX_HEADING: re.Pattern[str] = re.compile(r"syntax", re.I)

NON_FORM_SECTIONS: re.Pattern[str] = re.compile(
    r"^(parameters|required parameters|optional parameters|arguments|returns|usage notes|"
    r"examples?|output|access control.*|(?-i:.*\b[Pp]arameters\b.*|.*\b[Ee]xamples?\b.*)|"
    r".*usage notes.*|"
    r"collation details|error handling|.*\bactions\b.*\(`.*`\))$",
    re.I,
)

SESSION_TITLES: re.Pattern[str] = re.compile(
    r"^(BEGIN|START TRANSACTION|COMMIT|ROLLBACK|USE\b.*|SET|UNSET|ALTER SESSION|SHOW TRANSACTIONS|"
    r"SHOW LOCKS|SHOW PARAMETERS|SHOW VARIABLES|DESCRIBE RESULT|ABORT TRANSACTION)$",
    re.I,
)

DML_TITLES: re.Pattern[str] = re.compile(
    r"^(INSERT.*|MERGE|UPDATE|DELETE|TRUNCATE TABLE|COPY INTO.*|COPY FILES|PUT|GET|LIST|REMOVE)$",
    re.I,
)

UTILITY_TITLES: re.Pattern[str] = re.compile(r"^(EXPLAIN|CALL.*|EXECUTE .*|CANCEL.*)$", re.I)

QUERY_TITLES: re.Pattern[str] = re.compile(r"^(SELECT|WITH)$", re.I)

H1: re.Pattern[str] = re.compile(r"<h1[^>]*>(.*?)</h1>", re.S)

PILCROW_STRIP: str = " \n\t\u00b6"

TAG: re.Pattern[str] = re.compile(r"<[^>]+>")
STATUS_ABSENT: str = "absent_from_source"
