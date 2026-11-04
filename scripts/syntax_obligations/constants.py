"""Settings for deriving syntax-coverage obligations from the catalogue grammar."""

from __future__ import annotations

import re

from scripts.syntax_obligations.models import FormStyle

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
KIND_EXTREME: str = "extreme"

NODE_OPTIONAL: str = "optional"
NODE_CHOICE: str = "choice"
NODE_TOKEN: str = "token"
TOKEN_PLACEHOLDER: str = "placeholder"
TOKEN_KEYWORD: str = "keyword"
TOKEN_PUNCT: str = "punct"
TOKEN_ELISION: str = "elision"
TOKEN_LITERAL: str = "literal"
TOKEN_BRACKET: str = "bracket"
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
IDENTIFIER_RESERVED: str = "reserved"
IDENTIFIER_QUOTED: str = "quoted"
QUOTED_WITHOUT_BLANK: str = (
    "a double-quoted identifier in mixed case without blanks; blanks inside quotes belong to "
    "the double-quoted identifier form"
)
QUOTED_WITHOUT_BLANK_NAMES: frozenset[str] = frozenset(
    {"label", "nested_procedure_name", "arg_name"}
)
IDENTIFIER_CLASSES: tuple[tuple[str, str], ...] = (
    (IDENTIFIER_QUOTED, "a double-quoted identifier with blanks and mixed case"),
    (
        IDENTIFIER_RESERVED,
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
IDENTIFIER_CLASS: str = "identifier"
QUERY_CLASS: str = "query"
STATEMENT_CLASS: str = "statement"
LEXICALLY_DISTINCT_WRONG_FILL: dict[str, str] = {
    "identifier": "'v1'",
    "integer_literal": "'x'",
    "string_literal": "m1",
    "data_type": "'INT'",
    "statement": "1",
    "query": "1",
}
LEXICAL_WRONG: dict[str, str] = {
    "identifier": "a string literal",
    "integer_literal": "a string literal",
    "string_literal": "an unquoted identifier",
    "data_type": "a string literal",
    "statement": "a bare literal",
    "query": "a bare literal",
}
SUBGRAMMAR_CLASS: str = "subgrammar"

STATEMENT_TERMINATOR: str = ";"
RESERVED_WORD: str = "SELECT"
CLOSE_REPEAT_AFTER: frozenset[str] = frozenset({"(", ","})
PROCEDURE_WRAPPER: str = (
    "WITH p1 AS PROCEDURE ( ) RETURNS VARCHAR LANGUAGE SQL AS $$ {block} $$ CALL p1 ( ) ;"
)
CLASS_FILL: dict[str, str] = {
    "statement": "NULL",
    "query": "SELECT 1",
    "data_type": "INT",
    "integer_literal": "1",
    "string_literal": "'m'",
    "expression": "1",
    "identifier": "v1",
    "operand": "v1",
}
NAME_FILL: dict[str, str] = {
    "cursor_name": "c1",
    "resultset_name": "r1",
    "result_set_name": "r1",
    "exception_name": "e1",
    "label": "l1",
    "counter_variable": "i",
    "row_variable": "row1",
    "nested_procedure_name": "p1",
    "exception_number": "-20001",
    "condition": "TRUE",
    "boolean_expression": "TRUE",
    "variable_declaration": "v1 INT",
    "cursor_declaration": "c1 CURSOR FOR SELECT 1",
    "resultset_declaration": "r1 RESULTSET",
    "nested_stored_procedure_declaration": ("p1 PROCEDURE ( ) RETURNS INT AS BEGIN RETURN 1 ; END"),
    "exception_declaration": "e1 EXCEPTION",
    "exception_handler": "WHEN OTHER THEN NULL ;",
    "variable_assignment": "v1 := 1",
    "cursor_assignment": "c1 CURSOR FOR SELECT 1",
    "resultset_assignment": "r1 RESULTSET := ( SELECT 1 )",
    "nested_procedure_definition": "BEGIN RETURN 1 ; END",
}
HAND_CASE_KEYS: tuple[str, ...] = (
    "sql",
    "also_sql",
    "context",
    "outer_open",
    "open",
    "core",
    "close",
    "outer_close",
    "expect",
    "stage",
    "also_expect",
    "also_stage",
    "size",
)
EXPECT_ACCEPT: str = "accept"
EXPECT_REJECT: str = "reject"
EXPECT_OBSERVE: str = "observe"
HAND_SIZE: str = "size"
REPEAT_SLOT: str = "{n}"
LATER_ERROR_STAGES: frozenset[str] = frozenset({"", "semantic", "runtime", "error"})
LABEL_SLOT: str = "{label}"
BUILTIN_NAMES: frozenset[str] = frozenset(
    {"STATEMENT_ERROR", "EXPRESSION_ERROR", "OTHER", "SQLCODE", "SQLERRM", "SQLSTATE"}
)
CASE_CAPTURE: str = "execute"
LANE_A_BLOCK_SESSION: dict[str, object] = {
    "ABORT_DETACHED_QUERY": True,
    "STATEMENT_QUEUED_TIMEOUT_IN_SECONDS": 30,
    "STATEMENT_TIMEOUT_IN_SECONDS": 30,
    "TIMEZONE": "UTC",
    "USE_CACHED_RESULT": False,
}

SYNTAX_ERROR_PHASE: str = "syntax_error"
STAGE_BY_RECEIPT_PHASE: dict[str, str] = {
    "accepted": "",
    "semantic_error": "semantic",
    "runtime_error": "runtime",
}

ACTION_MIN_OPTIONS: str = "min"
ACTION_ALL_OPTIONS: str = "max"
ACTION_CHOICE_BRANCH: str = "branch"
ACTION_DROP_NODE: str = "drop"
ACTION_REPLACE_NODE: str = "replace"
ACTION_QUOTE_IDENTIFIERS: str = "quote"
ACTION_RESERVED_WORD: str = "reserved"
ACTION_NEST_OPERANDS: str = "nest"
ACTION_HAND: str = "hand"
HAND_SQL: str = "sql"
HAND_ALSO_SQL: str = "also_sql"
HAND_CORE: str = "core"
CONTEXT_BLOCK: str = "block"
CONTEXT_RAW: str = "raw"
CONTEXT_TOP: str = "top"
CONTEXT_PROCEDURE: str = "procedure"
DOUBLE_QUOTE: str = '"'
SINGLE_CHARACTER_LITERAL_LENGTH: int = 3
MIN_INTERACTION_OPTIONS: int = 2
PAIR_SIZE: int = 2
DIGEST_WIDTH: int = 12
MEASURE_KEY: str = "measure"
STAGE_KEY: str = "stage"
PACKET_KEYS: tuple[str, ...] = ("case_id", "sql", "sql_sha256", "capture", "expect")
GRAMMAR_DEFINITION: re.Pattern[str] = re.compile(r"^\s*<[^<>]+>\s*::=\s*")
QUOTED_PLACEHOLDER: re.Pattern[str] = re.compile(r"'<([^<>]+)>'")
CLOSERS: dict[str, str] = {OPEN_OPTIONAL: CLOSE_OPTIONAL, OPEN_CHOICE: CLOSE_CHOICE}

DECLARE_TAIL: str = " BEGIN RETURN 1 ; END ;"
BLOCK_OPEN: str = "BEGIN "
BLOCK_CLOSE: str = " END ;"
CURSOR: str = "{} CURSOR FOR SELECT 1"
RESULTSET: str = "{} RESULTSET DEFAULT ( SELECT 1 )"
ASYNC_RESULTSET: str = "{} RESULTSET DEFAULT ASYNC ( SELECT 1 )"
EXCEPTION: str = "{} EXCEPTION"
VARIABLE: str = "{} INT"
DECLARE_REPEAT: str = (
    "[ { <variable_declaration> | <cursor_declaration> | <resultset_declaration> | "
    "<nested_stored_procedure_declaration> | <exception_declaration> } ; ... ]"
)
LOOP_STOPS_AT_ONCE: dict[str, str] = {"statement": "BREAK"}
WHILE_STOPS_AT_ONCE: dict[str, str] = {"condition": "FALSE"}
OPEN_CURSOR_BEFORE_USE: str = "cursor_name"
INSIDE_LABELLED_LOOP: FormStyle = FormStyle(
    prefix="BEGIN FOR i0 IN 1 TO 1 DO ", suffix=" END FOR {label} ; END ;"
)
RAISE_INSIDE_HANDLER: FormStyle = FormStyle(
    prefix=BLOCK_OPEN,
    suffix=" EXCEPTION WHEN OTHER THEN NULL ; END ;",
    uses={"exception_name": EXCEPTION},
)
SCRIPTING_SELF_DECLARING_BLOCKS: dict[str, FormStyle] = {
    "sf.scripting.": FormStyle(prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE),
    "sf.scripting.begin": FormStyle(prefix="", suffix=""),
    "sf.scripting.declare.": FormStyle(prefix="DECLARE ", suffix=DECLARE_TAIL),
    "sf.scripting.declare.declare": FormStyle(
        prefix="", suffix=DECLARE_TAIL, clauses={DECLARE_REPEAT: "v2 INT ;"}
    ),
    "sf.scripting.exception": FormStyle(
        prefix="BEGIN NULL ; ",
        suffix=BLOCK_CLOSE,
        fills={"exception_name": "STATEMENT_ERROR"},
        clauses={"[ WHEN ... ]": "WHEN EXPRESSION_ERROR THEN NULL ;"},
        uses={"exception_name": EXCEPTION},
    ),
    "sf.scripting.case.end": FormStyle(
        prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE, clauses={"[ WHEN ... ]": "WHEN 2 THEN NULL ;"}
    ),
    "sf.scripting.case.end.syntax": FormStyle(
        prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE, clauses={"[ WHEN ... ]": "WHEN FALSE THEN NULL ;"}
    ),
    "sf.scripting.loop": FormStyle(prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE, fills=LOOP_STOPS_AT_ONCE),
    "sf.scripting.while": FormStyle(
        prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE, fills=WHILE_STOPS_AT_ONCE
    ),
    "sf.scripting.break": INSIDE_LABELLED_LOOP,
    "sf.scripting.continue": INSIDE_LABELLED_LOOP,
    "sf.scripting.raise": RAISE_INSIDE_HANDLER,
    "sf.scripting.open": FormStyle(
        prefix=BLOCK_OPEN,
        suffix=BLOCK_CLOSE,
        uses={"cursor_name": CURSOR, "bind_variable_1": VARIABLE, "bind_variable_2": VARIABLE},
    ),
    "sf.scripting.fetch": FormStyle(
        prefix=BLOCK_OPEN,
        suffix=BLOCK_CLOSE,
        uses={"cursor_name": CURSOR, "variable": VARIABLE},
        opens=OPEN_CURSOR_BEFORE_USE,
    ),
    "sf.scripting.close": FormStyle(
        prefix=BLOCK_OPEN,
        suffix=BLOCK_CLOSE,
        uses={"cursor_name": CURSOR},
        opens=OPEN_CURSOR_BEFORE_USE,
    ),
    "sf.scripting.for.do-end-for": FormStyle(
        prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE, uses={"cursor_name": CURSOR}
    ),
    "sf.scripting.await": FormStyle(
        prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE, uses={"result_set_name": ASYNC_RESULTSET}
    ),
    "sf.scripting.cancel": FormStyle(
        prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE, uses={"result_set_name": ASYNC_RESULTSET}
    ),
    "sf.scripting.let.let-cursor-for.cursor-assignment-syntax": FormStyle(
        prefix=BLOCK_OPEN, suffix=BLOCK_CLOSE, uses={"resultset_name": RESULTSET}
    ),
}
FAMILY_STYLES: dict[str, dict[str, FormStyle]] = {"scripting": SCRIPTING_SELF_DECLARING_BLOCKS}
