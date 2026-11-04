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
OPEN_PAREN: str = "("
CLOSE_PAREN: str = ")"
PARENS: frozenset[str] = frozenset({OPEN_PAREN, CLOSE_PAREN})
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
    (re.compile(r"^(session|bind|snowflake_scripting)_variable$"), "operand"),
    (re.compile(r"statement"), "statement"),
    (re.compile(r"query|subquery"), "query"),
    (re.compile(r"(^|_)(data_)?type$|_type$"), "data_type"),
    (re.compile(r"number$|^n$|^num|_count$|^count$|position$"), "integer_literal"),
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
EXPECT_TERMINATES: str = "terminates"
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
ELISION_START: str = "^"
PURE_ELISION: str = "[ ... ]"
LIST_SEPARATOR: str = ","
LIST_ITEM_KEYWORDS: frozenset[str] = frozenset(
    {"AS", "NOT", "NULL", "ASC", "DESC", "NULLS", "FIRST", "LAST"}
)
GLUED_PREFIXES: re.Pattern[str] = re.compile(r"(?<![^ ])([$:@]) (?=[^ =])")
QUERY_FORMS: tuple[str, ...] = ("sf.query.constructs",)
UNPACKED_FORMS: frozenset[str] = frozenset(QUERY_FORMS)
QUERY_ELISIONS: dict[str, tuple[str, tuple[str, ...], str]] = {
    PURE_ELISION: ("", QUERY_FORMS, "further clauses of the SELECT statement"),
    ELISION_START: ("SELECT 1 FROM t1", QUERY_FORMS, "the query the clause belongs to"),
    "SELECT": (
        "1",
        ("sf.query.select.select", "sf.query.select.select.selecting-specific-columns"),
        "the select list",
    ),
    "TOP": (
        "1",
        ("sf.query.select.select", "sf.query.select.select.selecting-specific-columns"),
        "the select list after TOP <n>",
    ),
    "FROM": ("t1", ("sf.query.from",), "the FROM source"),
    "JOIN": ("t2", ("sf.query.join.join", "sf.query.join.join-2"), "the joined source"),
    "WITH": ("w1 AS ( SELECT 1 )", ("sf.query.with.select",), "the common table expressions"),
    "INTO": (":v1", ("sf.query.into",), "the INTO targets"),
    "WHERE": ("TRUE", ("sf.query.where",), "the WHERE predicate"),
    "GROUP BY": (
        "1",
        (
            "sf.query.group-by.groupitem",
            "sf.query.group-by.all",
            "sf.query.group-by-cube",
            "sf.query.group-by-rollup",
            "sf.query.group-by-grouping-sets",
        ),
        "the grouping items",
    ),
    "HAVING": ("TRUE", ("sf.query.having",), "the HAVING predicate"),
    "QUALIFY": ("TRUE", ("sf.query.qualify.select-from-qualify",), "the QUALIFY predicate"),
    "ORDER BY": (
        "1",
        ("sf.query.order-by.orderitem", "sf.query.order-by.all"),
        "the ordering items",
    ),
    "CONNECT BY": ("PRIOR c1 = c2", ("sf.query.connect-by",), "the hierarchy condition"),
    "LIMIT": ("1", ("sf.query.limit.limit",), "the LIMIT count"),
    "AT": ("( OFFSET => -60 )", ("sf.query.at-before",), "the time travel point"),
    "BEFORE": ("( OFFSET => -60 )", ("sf.query.at-before",), "the time travel point"),
    "PIVOT": (
        "( SUM ( c1 ) FOR c2 IN ( 1 ) )",
        ("sf.query.pivot",),
        "the PIVOT body",
    ),
    "CHANGES": (
        "( INFORMATION => DEFAULT ) AT ( OFFSET => -60 )",
        ("sf.query.changes",),
        "the change tracking clause",
    ),
    "LATERAL": (
        "FLATTEN ( INPUT => ARRAY_CONSTRUCT ( 1 ) )",
        ("sf.query.join-lateral",),
        "the lateral view",
    ),
    "MATCH_RECOGNIZE": (
        "( PATTERN ( a ) DEFINE a AS TRUE )",
        ("sf.query.match-recognize.match-recognize-pattern-define-as",),
        "the MATCH_RECOGNIZE body",
    ),
    "UNPIVOT": ("( v FOR n IN ( c1 ) )", ("sf.query.unpivot", "sf.query.pivot"), "the body"),
    "VALUES": ("( 1 )", ("sf.query.values",), "the VALUES rows"),
    "SAMPLE": ("( 10 )", ("sf.query.sample",), "the sampling clause"),
    "RESAMPLE": (
        "( USING ts INCREMENT BY INTERVAL '1 hour' )",
        ("sf.query.resample",),
        "the RESAMPLE body",
    ),
    "UPDATE": ("", ("sf.query.for-update",), "the FOR UPDATE options"),
    "SEMANTIC_VIEW": (
        "sv1 DIMENSIONS d1",
        ("sf.query.semantic-view",),
        "the semantic view arguments",
    ),
    "ASOF JOIN": (
        "t2 MATCH_CONDITION ( t1.c1 >= t2.c1 )",
        ("sf.query.asof-join",),
        "the joined source and its match condition",
    ),
    "AND": ("t1.c1 = t2.c1", (), "further ON equalities; this form's own repetition"),
    "AS": ("", (), "repetition of the preceding aliased list item; this form's own list"),
}
TOP_LEVEL_FAMILIES: tuple[str, ...] = (
    "functions",
    "dml",
    "transaction_session",
    "utility",
    "ddl",
    "security_admin",
    "classes",
)
QUERY_FILLS: dict[str, str] = {rule: fill for rule, (fill, _, _) in QUERY_ELISIONS.items()}
QUERY_PLACEHOLDER_FILLS: dict[str, str] = {
    "left_table": "t1",
    "right_table": "t2",
    "left_table.timecol": "t1.c1",
    "right_table.timecol": "t2.c1",
    "table.col": "t1.c1",
    "timestamp": "TO_TIMESTAMP_LTZ ( '2026-01-01' )",
    "time_difference": "-60",
    "id": "'01c00000-0000-0000-0000-000000000000'",
    "data_source": "t1",
    "wait_time": "10",
    "left_hand_table_expression": "t1",
    "inline_view": "SELECT 1",
    "object_ref1": "t1",
    "object_ref2": "t2",
    "symbol": "a",
    "aggregate_function": "SUM",
    "value_column": "c1",
    "pivot_value_1": "1",
    "pivot_value_2": "2",
    "object_reference": "t1",
    "time_series_constant": "INTERVAL '1 hour'",
    "col_position": "1",
    "level_expression": "LEVEL",
    "symbols_and_operators": "a",
    "position": "1",
    "namespace": "d1",
    "object_state": "OFFSET => -60",
    "change_tracking_type": "INFORMATION => DEFAULT",
    "table_function": "TABLE ( FLATTEN ( INPUT => ARRAY_CONSTRUCT ( 1 ) ) )",
    "subquery": "SELECT 1",
    "stage_name": "s1",
    "format_name": "f1",
    "regex_pattern": "'.*'",
    "base_table_name": "t1",
    "anchor_column_list": "c1",
    "recursive_column_list": "c1",
    "probability": "10",
    "num": "10",
    "seed": "1",
}
QUERY_CLAUSES: dict[str, str] = {"[ , ( ... ) ]": ", ( 2 )"}
QUERY_FRAGMENT_PREFIXES: dict[str, str] = {
    "sf.query.asof-join": "SELECT * ",
    "sf.query.resample": "SELECT * ",
    "sf.query.qualify.qualify": "SELECT 1 FROM t1 ",
    "sf.query.match-recognize.match-recognize-pattern-define-as": "SELECT * FROM t1 ",
    "sf.query.semantic-view": "SELECT * FROM ",
}
QUERY_STYLES: dict[str, FormStyle] = {
    "sf.query.": FormStyle(
        prefix="",
        suffix="",
        fills=QUERY_PLACEHOLDER_FILLS,
        clauses=QUERY_CLAUSES,
        elisions=QUERY_FILLS,
    ),
    **{
        form: FormStyle(
            prefix=prefix,
            suffix="",
            fills=QUERY_PLACEHOLDER_FILLS,
            clauses=QUERY_CLAUSES,
            elisions=QUERY_FILLS,
        )
        for form, prefix in QUERY_FRAGMENT_PREFIXES.items()
    },
}
EXPRESSIONS_FILLS: dict[str, str] = {
    "a": "1",
    "b": "1",
    "expr": "1",
    "array_expr": "ARRAY_CONSTRUCT ( 1 , 2 )",
    "index": "0",
    "object_expr": "OBJECT_CONSTRUCT ( 'k' , 1 )",
    "key": "k1",
    "element_type": "INT",
    "key_type": "VARCHAR",
    "value_type": "INT",
    "precision": "9",
    "fractional_seconds_precision": "3",
    "scale": "2",
    "length": "10",
    "dimension": "3",
    "address": "'192.0.2.1'",
    "period_string": "'[2026-01-01, 2026-01-02)'",
    "string": "'2026-01-01'",
    "model_name": "m1",
    "method_name": "predict",
    "version_or_alias_name": "v1",
    "table_name": "t1",
    "input_table": "t1",
    "alias_1": "a1",
    "alias_2": "a2",
    "table_function": "FLATTEN",
    "arg_1": "INPUT => ARRAY_CONSTRUCT ( 1 )",
    "arg": "1",
    "column_name": "c1",
    "user_defined_type_name": "u1",
    "query": "SELECT 1",
    "sql_statement_1": "SELECT 1",
    "sql_statement_2": "SELECT 2",
    "sql_statement_n": "SELECT 3",
    "variable_name": "v1",
    "n": "1",
    "string_literal": "'t1'",
    "session_variable": "$v1",
    "bind_variable": "?",
    "snowflake_scripting_variable": ":v1",
}
EXPRESSIONS_ELISIONS: dict[str, tuple[str, tuple[str, ...], str]] = {
    "SELECT": (
        "*",
        ("sf.query.select.select", "sf.query.select.select.selecting-specific-columns"),
        "the select list",
    ),
    "MODEL": ("1", (), "the method's arguments, which depend on the model"),
}
FAMILY_ELISIONS: dict[str, dict[str, tuple[str, tuple[str, ...], str]]] = {
    "query": QUERY_ELISIONS,
    "expressions": EXPRESSIONS_ELISIONS,
}
EXPRESSIONS_TYPE_FORMS: tuple[str, ...] = (
    "data-types-datetime.date",
    "data-types-datetime.datetime",
    "data-types-datetime.time",
    "data-types-datetime.timestamp",
    "data-types-datetime.timestamp-variations",
    "data-types-datetime.interval-yearmonthqualifier-daytimequalifier",
    "data-types-logical",
    "data-types-numeric.decfloat",
    "data-types-numeric.decimal",
    "data-types-numeric.double",
    "data-types-numeric.float",
    "data-types-numeric.int",
    "data-types-numeric.number",
    "data-types-period",
    "data-types-semistructured.array",
    "data-types-semistructured.object",
    "data-types-semistructured.variant",
    "data-types-structured.array",
    "data-types-structured.map",
    "data-types-structured.object",
    "data-types-text.binary",
    "data-types-text.char",
    "data-types-text.string",
    "data-types-text.varbinary",
    "data-types-text.varchar",
    "data-types-unknown",
    "data-types-unstructured",
    "data-types-user-defined",
    "data-types-vector",
)
EXPRESSIONS_COLUMN_FORMS: tuple[str, ...] = ("data-types-ipaddress", "data-types-uuid")
EXPRESSIONS_FROM_FORMS: tuple[str, ...] = ("literals-table", "identifier-literal")
EXPRESSIONS_SELECT_FORMS: tuple[str, ...] = (
    "data-types-datetime.date-time-constant",
    "data-types-datetime.interval",
    "data-types-datetime.interval.interval-constants",
    "data-types-ipaddress.typed-literal",
    "data-types-period.typed-literal",
    "data-types-semistructured.array-index-access",
    "data-types-semistructured.object-key-access",
)
EXPRESSIONS_STATEMENT_FORMS: tuple[str, ...] = (
    "commands-model-function.model-default-version",
    "commands-model-function.model-version-or-alias",
    "functions-table",
    "snowflake-db-classes",
    "operators-flow",
)
EXPRESSIONS_FORM_FILLS: dict[str, dict[str, str]] = {
    "data-types-datetime.interval": {"string": "'1'"},
    "snowflake-db-classes": {"method_name": "i1!m1"},
}


EXPRESSIONS_ELISION_FILLS: dict[str, str] = {
    rule: fill for rule, (fill, _, _) in EXPRESSIONS_ELISIONS.items()
}
EXPRESSIONS_FORM_CONTEXTS: dict[str, tuple[str, str]] = {
    "": ("SELECT ", " ;"),
    **dict.fromkeys(EXPRESSIONS_TYPE_FORMS, ("SELECT CAST ( NULL AS ", " ) ;")),
    **dict.fromkeys(EXPRESSIONS_COLUMN_FORMS, ("CREATE OR REPLACE TEMPORARY TABLE t1 ( ", " ) ;")),
    **dict.fromkeys(EXPRESSIONS_FROM_FORMS, ("SELECT * FROM ", " ;")),
    **dict.fromkeys(EXPRESSIONS_STATEMENT_FORMS, ("", "")),
    **dict.fromkeys(EXPRESSIONS_SELECT_FORMS, ("SELECT ", " ;")),
}
EXPRESSIONS_STYLES: dict[str, FormStyle] = {
    f"sf.expressions.{form}": FormStyle(
        prefix=prefix,
        suffix=suffix,
        fills={**EXPRESSIONS_FILLS, **EXPRESSIONS_FORM_FILLS.get(form, {})},
        elisions=EXPRESSIONS_ELISION_FILLS,
        terminate=not suffix,
    )
    for form, (prefix, suffix) in EXPRESSIONS_FORM_CONTEXTS.items()
}
FAMILY_STYLES: dict[str, dict[str, FormStyle]] = {
    "scripting": SCRIPTING_SELF_DECLARING_BLOCKS,
    "query": QUERY_STYLES,
    "expressions": EXPRESSIONS_STYLES,
    **{family: {f"sf.{family}.": FormStyle(prefix="", suffix="")} for family in TOP_LEVEL_FAMILIES},
}
