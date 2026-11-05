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
VALUE_TOKENS: frozenset[str] = frozenset({TOKEN_KEYWORD, TOKEN_LITERAL})
TOKEN_BRACKET: str = "bracket"
OPEN_OPTIONAL: str = "["
CLOSE_OPTIONAL: str = "]"
OPEN_CHOICE: str = "{"
CLOSE_CHOICE: str = "}"
BAR: str = "|"
NAMED_ARGUMENT: str = "=>"
ASSIGNMENT_SIZE: int = 2
ASSIGNMENTS: frozenset[str] = frozenset({"=", NAMED_ARGUMENT})
ELISION: str = "..."
OPEN_PAREN: str = "("
CLOSE_PAREN: str = ")"
PARENS: frozenset[str] = frozenset({OPEN_PAREN, CLOSE_PAREN})
ARROWS: frozenset[str] = frozenset({"->", "->>"})
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
    r"|(?P<bracket>[\[\]{}]|\|(?!\|))|(?P<keyword>[A-Za-z_][A-Za-z0-9_$]*|\d+[A-Za-z][A-Za-z0-9_]*)"
    r"|(?P<punct>:=|::|=>|->>|->|<>|!=|<=|>=|\|\||\S))"
)
CLASS_RULES: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"_(declaration|assignment|definition|handler)$"), "subgrammar"),
    (re.compile(r"^(session|bind|snowflake_scripting)_variable$"), "operand"),
    (re.compile(r"^select_statement$"), "query"),
    (re.compile(r"^(reference|array_construct)_statement$"), "expression"),
    (re.compile(r"statement"), "statement"),
    (re.compile(r"(^|_)(sub)?query$"), "query"),
    (re.compile(r"^workload_type$"), "operand"),
    (re.compile(r"^(object|entity)_type$"), "object_kind"),
    (re.compile(r"(^|_)(data_)?type$|_type$"), "data_type"),
    (re.compile(r"number$|^n$|^num|_count$|^count$|position$"), "integer_literal"),
    (re.compile(r"message|string|pattern|comment"), "string_literal"),
    (re.compile(r"^value_column$"), "identifier"),
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
    "object_kind": "'TABLE'",
    "statement": "1",
    "query": "1",
}
LEXICAL_WRONG: dict[str, str] = {
    "identifier": "a string literal",
    "integer_literal": "a string literal",
    "string_literal": "an unquoted identifier",
    "data_type": "a string literal",
    "object_kind": "a string literal",
    "statement": "a bare literal",
    "query": "a bare literal",
}
OBSERVED_WRONG: frozenset[str] = frozenset({"string_literal"})
SUBGRAMMAR_CLASS: str = "subgrammar"
EXPRESSION_ARGUMENT_FAMILIES: frozenset[str] = frozenset({"functions", "classes"})
ARGUMENT_STARTS: frozenset[str] = frozenset({"(", ",", "=>", "=", "BY"})
OPERAND_KEYWORDS: frozenset[str] = frozenset({"LIKE", "ILIKE", "RLIKE", "REGEXP", "ESCAPE"})
CALLEE_STARTS: str = "<_$"
OPTION_ASSIGNMENTS: frozenset[str] = frozenset({"="})
PATH_SEPARATORS: frozenset[str] = frozenset({"/"})
CALL_SEPARATORS: frozenset[str] = frozenset({"(", ","})
CLAUSE_CALLEES: frozenset[str] = frozenset(
    {
        "OVER",
        "GROUP",
        "FILTER",
        "WITHIN",
        "TABLE",
        "IN",
        "BY",
        "AS",
        "USING",
        "VALUES",
        "ON",
        "ANY",
        "ALL",
        "TAG",
    }
)
DANGLING_CALL: re.Pattern[str] = re.compile(
    r"\( ,|, ,|, \)|=> ,|=> \)|=> $|, $|\( $|\( =>|, =>"
    r"|[(,] (?:AS|FROM|IN|FOR|USING|BY|ON)\b|[(,] \.|[-+*/%|&<>=] [,)]"
)
ARITY_CLASS: str = "arity"
ARITY_REASON: str = (
    "D97 extension (auditor 11406dcd, coordinator): an omission that leaves a well-formed "
    "ordinary call, such as VAR_POP ( ) or BITXOR ( 1 ), is an arity error found when the "
    "function name is resolved, not a syntax error; withdrawn from the syntax score and kept "
    "as unscored observe cases in arity/<family>.jsonl"
)
ARGUMENT_CLASS_REASON: str = (
    "D97: a whole argument of an ordinary function call is an <expr> to the parser, so a wrong "
    "class there (an identifier, literal or query where the page names another) is a type or "
    "semantic question, not syntax, and is not lexically distinct under D82; class negatives "
    "stay where the slot is not an expression (keywords, types, @stage names, ON column lists, "
    "qualified-name parts, statement identifiers)"
)
WITHDRAWN_HEADER: str = (
    "# Derived rejections of family {family} withdrawn because the documented grammar also\n"
    "# renders their SQL as valid; generated by scripts/build_syntax_obligations.py."
)
WITHDRAWN_CLASS_HEADER: str = (
    "\n# Each [[withdrawn_class]] withdraws one class of derived negatives by decision, once,"
    " with its count."
)
NON_CALL_KEYWORDS: frozenset[str] = frozenset(
    {"ON", "AS", "IN", "USING", "VALUES", "FROM", "WHERE", "SELECT", "AND", "OR", "NOT"}
)

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
    "object_kind": "TABLE",
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
    "documented",
    "also_documented",
)
HAND_DOCUMENTED: str = "documented"
HAND_ALSO_DOCUMENTED: str = "also_documented"
EXPECT_ACCEPT: str = "accept"
EXPECT_REJECT: str = "reject"
EXPECT_OBSERVE: str = "observe"
REJECTING: frozenset[str] = frozenset({"reject", EXPECT_OBSERVE})
EXPECT_TERMINATES: str = "terminates"
HAND_SIZE: str = "size"
REPEAT_SLOT: str = "{n}"
LATER_ERROR_STAGES: frozenset[str] = frozenset({"", "semantic", "runtime", "error"})
LABEL_SLOT: str = "{label}"
BUILTIN_NAMES: frozenset[str] = frozenset(
    {"STATEMENT_ERROR", "EXPRESSION_ERROR", "OTHER", "SQLCODE", "SQLERRM", "SQLSTATE"}
)
CASE_CAPTURE: str = "execute"
CASE_CAPTURE_DCM: str = "dcm_definition_file"
CASE_CAPTURE_RUN_DATABASE: str = "execute_ddl_in_run_database"
RUN_DATABASE_SCHEMAS: tuple[str, ...] = (
    "CREATE SCHEMA <run_database>.CASES",
    "USE SCHEMA <run_database>.CASES",
    "CREATE SCHEMA <run_database>.<case_schema>",
    "USE SCHEMA <run_database>.<case_schema>",
)
RUN_DATABASE_SETUPS: dict[str, tuple[str, ...]] = {
    "ddl": (
        "CREATE DATABASE <run_database> COMMENT = 'sqlquack ddl sweep <run_nonce>'",
        *RUN_DATABASE_SCHEMAS,
    ),
    "security_admin": (
        "CREATE DATABASE <run_database> COMMENT = 'sqlquack d103 sweep <run_nonce>'",
        *RUN_DATABASE_SCHEMAS,
    ),
    "classes": (
        "CREATE DATABASE <run_database> COMMENT = 'sqlquack d103 sweep <run_nonce>'",
        *RUN_DATABASE_SCHEMAS,
    ),
}
HELD_RUN_DATABASE_SETUP: tuple[str, ...] = RUN_DATABASE_SETUPS["security_admin"]
SHARED_SCHEMA_SETUP: str = "-- shared schema;"
COUNTED_RUN_STATUSES: frozenset[str] = frozenset({"imported", "reconciled"})
DCM_DEFINITION_FORM: re.Pattern[str] = re.compile(r"\.create-[a-z0-9-]+\.define-")
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
DOLLAR_QUOTE: str = "$$"
DOLLAR_BODY: re.Pattern[str] = re.compile(r"\$\$[^$]+\$\$")
DOLLAR_BODY_KEY: str = "$$m$$"
SINGLE_QUOTE: str = "'"
SINGLE_CHARACTER_LITERAL_LENGTH: int = 3
MIN_INTERACTION_OPTIONS: int = 2
PAIR_SIZE: int = 2
DIGEST_WIDTH: int = 12
MEASURE_KEY: str = "measure"
STAGE_KEY: str = "stage"
# D133 (D97b, amended by D133a): an observed, undocumented, non-extreme case settles from its
# receipts when every current receipt for its SQL, setup and session agrees on phase and errno.
D133_REJECT_ERRNOS: frozenset[str] = frozenset({"921"})
D133_NEVER_ACCEPT_ERRNO: str = "1003"
D133_ACCEPT_STAGES: dict[str, str] = {
    "accepted": "",
    "semantic_error": "semantic",
    "runtime_error": "runtime",
    "unsupported_feature": "error",
}
D133_MARKS: dict[str, str] = {
    "accept": "settled accept (D133)",
    "reject": "settled reject (D133)",
}
D113_MARK: str = "documented (D113)"
MARKS_DIR: str = "marks"
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
OPENED: int = -2
# An all-optional property list after SET renders one property: Snowflake rejects `SET ;` (D118).
SET_KEYWORDS: frozenset[str] = frozenset({"SET"})
SET_ANCHOR_PREFERENCE: tuple[str, ...] = ("COMMENT", "ENABLED")
LIST_ITEM_KEYWORDS: frozenset[str] = frozenset(
    {"AS", "NOT", "NULL", "ASC", "DESC", "NULLS", "FIRST", "LAST", "DEFAULT"}
)
# A documented `@` before a stage fill that carries its own `@` renders one (D118).
DOUBLED_STAGE_SIGIL: re.Pattern[str] = re.compile(r"(?<![^ ])@ (?=@[^ @])")
GLUED_PREFIXES: re.Pattern[str] = re.compile(r"(?<![^ ])([$:@]) (?![a-z]+:/)(?=[^ =])")
STAGE_TOKEN: str = r'(@(?:"[^"]*"|[^\s"])*'
STAGE_GLUES: tuple[re.Pattern[str], ...] = (
    re.compile(STAGE_TOKEN + r") (?=[./%~])"),
    re.compile(STAGE_TOKEN + r"[.%]) (?![a-z]+:)(?=\S)"),
    re.compile(STAGE_TOKEN + r"~) (?=/)"),
    re.compile(STAGE_TOKEN + r"/) (?![a-z]+:)(?=[a-z0-9\"'])"),
)
STAGE_KEYWORD: re.Pattern[str] = re.compile(r"(?<![^ ])(@[^ ]*?[%.]|@)([A-Z][A-Z0-9_]*)(?=[ ;]|$)")
EMPTY_PROPERTY_LIST: re.Pattern[str] = re.compile(r"(?<!STARTS )\b(?:SET|UNSET|WITH) ;$")
EMPTY_CLAUSE: re.Pattern[str] = re.compile(r"\bON ;$")
EMPTY_LIST: re.Pattern[str] = re.compile(r"= \( \)")
GLUED_INFIX_RIGHT: re.Pattern[str] = re.compile(r"(?<![^ ])! (?=[A-Za-z_\"'])")
GLUED_INFIX_LEFT: re.Pattern[str] = re.compile(r"(?<=[\w\"')]) !(?=[ A-Za-z_\"'])")
QUERY_FORMS: tuple[str, ...] = ("sf.query.constructs",)
RECEIPT_UNPACKED_FORMS: tuple[str, ...] = (
    "sf.ddl.alter-iceberg-table.datagovnpolicytagaction",
    "sf.ddl.alter-agent.set",
    "sf.ddl.alter-alert.unset",
    "sf.ddl.alter-application-package.set",
    "sf.ddl.alter-application-package.unset",
    "sf.ddl.alter-application.set-shared-telemetry-events",
    "sf.ddl.alter-application.unset",
    "sf.ddl.alter-compute-pool.set",
    "sf.ddl.alter-cortex-search.set",
    "sf.ddl.alter-database-catalog-linked.set",
    "sf.ddl.alter-database.set",
    "sf.ddl.alter-database.unset-data-retention-time-in-days-max-data-extension-time-in-days-stage-undrop-retention-time-in-days",
    "sf.ddl.alter-dbt-project.unset",
    "sf.ddl.alter-dcm-project.unset",
    "sf.ddl.alter-dynamic-table.datagovnpolicytagaction",
    "sf.ddl.alter-dynamic-table.searchoptimizationaction",
    "sf.ddl.alter-dynamic-table.set",
    "sf.ddl.alter-dynamic-table.set-unset-comment",
    "sf.ddl.alter-dynamic-table.unset",
    "sf.ddl.alter-external-table.add-partition-location",
    "sf.ddl.alter-external-table.drop-partition-location",
    "sf.ddl.alter-iceberg-table.set",
    "sf.ddl.alter-iceberg-table.unset",
    "sf.ddl.alter-listing.listing",
    "sf.ddl.alter-materialized-view.rename-to-cluster-by",
    "sf.ddl.alter-model-add-version.internalstage",
    "sf.ddl.alter-schema.set",
    "sf.ddl.alter-schema.unset-data-retention-time-in-days-max-data-extension-time-in-days-stage-undrop-retention-time-in-days",
    "sf.ddl.alter-sequence.sequence",
    "sf.ddl.alter-streamlit.push",
    "sf.ddl.alter-table-column.datagovnpolicytagaction",
    "sf.ddl.alter-table-column.drop-default-set-default",
    "sf.ddl.alter-table-event-table.set",
    "sf.ddl.alter-table.constraintaction",
    "sf.ddl.alter-table.datagovnpolicytagaction",
    "sf.ddl.alter-table.datametricfunctionaction",
    "sf.ddl.alter-table.set",
    "sf.ddl.alter-tag.set",
    "sf.ddl.alter-tag.unset-data-movement-policy",
    "sf.ddl.alter-task.set",
    "sf.ddl.alter-task.unset",
    "sf.ddl.alter-view.datagovnpolicytagaction",
    "sf.ddl.alter-view.datametricfunctionaction",
    "sf.ddl.alter-view.set",
    "sf.ddl.alter-view.unset",
    "sf.ddl.create-artifact-repository",
    "sf.ddl.create-database-catalog-linked",
    "sf.ddl.create-database.create-database",
    "sf.ddl.create-dataset",
    "sf.ddl.create-dynamic-table.create-dynamic-iceberg-table",
    "sf.ddl.create-dynamic-table.create-dynamic-table-target-lag",
    "sf.ddl.create-dynamic-table.create-or-alter-dynamic-iceberg-table",
    "sf.ddl.create-event-table.table",
    "sf.ddl.create-event-table.table-clone",
    "sf.ddl.create-external-agent",
    "sf.ddl.create-hybrid-table.table",
    "sf.ddl.create-iceberg-table-rest.table-as-select",
    "sf.ddl.create-iceberg-table-rest.table-masking-policy-as",
    "sf.ddl.create-iceberg-table.table",
    "sf.ddl.create-iceberg-table.table-base-location.parquet-files",
    "sf.ddl.create-iceberg-table.table.iceberg-rest-in-a-catalog-linked-database",
    "sf.ddl.create-interactive-table.as",
    "sf.ddl.create-materialized-view",
    "sf.ddl.create-model.internalstage",
    "sf.ddl.create-procedure.create-procedure-returns-table",
    "sf.ddl.create-procedure.create-procedure-returns-table-2",
    "sf.ddl.create-procedure.create-procedure-returns-table-3",
    "sf.ddl.create-procedure.create-procedure-returns-table.java-handler",
    "sf.ddl.create-procedure.create-procedure-returns-table.python-handler",
    "sf.ddl.create-procedure.create-procedure-returns-table.scala-handler",
    "sf.ddl.create-procedure.create-procedure-returns-table.snowflake-scripting-handler",
    "sf.ddl.create-semantic-view.create-or-alter-semantic",
    "sf.ddl.create-semantic-view.create-semantic-view-tables",
    "sf.ddl.create-service",
    "sf.ddl.create-table-constraint.alter-table-add-column",
    "sf.ddl.create-table-constraint.alter-table-add-outoflineuniquepk",
    "sf.ddl.create-table-constraint.create-table",
    "sf.ddl.create-table-constraint.create-table-inlineuniquepk-inlinefk",
    "sf.ddl.create-table.create-or-alter-table",
    "sf.ddl.create-table.create-table",
    "sf.ddl.create-view.create-or-alter-view",
    "sf.ddl.create-view.create-view-as",
    "sf.ddl.desc-function-dmf",
    "sf.ddl.desc-function-spcs",
    "sf.ddl.desc-transaction",
    "sf.ddl.drop-iceberg-table",
    "sf.ddl.show-cortex-search",
    "sf.ddl.show-nodes-compute-pool",
    "sf.ddl.show-rules-in-event-routing-table",
    "sf.ddl.show-warehouses",
    "sf.ddl.truncate-materialized-view",
    "sf.dml.copy-into-location",
    "sf.dml.copy-into-table.internalstage-externalstage-externallocation",
    "sf.dml.copy-into-table.select-from-internalstage-externalstage",
    "sf.dml.list.internalstage-externalstage",
    "sf.dml.remove",
    "sf.expressions.data-types-datetime.interval-yearmonthqualifier-daytimequalifier",
    "sf.functions.accumulate",
    "sf.functions.cast.cast-as",
    "sf.functions.get-path.colon.syntax",
    "sf.functions.system-cke-hash-function",
    "sf.functions.system-explain-plan-json",
    "sf.functions.system-show-budget-shared-resource-candidates",
    "sf.functions.to-query",
    "sf.query.connect-by",
    "sf.query.from",
    "sf.query.select.select.selecting-specific-columns",
    "sf.transaction_session.show-parameters",
    "sf.utility.call-with.table-language-java-runtime-version",
    "sf.utility.call-with.table-language-java-runtime-version.java",
    "sf.utility.call-with.table-language-python-runtime-version",
    "sf.utility.call-with.table-language-python-runtime-version.python",
    "sf.utility.call-with.table-language-scala-runtime-version",
    "sf.utility.call-with.table-language-scala-runtime-version.scala",
    "sf.utility.execute-dbt-project.project",
    "sf.utility.execute-dbt-project.project-2",
    "sf.utility.execute-dcm-project.deploy-fromsource-fromdeployment",
    "sf.utility.execute-dcm-project.plan-fromsource-fromdeployment",
    "sf.utility.execute-dcm-project.preview-using-configuration-from",
)
UNPACKED_FORMS: frozenset[str] = frozenset(QUERY_FORMS) | frozenset(RECEIPT_UNPACKED_FORMS)
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
QUERY_FORM_FILLS: dict[str, dict[str, str]] = dict.fromkeys(
    (
        "sf.query.group-by.groupitem",
        "sf.query.group-by-cube",
        "sf.query.group-by-rollup",
        "sf.query.group-by-grouping-sets",
        "sf.query.order-by.orderitem",
    ),
    {"expr": "v1 + 1"},
)
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
    **{
        form: FormStyle(
            prefix="",
            suffix="",
            fills={**QUERY_PLACEHOLDER_FILLS, **fills},
            clauses=QUERY_CLAUSES,
            elisions=QUERY_FILLS,
        )
        for form, fills in QUERY_FORM_FILLS.items()
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
    "snowflake-db-classes": {"method_name": "m1", "instance_name": "i1"},
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
FUNCTIONS_TABLE_FORMS: tuple[str, ...] = (
    "alert-history",
    "application-callback-history",
    "application-specification-status-history",
    "auto-refresh-registration-history",
    "automatic-clustering-history",
    "available-listing-refresh-history",
    "available-listings",
    "bind-values",
    "complete-task-graphs",
    "copy-history",
    "cortex-search-batch",
    "cortex-search-data-scan",
    "cortex-search-refresh-history",
    "cumulative-privacy-losses",
    "current-task-graphs",
    "data-metric-function-expectations",
    "data-metric-function-references.metric-name",
    "data-metric-function-references.ref-entity-name-ref-entity-domain",
    "data-quality-monitoring-expectation-status",
    "data-quality-monitoring-results",
    "data-transfer-history",
    "database-refresh-history",
    "database-refresh-progress.database-refresh-progress",
    "database-refresh-progress.database-refresh-progress-by-job",
    "database-replication-usage-history",
    "database-storage-usage-history",
    "dbt-project-execution-history",
    "dcm-deployment-history",
    "dynamic-table-graph-history",
    "dynamic-table-refresh-history",
    "dynamic-tables",
    "estimate-hybrid-table-storage-usage",
    "estimate-remaining-dp-aggregates",
    "external-functions-history",
    "external-table-files",
    "external-table-registration-history",
    "flatten",
    "generator.generator",
    "generator.generator-rowcount",
    "get-ai-evaluation-data-snowflake-local",
    "get-ai-observability-events-snowflake-local",
    "get-ai-observability-logs-snowflake-local",
    "get-ai-record-trace-snowflake-local",
    "get-contacts",
    "get-job-history",
    "get-lineage-snowflake-core",
    "get-object-references",
    "get-query-operator-stats",
    "get-table-version-refs",
    "get-table-versions",
    "iceberg-table-file-registration-history",
    "iceberg-table-files",
    "iceberg-table-snapshot-refresh-history",
    "infer-schema",
    "listing-refresh-history",
    "login-history.login-history",
    "login-history.login-history-by-user",
    "materialized-view-refresh-history",
    "my-budget-summary-snowflake-local",
    "network-rule-references.container-name-container-type",
    "network-rule-references.network-rule-name",
    "notification-history",
    "online-feature-table-refresh-history",
    "pipe-usage-history",
    "policy-references.policy-name",
    "policy-references.policy-name-policy-kind",
    "policy-references.ref-entity-name-ref-entity-domain",
    "query-acceleration-history",
    "query-history.query-history",
    "query-history.query-history-by-session",
    "query-history.query-history-by-user",
    "query-history.query-history-by-warehouse",
    "replication-group-dangling-references",
    "replication-group-lag-history",
    "replication-group-refresh-history.replication-group-refresh-history",
    "replication-group-refresh-history.replication-group-refresh-history-all",
    "replication-group-refresh-progress.replication-group-refresh-progress",
    "replication-group-refresh-progress.replication-group-refresh-progress-all",
    "replication-group-refresh-progress.replication-group-refresh-progress-by-job",
    "replication-group-usage-history",
    "replication-usage-history",
    "rest-event-history",
    "result-scan",
    "search-optimization-history",
    "semantic-view-materialization-refresh-history",
    "serverless-alert-history",
    "serverless-task-history",
    "spcs-get-events",
    "spcs-get-logs",
    "spcs-get-metrics",
    "split-to-table",
    "stage-directory-file-registration-history",
    "stage-storage-usage-history",
    "storage-lifecycle-policy-history.policy-name",
    "storage-lifecycle-policy-history.ref-entity-name-ref-entity-domain",
    "strtok-split-to-table",
    "system-data-metric-scan",
    "system-evaluate-data-quality-expectations",
    "system-stream-backlog",
    "tag-references",
    "tag-references-all-columns",
    "tag-references-with-lineage",
    "task-dependents",
    "task-graph-changes",
    "task-history",
    "to-query",
    "validate",
    "validate-pipe-load",
    "warehouse-load-history",
    "warehouse-metering-history",
)
FUNCTIONS_STATEMENT_FORMS: tuple[str, ...] = (
    "ai-function-evaluation.create-experiment-type-from",
    "ai-function-evaluation.execute-experiment",
    "ai-function-optimization.create-experiment-type-from",
    "dmf-row-count-difference",
    "dmf-seconds-since-row-count-change",
    "dmf-source-target-lag.alter-table-add-data",
    "policy-context",
    "prompt",
    "set-sys-context",
    "system-create-evaluation-dataset",
    "system-evaluate-data-quality-expectations-persist-result",
    "system-get-classification-result",
    "system-global-account-set-parameter",
    "system-set-row-timestamp-on-all-supported-tables.select-system-set-row-timestamp-on-all-supported-tables",
    "system-wait-for-openflow-connector-status",
    "system-wait-for-openflow-deployment-status",
    "system-wait-for-openflow-runtime-status",
    "system-wait-for-stable-openflow-connectors",
    "system-wait-for-stable-openflow-deployments",
    "system-wait-for-stable-openflow-runtimes",
)
FUNCTIONS_METRIC_ON_FORMS: tuple[str, ...] = (
    "dmf-accepted-values",
    "dmf-accepted-values-percent",
    "dmf-referential-integrity-count.table",
    "dmf-referential-integrity-count.table.syntax",
)
FUNCTIONS_FILLS: dict[str, str] = {
    "window_frame": "ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW",
    "orderby_clause": "ORDER BY 1",
    "order_by_expr": "1",
    "partition_expr": "1",
    "date_or_time_part": "DAY",
    "query": "SELECT 1",
    "subquery": "SELECT 1",
    "service_name": "svc1",
    "windowdefinition": "PARTITION BY 1 ORDER BY 1",
    "fixedrangeframe": "ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW",
    "function": "SUM",
    "arguments": "1",
    "lambda_expression": "x -> x > 0",
    "lambda-expression": "x -> x > 0",
    "initialize_lambda": "( ) -> 0",
    "accumulate_lambda": "( s , x ) -> s + x",
    "combine_lambda": "( s1 , s2 ) -> s1 + s2",
    "terminate_lambda": "s -> s",
    "sql_statement_expression": "'SELECT 1'",
    "sql_query_id_expression": "LAST_QUERY_ID ( )",
    "agent_type": "'CORTEX AGENT'",
    "escape": "'!'",
    "escape_char": "'!'",
}
FUNCTIONS_LITERAL_FILLS: dict[str, str] = {
    "'<sql_statement>'": "'SELECT 1'",
    "'<select_statement>'": "'SELECT 1'",
    "'<agent_name>[!<version>]'": "'a1'",
    "'[<namespace>.]<object_name>'": "'d1.t1'",
    "'{ internalStage | externalStage }'": "'@s1'",
    "'@<stage>'": "'@s1'",
    "'<schema_name>.<object_name>'": "'s1.t1'",
    "'( <col1> [ , <col2> ... ] )'": "'( c1 , c2 )'",
    "'( <expr1> [ , <expr2> ... ] )'": "'( c1 , c2 )'",
    "'<column> [ , <column> ... ]'": "'c1 , c2'",
    "'<object> [, <object> ...]'": "'o1, o2'",
    "'<category>,<category>'": "'c1,c2'",
    "'[ STALENESS_CHECK_OVERRIDE ] [ , OWNERSHIP_TRANSFER_CHECK_OVERRIDE ]'": (
        "'STALENESS_CHECK_OVERRIDE'"
    ),
    "'/<dbName>/<schemaName>/<repositoryName>'": "'/d1/s1/r1'",
    "'<email_address_1> [ , ... <email_address_N> ]'": "'a@example.com'",
    "'<dbName>.<schemaName>.<tableName>'": "'d1.s1.t1'",
    "'<email_address_list>'": "'a@example.com'",
    "'<qualified_table_name>'": "'d1.s1.t1'",
    "'<function_name>'": "'ai_complete'",
    "'<model_name>'": "'llama3.3-70b'",
    "'<service_name>'": "'svc1'",
    "'<container_name>'": "'echo'",
}
FUNCTIONS_FORM_FILLS: dict[str, dict[str, str]] = {
    "reduce": {"lambda_expression": "( acc , x ) -> acc + x"},
}
FUNCTIONS_FORM_CONTEXTS: dict[str, tuple[str, str]] = {
    "": ("SELECT ", " ;"),
    **dict.fromkeys(FUNCTIONS_FORM_FILLS, ("SELECT ", " ;")),
    **dict.fromkeys(FUNCTIONS_TABLE_FORMS, ("SELECT * FROM TABLE ( ", " ) ;")),
    **dict.fromkeys(FUNCTIONS_STATEMENT_FORMS, ("", "")),
    **dict.fromkeys(FUNCTIONS_METRIC_ON_FORMS, ("ALTER TABLE t1 ADD DATA METRIC FUNCTION ", " ;")),
}
FUNCTIONS_STYLES: dict[str, FormStyle] = {
    f"sf.functions.{form}": FormStyle(
        prefix=prefix,
        suffix=suffix,
        fills={**FUNCTIONS_FILLS, **FUNCTIONS_FORM_FILLS.get(form, {})},
        literal_fills=FUNCTIONS_LITERAL_FILLS,
        terminate=not suffix,
        embedded=bool(suffix),
    )
    for form, (prefix, suffix) in FUNCTIONS_FORM_CONTEXTS.items()
}
DML_FILLS: dict[str, str] = {
    "file_url": "file:///tmp/d1/f1.csv",
    "file_directory_url": "file:///tmp/d1/",
    "integer": "1",
    "int_stage_name": "s1",
    "ext_stage_name": "s1",
    "stage_name": "s1",
    "path": "p1",
    "branch_path": "branches/main",
    "repository_clone": "r1",
    "file_col_num": "1",
    "element": "e1",
    "additional_table": "t2",
    "additional_tables": "t2",
    "integration_name": "i1",
    "source": "t2",
    "join_expr": "v1.c1 = t2.c1",
}
DML_LITERAL_FILLS: dict[str, str] = {
    "'<s3_url>'": "'s3://b1/p1/'",
    "'<gcs_url>'": "'gcs://b1/p1/'",
    "'<azure_url>'": "'azure://a1.blob.core.windows.net/c1/p1/'",
    "'<file_format_name_ref>'": "'f1'",
}
SQL_BODY: str = "'BEGIN RETURN 1 ; END'"
VARIABLE_BLOCK_OPEN: str = "DECLARE v1 VARCHAR DEFAULT 'SELECT 1' ; BEGIN "
UTILITY_FILLS: dict[str, str] = {
    "statement": "SELECT 1",
    "specification_text": "$$ m $$",
    "stage": "s1",
    "stage_name": "s1",
    "path": "p1",
    "filename": "f1.sql",
    "yaml_spec_text": "$$ m $$",
    "system_function": "SYSTEM$DBT_GET_LAST_RUN_TARGET ( 'p1' )",
}
UTILITY_LITERAL_FILLS: dict[str, str] = {
    "'com.snowflake:snowpark:<version>'": "'com.snowflake:snowpark:1.14.0'",
    "'com.snowflake:snowpark_<scala_version>:<version>'": "'com.snowflake:snowpark_2.12:1.14.0'",
    "'snowflake-snowpark-python[==<version>]'": "'snowflake-snowpark-python'",
    "'<package_name>[==<version>]'": "'numpy==1.26.4'",
    "'<module_file_name>.<function_name>'": "'m1.run'",
    "'<dbt_args>'": "'run --select m1'",
    "'<notebook_args>'": "'a1 a2'",
    "'<path>/<filename>'": "'p1/f1.sql'",
    "'/<path>/<filename>'": "'/p1/f1.sql'",
    "'./<path>/<filename>'": "'./p1/f1.sql'",
    "'../<path>/<filename>'": "'../p1/f1.sql'",
}
DDL_FILLS: dict[str, str] = {
    **DML_FILLS,
    "rows": "1",
    "patch_num": "1",
    "storage_gb": "10",
    "step_num": "1",
    "start_num": "1",
    "sequence_interval": "1",
    "sequence_num": "1",
    "timestamp_ms": "1",
    "seconds": "60",
    "time_difference": "-60",
    "id": "'q1'",
    "query_id": "'q1'",
    "timestamp": "CURRENT_TIMESTAMP ( )",
    "dml_statement": "INSERT INTO SELF SELECT 1",
    "object_type": "TABLE",
    "workload_type": "USER",
    "copy_statement": "COPY INTO t1 FROM @s1",
    "window_function": "SUM",
    "windowFrameClause": "ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW",
    "scoring_profile": "'{}'",
    "sql": "SELECT 1",
    "procedure_definition": "$$ BEGIN RETURN 1 ; END $$",
    "action": "SELECT 1",
    "boolean": "TRUE",
    "boolean_expr": "TRUE",
    "budget_in_bytes": "1000000",
    "comma_separated_instance_ids": "0 , 2",
    "constraint_properties": "RELY",
    "context_function_1": "CURRENT_USER",
    "context_function_2": "CURRENT_ROLE",
    "git_author_email": "'u1@example.com'",
    "git_username": "'u1'",
    "git_password": "'m'",
    "hour_of_day": "3",
    "instance_id": "0",
    "snapshot_id": "1",
    "transaction_id": "1",
    "width": "10",
    "java_jdk_version": "'11'",
    "python_version": "'3.10'",
    "scala_version": "'2.12'",
    "log_event_level": "INFO",
    "log_level": "INFO",
    "list_of_properties": "ANOMALY_DETECTION = TRUE",
    "manifest": "$$m: 1$$",
    "specification": "'{}'",
    "specification_object": "$$m: 1$$",
    "specification_text": "$$m: 1$$",
    "specification_yaml": "$$m: 1$$",
    "yaml_fragment": "$$m: 1$$",
    "yaml_spec": "$$m: 1$$",
    "object_visibility_spec": "$$m: 1$$",
    "authentication_properties": "ISSUER = 'm' AUDIENCE = 'm'",
    "object_privilege": "USAGE",
    "objectPrivilege": "USAGE",
    "path_to_stage": "'@d1.s1.st1/p1'",
    "path_to_version_directory": "'@d1.s1.st1/p1'",
    "source_location": "'@d1.s1.st1'",
    "yaml_manifest_stage_location": "@d1.s1.st1/p1",
    "prefix": "'p1'",
    "property": "SUSPEND_TASK_AFTER_NUM_FAILURES",
    "default_property": "USER_TASK_TIMEOUT_MS",
    "session_parameter": "LOCK_TIMEOUT",
    "purpose": "STEWARD",
    "region_group": "g1",
    "search_method_with_target": "EQUALITY ( v1 )",
    "arg_data_type_2": "INT",
    "default_target": "'t1'",
    "start_time": "'2026-01-01T00:00:00Z'",
    "endpoint_name": "e1",
    "start_column": "v1",
    "end_column": "v1",
}
REPEAT_FILLS: dict[tuple[str, str], str] = {
    ("integration_type_name", "SECURITY INTEGRATIONS"): "API INTEGRATIONS",
    ("object_type", "DATABASES"): "SHARES",
    ("type", "DATABASES"): "DYNAMIC_TABLES",
    ("session_parameter", "LOCK_TIMEOUT"): "STATEMENT_TIMEOUT_IN_SECONDS",
    ("task_name", "p1"): "p2",
}
ALERT_PAGES: tuple[str, ...] = ("sf.ddl.create-alert", "sf.ddl.alter-alert")
SECURITY_ADMIN_FILLS: dict[str, str] = {
    **DDL_FILLS,
    "client_type": "GO_DRIVER",
    "integration_type": "SECURITY",
    "actions": "SET COMMENT = 'm'",
    "expression_on_arg_name": "TRUE",
    "expression_on_arg_name_to_mask": "'m'",
    "value": "30",
    "arg_1": "v2",
    "arg_type_1": "VARCHAR",
    "arg_type_to_mask": "VARCHAR",
    "consumer_account": "o1.a1",
    "days_integer": "7",
    "idp_parameters": (
        "SAML2_ISSUER = 'm' SAML2_SSO_URL = 'https://example.com/sso' SAML2_PROVIDER = "
        "'OKTA' SAML2_X509_CERT = 'm'"
    ),
    "integration_type_params": (
        "TYPE = OAUTH OAUTH_CLIENT = CUSTOM OAUTH_CLIENT_TYPE = 'CONFIDENTIAL' "
        "OAUTH_REDIRECT_URI = 'https://example.com/cb'"
    ),
    "interface": "DEFAULT",
    "json_object": "'{}'",
    "list_of_interfaces": "'ALL'",
    "schedule": "'60 MINUTE'",
    "oauth_parameters": (
        "OAUTH_CLIENT_ID = 'm' OAUTH_CLIENT_SECRET = 'm' OAUTH_TOKEN_ENDPOINT = "
        "'https://example.com/token' OAUTH_AUTHORIZATION_ENDPOINT = "
        "'https://example.com/auth'"
    ),
    "object_type_plural": "TABLES",
    "policy_rules_yaml": (
        "rules: [ { name: r1 , operations: [ MODIFY_MULTI_PARTY_APPROVAL ] , approvers: { "
        "users: [ 'ADMIN1' ] } } ]"
    ),
    "parameters": "ENABLED = TRUE",
    "parameter": "COMMENT",
    "partner_application": "LOOKER",
    "public_key1": "'MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA'",
    "public_key2": "'MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEB'",
    "queue": (
        "( NAME = 'q1' NOTIFICATION_PROVIDER = AWS_SNS AWS_SNS_TOPIC_ARN = "
        "'arn:aws:sns:us-west-1:12345:t1' )"
    ),
    "region_group_id": "g1",
    "snowflake_region_id": "aws_us_west_2",
    "threshold": "80",
    "user_password": "'m'",
    "yaml-body": "m: 1",
    "yaml_body": "m: 1",
    "yaml_definition": "m: 1",
    "object_type": "TABLE",
    "entity_type": "DATABASE",
    "integration_type_name": "SECURITY INTEGRATIONS",
    **{
        word: word
        for word in (
            "artifact_repository_api",
            "aws_api_gateway",
            "aws_gov_api_gateway",
            "aws_gov_private_api_gateway",
            "aws_private_api_gateway",
            "azure_api_management",
            "external_mcp",
            "git_https_api",
            "google_api_gateway",
            "all",
            "none",
        )
    },
}
SECURITY_ADMIN_PAGE_FILLS: dict[str, dict[str, str]] = {
    "sf.security_admin.create-resource-monitor": {"timestamp": "'2030-01-01 00:00'"},
    "sf.security_admin.alter-resource-monitor": {"timestamp": "'2030-01-01 00:00'"},
    "sf.security_admin.create-aggregation-policy": {
        "body": "AGGREGATION_CONSTRAINT ( MIN_GROUP_SIZE => 5 )"
    },
    "sf.security_admin.alter-aggregation-policy": {
        "expression": "AGGREGATION_CONSTRAINT ( MIN_GROUP_SIZE => 5 )"
    },
    "sf.security_admin.create-join-policy": {"body": "JOIN_CONSTRAINT ( JOIN_REQUIRED => TRUE )"},
    "sf.security_admin.create-privacy-policy": {"body": "PRIVACY_BUDGET ( BUDGET_NAME => 'b1' )"},
    "sf.security_admin.alter-privacy-policy": {
        "expression": "PRIVACY_BUDGET ( BUDGET_NAME => 'b1' )"
    },
    "sf.security_admin.create-projection-policy": {
        "body": "PROJECTION_CONSTRAINT ( ALLOW => TRUE )"
    },
    "sf.security_admin.alter-projection-policy": {
        "expression": "PROJECTION_CONSTRAINT ( ALLOW => TRUE )"
    },
    "sf.security_admin.create-feature-policy": {"type": "DATABASES"},
    "sf.security_admin.create-failover-group": {"object_type": "DATABASES"},
    "sf.security_admin.create-replication-group": {"object_type": "DATABASES"},
    "sf.security_admin.alter-failover-group": {"object_type": "DATABASES"},
    "sf.security_admin.alter-replication-group": {"object_type": "DATABASES"},
    "sf.security_admin.alter-feature-policy": {"type": "DATABASES"},
    "sf.security_admin.alter-join-policy": {
        "expression": "JOIN_CONSTRAINT ( JOIN_REQUIRED => TRUE )"
    },
}
DDL_LITERAL_FILLS: dict[str, str] = {
    **UTILITY_LITERAL_FILLS,
    "'<num> { seconds | minutes | hours | days }'": "'1 minute'",
    "'<num> { HOURS | MINUTES | SECONDS }'": "'1 MINUTES'",
    "'USING CRON <expr> <time_zone>'": "'USING CRON 0 9 * * * UTC'",
    "'USING CRON <cron_spec> <timezone>'": "'USING CRON 0 9 * * * UTC'",
    "'{ AUTO | 16MB | 32MB | 64MB | 128MB }'": "'AUTO'",
    "'<path>/[<filename>]'": "'p1/f1.csv'",
    "'<path>/[<filename>'": "'p1/f1.csv'",
    "'<file_path>'": "'p1/f1.csv'",
    "'<database>.<schema>.<tablename>'": "'d1.s1.t1'",
    "'{ <num> MINUTE | USING CRON <expr> <time_zone> }'": "'1 MINUTE'",
    "'{ <number> MINUTE | USING CRON <expr> <time_zone> }'": "'1 MINUTE'",
    "'XSMALL | SMALL | MEDIUM | LARGE | XLARGE | XXLARGE'": "'XSMALL'",
    "'{ XSMALL | SMALL | MEDIUM | LARGE | XLARGE | XXLARGE }'": "'XSMALL'",
    "'snow://workspace/<workspace_path>'": "'snow://workspace/w1'",
    "'@<database_name>.<schema_name>.<stage_name>'": "'@d1.s1.st1'",
    "'<event_definition'": "'e1'",
    "'<num> MINUTE'": "'1 MINUTE'",
    "'<region_group>.<snowflake_region>'": "'PUBLIC.AWS_US_WEST_2'",
    "'<num> { hours | days }'": "'1 days'",
    "'<num> days'": "'1 days'",
    "'{ S3 | S3CHINA | S3GOV }'": "'S3'",
    "'azure://onelake.blob.fabric.microsoft.com/<workspace_id>/<item_id>/Files[/<path>/]'": (
        "'azure://onelake.blob.fabric.microsoft.com/w1/i1/Files/p1/'"
    ),
    "'s3compat://<bucket>[/<path>/]'": "'s3compat://b1/p1/'",
    "'{string_literal}'": "'m'",
    '"<volume_name>"': '"d1"',
    "'s3compat://{bucket}[/{path}/]'": "'s3compat://b1/p1/'",
    "'<protocol>://<bucket>[/<path>/]'": "'s3://b1/p1/'",
    "'gcs://<bucket>[/<path>/]'": "'gcs://b1/p1/'",
    "'azure://<account>.blob.core.windows.net/<container>[/<path>/]'": (
        "'azure://a1.blob.core.windows.net/c1/p1/'"
    ),
    "'( <purpose> = <contact> )'": "'( p1 = c1 )'",
    "'asset://<asset_path>'": "'asset://a1/'",
    "'{ <num> MINUTE | <num> HOUR | USING CRON <expr> <time_zone> }'": "'1 HOUR'",
    "'https://example.com/<...>'": "'https://example.com/'",
    "'<...>'": "'https://example.com/'",
}
CLASS_METHOD_CALL: str = "CALL "
CLASS_METHOD_SEGMENT: str = ".methods."
CLASSES_WITH_METHODS: tuple[str, ...] = (
    "anomaly-detection",
    "anomaly-insights",
    "budget",
    "classification",
    "classification-profile",
    "custom-classifier",
    "forecast",
    "top-insights",
)
CLASS_METHODS_WITH_CALL: tuple[str, ...] = (
    "sf.classes.budget.methods.activate",
    "sf.classes.budget.methods.deactivate",
)
CLASS_DATA_REFERENCE: str = "SYSTEM$REFERENCE ( 'VIEW' , 'v1' )"
CLASS_TAG_REFERENCE: str = (
    "( SELECT SYSTEM$REFERENCE ( 'TAG' , 'd1.s1.tg1' , 'SESSION' , 'APPLYBUDGET' ) )"
)
CLASSES_FILLS: dict[str, str] = {
    **DDL_FILLS,
    "reference_statement": CLASS_TAG_REFERENCE,
    "array_construct_statement": "ARRAY_CONSTRUCT ( 'm' )",
    "array_of_arguments": "[ 'm' ]",
    "input_data": CLASS_DATA_REFERENCE,
    "reference_to_data_to_analyze": CLASS_DATA_REFERENCE,
    "reference_to_training_data": CLASS_DATA_REFERENCE,
    "config": "OBJECT_CONSTRUCT ( 'service_types' , ARRAY_CONSTRUCT ( ) )",
    "config_object": "OBJECT_CONSTRUCT ( 'evaluate' , FALSE )",
    "configuration_object": "OBJECT_CONSTRUCT ( 'prediction_interval' , 0.95 )",
    "object": "OBJECT_CONSTRUCT ( 'k1' , 'm' )",
    "array": "ARRAY_CONSTRUCT ( OBJECT_CONSTRUCT ( 'category' , 'NAME' ) )",
    "series": "TO_VARIANT ( 'm' )",
    "tag-pairs": f"[ [ {CLASS_TAG_REFERENCE} , 'sales' ] ]",
    "operation_mode": "'UNION'",
    "boolean_value": "TRUE",
    "days": "5",
    "threshold": "75",
    "threshold_percent": "10",
    "warehouse_id": "838",
    "forecasting_periods": "3",
    "regular_expression": "'m'",
    "account_name": "'a1'",
}
CLASS_PROCEDURE_REFERENCE: str = (
    "SYSTEM$REFERENCE ( 'PROCEDURE' , 'd1.s1.p1(STRING)' , 'SESSION' , 'USAGE' )"
)
CLASS_TABLE_REFERENCE: str = (
    "( SELECT SYSTEM$REFERENCE ( 'TABLE' , 't1' , 'SESSION' , 'APPLYBUDGET' ) )"
)
CLASS_AGENT_REFERENCE: str = "( SELECT SYSTEM$REFERENCE ( 'CORTEX AGENT' , 'ag1' ) )"
CLASSES_PAGE_FILLS: dict[str, dict[str, str]] = {
    "sf.classes.budget.methods.add-custom-action": {
        "reference_statement": CLASS_PROCEDURE_REFERENCE
    },
    "sf.classes.budget.methods.set-cycle-start-action": {
        "reference_statement": CLASS_PROCEDURE_REFERENCE
    },
    "sf.classes.budget.methods.add-resource": {"reference_statement": CLASS_TABLE_REFERENCE},
    "sf.classes.budget.methods.remove-resource": {"reference_statement": CLASS_TABLE_REFERENCE},
    "sf.classes.budget.methods.add-resource-tag": {"reference_statement": CLASS_TAG_REFERENCE},
    "sf.classes.budget.methods.remove-resource-tag": {"reference_statement": CLASS_TAG_REFERENCE},
    "sf.classes.budget.methods.add-shared-resource": {"reference_statement": CLASS_AGENT_REFERENCE},
    "sf.classes.budget.methods.remove-shared-resource": {
        "reference_statement": CLASS_AGENT_REFERENCE
    },
    "sf.classes.classification-profile.commands.create-classification-profile": {
        "config_object": "OBJECT_CONSTRUCT ( 'auto_tag' , FALSE )"
    },
}
CLASSES_LITERAL_FILLS: dict[str, str] = {
    **DDL_LITERAL_FILLS,
    "'<email_address> [, <email_address> ... ]'": "'u1@example.com, u2@example.com'",
    "'<email> [ , <email> [ , ... ] ]'": "'u1@example.com, u2@example.com'",
    "'<email_list>'": "'u1@example.com, u2@example.com'",
    "'<refresh_interval>'": "'TIER_1H'",
    "'<anomaly_scope>'": "'ACCOUNT'",
}
CLASS_METHODS_WITH_SELECT: tuple[str, ...] = (
    "sf.classes.classification-profile.methods.describe",
    "sf.classes.custom-classifier.methods.list",
)
FAMILY_STYLES: dict[str, dict[str, FormStyle]] = {
    "scripting": SCRIPTING_SELF_DECLARING_BLOCKS,
    "query": QUERY_STYLES,
    "expressions": EXPRESSIONS_STYLES,
    "functions": FUNCTIONS_STYLES,
    **{family: {f"sf.{family}.": FormStyle(prefix="", suffix="")} for family in TOP_LEVEL_FAMILIES},
    "classes": {
        "sf.classes.": FormStyle(
            prefix="",
            suffix="",
            fills=CLASSES_FILLS,
            literal_fills=CLASSES_LITERAL_FILLS,
            observe_empty_properties=True,
        ),
        **{
            f"sf.classes.{name}{CLASS_METHOD_SEGMENT}": FormStyle(
                prefix=CLASS_METHOD_CALL,
                suffix="",
                fills=CLASSES_FILLS,
                literal_fills=CLASSES_LITERAL_FILLS,
            )
            for name in CLASSES_WITH_METHODS
        },
        **{
            form: FormStyle(
                prefix="", suffix="", fills=CLASSES_FILLS, literal_fills=CLASSES_LITERAL_FILLS
            )
            for form in CLASS_METHODS_WITH_CALL
        },
        **{
            form: FormStyle(
                prefix="SELECT ",
                suffix="",
                fills=CLASSES_FILLS,
                literal_fills=CLASSES_LITERAL_FILLS,
            )
            for form in CLASS_METHODS_WITH_SELECT
        },
        **{
            form: FormStyle(
                prefix=CLASS_METHOD_CALL if CLASS_METHOD_SEGMENT in form else "",
                suffix="",
                fills={**CLASSES_FILLS, **fills},
                literal_fills=CLASSES_LITERAL_FILLS,
                observe_empty_properties=CLASS_METHOD_SEGMENT not in form,
            )
            for form, fills in CLASSES_PAGE_FILLS.items()
        },
    },
    "dml": {
        "sf.dml.": FormStyle(prefix="", suffix="", fills=DML_FILLS, literal_fills=DML_LITERAL_FILLS)
    },
    "ddl": {
        "sf.ddl.": FormStyle(
            prefix="",
            suffix="",
            fills=DDL_FILLS,
            literal_fills=DDL_LITERAL_FILLS,
            observe_empty_properties=True,
        ),
        **{
            prefix: FormStyle(
                prefix="",
                suffix="",
                fills={**DDL_FILLS, "condition": "SELECT 1"},
                literal_fills=DDL_LITERAL_FILLS,
                observe_empty_properties=True,
            )
            for prefix in ALERT_PAGES
        },
    },
    "security_admin": {
        "sf.security_admin.": FormStyle(
            prefix="",
            suffix="",
            fills=SECURITY_ADMIN_FILLS,
            literal_fills=DDL_LITERAL_FILLS,
            observe_empty_properties=True,
        ),
        **{
            prefix: FormStyle(
                prefix="",
                suffix="",
                fills={**SECURITY_ADMIN_FILLS, **fills},
                literal_fills=DDL_LITERAL_FILLS,
                observe_empty_properties=True,
            )
            for prefix, fills in SECURITY_ADMIN_PAGE_FILLS.items()
        },
    },
    "utility": {
        "sf.utility.": FormStyle(
            prefix="", suffix="", fills=UTILITY_FILLS, literal_fills=UTILITY_LITERAL_FILLS
        ),
        "sf.utility.call-with.table-language-sql-as": FormStyle(
            prefix="",
            suffix="",
            fills=UTILITY_FILLS,
            literal_fills={**UTILITY_LITERAL_FILLS, "'<procedure_definition>'": SQL_BODY},
        ),
        "sf.utility.execute-immediate.immediate": FormStyle(
            prefix="",
            suffix="",
            fills=UTILITY_FILLS,
            literal_fills={**UTILITY_LITERAL_FILLS, "'<string_literal>'": "'SELECT 1'"},
        ),
        "sf.utility.execute-immediate.immediate.syntax": FormStyle(
            prefix=VARIABLE_BLOCK_OPEN,
            suffix=BLOCK_CLOSE,
            fills=UTILITY_FILLS,
            literal_fills=UTILITY_LITERAL_FILLS,
        ),
    },
}
SCHEMA_QUALIFIER: str = "d1.s1."
DATABASE_QUALIFIER: str = "d1."
SCHEMA_REFERENCES: tuple[str, ...] = (
    "table_name",
    "ref_table_name",
    "stream_name",
    "network_rule",
    "root_task_name",
    "finalizer_task_name",
    "search_service_name",
    "external_stage_name",
)
DEFINE_QUALIFIED_NAMES: dict[str, tuple[str, str]] = {
    **dict.fromkeys(
        (
            "sf.ddl.create-alert.define-alert",
            "sf.ddl.create-data-metric-function.define-data-metric-function",
            "sf.ddl.create-dynamic-table.define-dynamic-table",
            "sf.ddl.create-file-format.define-file-format",
            "sf.ddl.create-function.define-function",
            "sf.ddl.create-pipe.define-pipe",
            "sf.ddl.create-procedure.define-procedure",
            "sf.ddl.create-semantic-view.define-semantic-view",
            "sf.ddl.create-sequence.define-sequence",
            "sf.ddl.create-stream.define-stream",
            "sf.ddl.create-streamlit.define-streamlit",
            "sf.ddl.create-tag.define-tag",
            "sf.ddl.create-task-graph.define-task-graph",
            "sf.ddl.create-task.define-task",
            "sf.ddl.create-view.define-view",
            "sf.security_admin.create-authentication-policy.define-authentication-policy",
            "sf.security_admin.create-network-rule.define-network-rule",
            "sf.security_admin.create-secret.define-secret",
        ),
        ("name", SCHEMA_QUALIFIER),
    ),
    "sf.ddl.create-table.define-table": ("table_name", SCHEMA_QUALIFIER),
    "sf.ddl.create-stage.define-stage": ("internal_stage_name", SCHEMA_QUALIFIER),
    "sf.ddl.create-schema.define-schema": ("name", DATABASE_QUALIFIER),
    "sf.security_admin.create-database-role.define-database-role": ("name", DATABASE_QUALIFIER),
    "sf.ddl.create-database.define-database": ("", ""),
    "sf.ddl.create-warehouse.define-warehouse": ("", ""),
    "sf.ddl.create-code-bundle.define-code-bundle": ("", ""),
    "sf.security_admin.create-network-policy.define-network-policy": ("", ""),
    "sf.security_admin.create-role.define-role": ("", ""),
    "sf.security_admin.create-share.define-share": ("", ""),
}
# A DEFINE TASK predecessor named like the task itself is a cycle DCM refuses (errno 1629).
DEFINE_FORM_FILLS: dict[str, dict[str, str]] = {
    "sf.ddl.create-task.define-task": {"task_name": "p1"}
}
DEFINE_REFERENCE_CLAUSES: tuple[str, ...] = (
    "AFTER",
    "FINALIZE =",
    "ROOTS = (",
    "FINALIZER =",
    "CORTEX SEARCH SERVICE",
    "REFRESH USING ( DELETE FROM",
)
DEFINE_SECRETS_CLAUSE: str = "SECRETS = ("
DEFINE_REFERENCE_ITEM: str = (
    r"(?:'[^']*'|(?:[A-Za-z_][A-Za-z0-9_$]*\.)*(?:\"[^\"]*\"|[A-Za-z_][A-Za-z0-9_$]*))"
)
