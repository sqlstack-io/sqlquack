"""Pinned DuckDB sources and the rules that turn them into catalogue forms."""

from __future__ import annotations

import re
from pathlib import Path

DUCKDB_VERSION: str = "2.0.0"
DOCS_VERSION: str = "2.0"
WEB_REPOSITORY: str = "https://github.com/duckdb/duckdb-web.git"
WEB_COMMIT: str = "b68c2955a464bac75ffca9e679e28e36956e14f1"
ENGINE_REPOSITORY: str = "https://github.com/duckdb/duckdb.git"
ENGINE_TAG: str = "v2.0.0"
ENGINE_COMMIT: str = "d54cbbe3df17e8e5ce68312f483b4155836112aa"
DOCS_ROOT: str = f"docs/{DOCS_VERSION}"
RAILROAD_ROOT: str = f"js/{DOCS_VERSION}"
GRAMMAR_ROOT: str = "src/parser/peg/grammar"
GRAMMAR_STATEMENTS: str = f"{GRAMMAR_ROOT}/statements"
WEB_PATHS: tuple[str, ...] = (DOCS_ROOT, RAILROAD_ROOT)
ENGINE_PATHS: tuple[str, ...] = (GRAMMAR_ROOT,)
SITE_URL: str = f"https://duckdb.org/docs/{DOCS_VERSION}/"

WEB_DIR: str = "web"
ENGINE_DIR: str = "engine"
RAILROAD_FILE: str = "railroad.json"
ENGINE_EXTRACT: str = "engine.json"
FETCH_RECORD: str = "fetch.json"
UTC_FORMAT: str = "%Y-%m-%dT%H:%M:%SZ"

RAILROAD_EVALUATOR: Path = Path(__file__).parent / "railroad.js"
RAILROAD_SHARED: str = "railroad.js"
RAILROAD_DIRS: tuple[str, ...] = ("statements", "query_syntax", "expressions")
EMPTY_PLACEHOLDER: str = "-"

PAGE_GLOBS: tuple[str, ...] = (
    "sql/**/*.md",
    "data/json/json_functions.md",
    "data/json/creating_json.md",
    "data/json/loading_json.md",
    "data/json/sql_to_and_from_json.md",
    "data/parquet/overview.md",
    "data/parquet/metadata.md",
    "data/csv/overview.md",
    "data/csv/auto_detection.md",
    "data/partitioning/partitioned_writes.md",
    "operations_manual/logging/overview.md",
    "configuration/secrets_manager.md",
    "guides/file_formats/read_duckdb.md",
    "guides/meta/list_tables.md",
    "guides/performance/how_to_tune_workloads.md",
    "extensions/extension_repositories.md",
    "configuration/pragmas.md",
    "configuration/overview.md",
)
MARKDOWN_GLOB: str = "**/*.md"
PAGE_EXCLUSIONS: tuple[tuple[str, str], ...] = (
    (
        "*.md",
        "landing, quick start, troubleshooting and what's-new pages that link to the reference "
        "and restate its examples",
    ),
    (
        "clients/**/*.md",
        "client APIs (C, CLI, Go, Java, Node.js, ODBC, Python, Rust, Wasm and others): "
        "host-language calls and CLI dot commands, not SQL syntax",
    ),
    ("connect/*.md", "connection and concurrency concepts, with no SQL blocks"),
    (
        "core_extensions/**/*.md",
        "one page per extension: syntax an extension adds exists only once it is installed "
        "and loaded, so it is not DuckDB 2.0.0's built-in syntax; the core grammar parses two "
        "spellings these pages document only for the iceberg, delta and unity_catalog catalogs, "
        "CREATE TABLE ... PARTITIONED BY ( ... ) and ALTER TABLE ... SET PARTITIONED BY (reviewed "
        "gaps in audit.toml) and CREATE TABLE ... WITH ( <option> = <value> ) (an [[unseen]] "
        "row in audit.toml); of the extensions loaded by default, icu.md lists features "
        "catalogued from the sql pages, and autocomplete.md's sql_auto_complete() completion "
        "helper is left uncatalogued with this group (check_peg_parser is catalogued from "
        "sql/peg_parser.md)",
    ),
    (
        "data/**/*.md",
        "import and export guides (appender, data sources, faulty CSV files, JSON format "
        "settings, multiple files, Parquet encryption, Hive partitioning) that apply the read "
        "functions, COPY and the named parameters catalogued from the in-scope data and "
        "statement pages",
    ),
    (
        "dev/**/*.md",
        "developer documentation (building DuckDB, sqllogictest, profiling); the profiling "
        "page's CALL enable_profiling(...) and CALL disable_profiling() table-function "
        "spellings are reviewed under [[function]] in audit.toml, and their PRAGMA spellings "
        "are documented on configuration/pragmas.md in the page set",
    ),
    (
        "extensions/*.md",
        "the extension mechanism (installing, distribution, versioning, community extensions, "
        "troubleshooting) whose INSTALL, LOAD and UPDATE EXTENSIONS spellings are catalogued: "
        "INSTALL by name with FORCE, AND LOAD, FROM and VERSION, and LOAD by name or '<path>', "
        "from statements/load_and_install.md, and INSTALL '<url>' from the in-set "
        "extension_repositories.md; troubleshooting.md's FORCE INSTALL '<path>', which parses, "
        "is documented on no in-set page and is left uncatalogued with this group",
    ),
    (
        "guides/**/*.md",
        "task guides that apply statements and functions catalogued from the reference "
        "(DESCRIBE TABLE '<url>' on guides/meta/describe.md and SUMMARIZE TABLE '<url>' on "
        "guides/meta/summarize.md are DESCRIBE and SUMMARIZE of the TABLE statement on a "
        "replacement scan, both catalogued); the guide pages with syntax of their own are in the "
        "page set",
    ),
    (
        "internals/*.md",
        "internals (storage format, vectors, the PIVOT rewrite, jemalloc); their examples "
        "restate catalogued statements, except that pivot.md shows PIVOT's internal rewrite "
        "into CREATE TEMPORARY TYPE ... AS ENUM ( <query> ) and PIVOT ... ON <column> IN "
        "<enum-type> without parentheses, which parse and run but are written as the engine's "
        "rewrite, not as syntax the reference documents, and are left uncatalogued ([[unseen]] "
        "rows in audit.toml)",
    ),
    (
        "operations_manual/**/*.md",
        "operating DuckDB (limits, Docker, migration, user agents, footprint, installation, "
        "security) with SET, PRAGMA, ATTACH and other catalogued statements; the logging "
        "overview is in the page set",
    ),
    (
        "quack/**/*.md",
        "the Quack client-server protocol of the quack extension, which is not loaded by "
        "default; CONNECT and DISCONNECT are catalogued from statements/connect.md",
    ),
)

FRONT_MATTER: re.Pattern[str] = re.compile(r"\A---\n(.*?)\n---\n", re.S)
FENCE: re.Pattern[str] = re.compile(r"^\s*```")
HEADING: re.Pattern[str] = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
SIGNATURE_HEADING: re.Pattern[str] = re.compile(r"^`([^`]+)`$")
DIAGRAM_DIV: re.Pattern[str] = re.compile(r'<div id="(rrdiagram\d*)">')
TABLE_ROW: re.Pattern[str] = re.compile(r"^\s*\|")

FORM_PREFIX: str = "dd"
FAMILIES: tuple[str, ...] = (
    "query",
    "dml",
    "ddl",
    "transaction_session",
    "security_admin",
    "utility",
    "functions",
    "expressions",
)
QUERY: str = "query"
DML: str = "dml"
DDL: str = "ddl"
SESSION: str = "transaction_session"
SECURITY: str = "security_admin"
UTILITY: str = "utility"
FUNCTIONS: str = "functions"
EXPRESSIONS: str = "expressions"
PAGE_FAMILIES: dict[str, str] = {
    "configuration/overview.md": SESSION,
    "configuration/pragmas.md": UTILITY,
    "sql/constraints.md": DDL,
    "sql/indexes.md": DDL,
    "sql/introduction.md": QUERY,
    "sql/peg-parser.md": UTILITY,
    "sql/peg_parser.md": UTILITY,
    "sql/data_types": EXPRESSIONS,
    "sql/dialect": EXPRESSIONS,
    "sql/expressions": EXPRESSIONS,
    "sql/functions": FUNCTIONS,
    "sql/meta": FUNCTIONS,
    "data/json": FUNCTIONS,
    "data/parquet": FUNCTIONS,
    "data/csv": FUNCTIONS,
    "data/partitioning": FUNCTIONS,
    "operations_manual/logging/overview.md": FUNCTIONS,
    "configuration/secrets_manager.md": SECURITY,
    "guides/file_formats/read_duckdb.md": FUNCTIONS,
    "guides/meta/list_tables.md": UTILITY,
    "extensions/extension_repositories.md": UTILITY,
    "sql/query_syntax": QUERY,
    "sql/statements/alter_database.md": DDL,
    "sql/statements/alter_table.md": DDL,
    "sql/statements/alter_view.md": DDL,
    "sql/statements/comment_on.md": DDL,
    "sql/statements/create_index.md": DDL,
    "sql/statements/create_macro.md": DDL,
    "sql/statements/create_schema.md": DDL,
    "sql/statements/create_secret.md": SECURITY,
    "sql/statements/create_sequence.md": DDL,
    "sql/statements/create_table.md": DDL,
    "sql/statements/create_trigger.md": DDL,
    "sql/statements/create_type.md": DDL,
    "sql/statements/create_view.md": DDL,
    "sql/statements/drop.md": DDL,
    "sql/statements/copy.md": DML,
    "sql/statements/delete.md": DML,
    "sql/statements/insert.md": DML,
    "sql/statements/merge_into.md": DML,
    "sql/statements/update.md": DML,
    "sql/statements/pivot.md": QUERY,
    "sql/statements/select.md": QUERY,
    "sql/statements/unpivot.md": QUERY,
    "sql/statements/set.md": SESSION,
    "sql/statements/set_variable.md": SESSION,
    "sql/statements/transactions.md": SESSION,
    "sql/statements/use.md": SESSION,
    "sql/statements": UTILITY,
}
PAGE_BASES: dict[str, str] = {"sql/query_syntax/select.md": "select-clause"}
DIAGRAM_FAMILY_OVERRIDES: dict[str, str] = {FUNCTIONS: EXPRESSIONS}
SIGNATURE_DIRS: tuple[str, ...] = (
    "sql/functions",
    "sql/meta",
    "data/json",
    "data/parquet",
    "data/csv",
)
TABLE_SIGNATURE: re.Pattern[str] = re.compile(r"^\|\s*\[?`([^`]+)`")
ALIAS_FOR: re.Pattern[str] = re.compile(r"Alias for `([A-Za-z_][A-Za-z0-9_]*)`")
TYPE_TABLE_PAGE: str = "sql/data_types/overview.md"
TYPE_TABLE_HEADINGS: tuple[str, ...] = ("General-Purpose Data Types", "Nested / Composite Types")
TYPE_BASE: str = "data-types"
CALL_SIGNATURE: re.Pattern[str] = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)$", re.S)
CALL_KEYWORDS: re.Pattern[str] = re.compile(
    r"\b(?:ORDER BY|WITHIN GROUP|OVER|FILTER|AS|FROM|FOR|IN)\b"
)
CODE_SPAN: re.Pattern[str] = re.compile(r"`([^`]+)`")
BARE_NAME: re.Pattern[str] = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
WORD_KEYWORD: re.Pattern[str] = re.compile(r"^[A-Z][A-Z_]*(?: [A-Z][A-Z_]*)*$")
LABEL_WORDS: int = 3
MIN_ACTION_ITEMS: int = 2
ID_SEGMENT_DIRS: frozenset[str] = frozenset({"statements", "query_syntax", "expressions"})

DECISION_DIAGRAMS: str = "each railroad diagram on the page read as one form"
DECISION_SPLIT: str = (
    "a diagram whose alternatives are whole statements or actions split per alternative"
)
DECISION_SIGNATURES: str = "signature headings of one function name form one call form"
DECISION_OPERATORS: str = "operator and keyword signatures form one special form per operator"
DECISION_TYPES: str = "each data type table row read as one form, its aliases as alternatives"
DECISION_HAND: str = "catalogued by hand from {basis}"
DECISION_RESTATED: str = "repeats a diagram another page owns; listed in that form's also_sources"
STATUS_FORMS: str = "forms"
STATUS_RESTATED: str = "restated"
STATUS_NO_SYNTAX: str = "no_syntax_block"
STATUS_OUT_OF_SCOPE: str = "out_of_scope"
HAND_FILE: str = "hand.toml"
NAME_KEYWORD: re.Pattern[str] = re.compile(r"^[a-z][a-z0-9_]*(?:-[a-z0-9_]+)+$")
PUNCTUATION: re.Pattern[str] = re.compile(r"^[^\w\s]+$")
SIGNATURE_DEPTH: int = 4
ALIAS_ROW: re.Pattern[str] = re.compile(r"^\|\s*\*\*Alias(?:es|\(es\))?\*\*\s*\|(.*)\|\s*$")
TABLE_FUNCTION_PAGE: str = "sql/meta/duckdb_table_functions.md"
TABLE_FUNCTION_DEPTH: int = 2

PEG_RULE_HEAD: re.Pattern[str] = re.compile(r"^(%?[A-Za-z_][A-Za-z0-9_]*)(?:\(([^)]*)\))?\s*<-")
PEG_TOKEN: re.Pattern[str] = re.compile(
    r"'(?:\\.|[^'\\])*'|\"(?:\\.|[^\"\\])*\"|\[(?:\\.|[^\]\\])*\]i?"
    r"|[A-Za-z_][A-Za-z0-9_]*\(?|[/?*+()!&<>.,]"
)

STATEMENT_RULE: str = "Statement"
KEYWORD_WORD: re.Pattern[str] = re.compile(r"^[A-Z][A-Z0-9_]*$")
WORD: re.Pattern[str] = re.compile(r"[A-Za-z0-9_]+")
PLACEHOLDER: re.Pattern[str] = re.compile(r"<([^<>]+)>")
DEFINITION_HEAD: re.Pattern[str] = re.compile(r"^<[^>]+>\s*::=\s*")
MAX_SET: int = 60
MAX_EXPANSION: int = 4096
MIN_CLAUSE_KEYWORDS: int = 2
AUDIT_FILE: str = "audit.toml"

OPERATOR_HEADER: re.Pattern[str] = re.compile(r"^\|\s*Operator\s*\|")
OPERATOR_PAGES: tuple[str, ...] = ("sql/expressions/comparison_operators.md",)
DECISION_OPERATOR_TABLES: str = "each operator table row read as a special form per operator"
OPERATOR_NAMES: dict[str, str] = {
    "<=>": "spaceship",
    "<->": "arrow-distance",
    "->>": "arrow-text",
    "**": "star-star",
    "//": "slash-slash",
    "<<": "shift-left",
    ">>": "shift-right",
    "<=": "lt-eq",
    ">=": "gt-eq",
    "==": "eq-eq",
    "<>": "lt-gt",
    "!=": "bang-eq",
    "<@": "contained-by",
    "@>": "contains",
    "&&": "and-and",
    "||": "concat",
    "^@": "starts-with",
    "->": "arrow",
    "+": "plus",
    "-": "minus",
    "*": "star",
    "/": "slash",
    "%": "percent",
    "^": "caret",
    "&": "amp",
    "|": "pipe",
    "~": "tilde",
    "!": "bang",
    "<": "lt",
    ">": "gt",
    "=": "eq",
    "@": "at",
}
CELL_SPLIT: re.Pattern[str] = re.compile(r"\|(?=(?:[^`]*`[^`]*`)*[^`]*$)")
ENGINE_QUERIES: dict[str, str] = {
    "functions": (
        "SELECT function_name, function_type, parameters::VARCHAR[] AS parameters, "
        "return_type, internal, alias_of FROM duckdb_functions() "
        "WHERE schema_name IN ('main', 'pg_catalog') ORDER BY ALL"
    ),
    "keywords": "SELECT keyword_name, keyword_category FROM duckdb_keywords() ORDER BY ALL",
    "types": "SELECT DISTINCT type_name, logical_type FROM duckdb_types() ORDER BY ALL",
}

NODE_SEQ: str = "seq"
NODE_CHOICE: str = "choice"
NODE_OPT: str = "opt"
NODE_STAR: str = "star"
NODE_PLUS: str = "plus"
NODE_NOT: str = "not"
NODE_AND: str = "and"
NODE_CAPTURE: str = "capture"
NODE_LIT: str = "lit"
NODE_REF: str = "ref"
NODE_CLASS: str = "class"
NODE_CALL: str = "call"
NODE_ANY: str = "any"
NODE_LISTS: tuple[str, ...] = (NODE_SEQ, NODE_CHOICE)
NODE_WRAPPERS: tuple[str, ...] = (
    NODE_OPT,
    NODE_STAR,
    NODE_PLUS,
    NODE_NOT,
    NODE_AND,
    NODE_CAPTURE,
)
NODE_OPTIONAL_WRAPPERS: tuple[str, ...] = (NODE_OPT, NODE_STAR)
NODE_REQUIRED_WRAPPERS: tuple[str, ...] = (NODE_PLUS, NODE_CAPTURE)

PEG_ESCAPE: str = "\\"
PEG_QUOTE_OPENERS: str = "'\"["
PEG_CLASS_OPEN: str = "["
PEG_CLASS_CLOSE: str = "]"
PEG_COMMENT: str = "#"
PEG_ALTERNATIVE: str = "/"
PEG_SEQUENCE_ENDS: tuple[str, ...] = ("", PEG_ALTERNATIVE, ")", ",", ">")
PEG_NOT: str = "!"
PEG_PREFIXES: tuple[str, ...] = (PEG_NOT, "&")
PEG_SUFFIXES: dict[str, str] = {"?": NODE_OPT, "*": NODE_STAR, "+": NODE_PLUS}
PEG_GROUP_OPEN: str = "("
PEG_GROUP_CLOSE: str = ")"
PEG_CAPTURE_OPEN: str = "<"
PEG_CAPTURE_CLOSE: str = ">"
PEG_ARGUMENT_SEPARATOR: str = ","
PEG_ANY: str = "."
PEG_NAME_START: str = "_"

OPTIONAL_OPEN: str = "["
OPTIONAL_CLOSE: str = "]"
NAME_SEPARATOR: str = " "
SINGLE_QUOTE: str = "'"
OPERANDS: tuple[str, ...] = ("<x>", "<y>")
ANCHOR_KEY: str = "anchor"
STATEMENTS_FOLDER: str = "statements"
EXAMPLE_COLUMN: int = 2
REVIEW_PREFIX: str = "prefix"
REVIEW_NAME: str = "name"
DEFINITION_RENAMES: dict[tuple[str, str], tuple[str, str]] = {
    ("statements/insert.js", "on-confict-clause"): (
        "on-conflict-clause",
        "insert.js misspells the internal name of the ON CONFLICT clause's definition as "
        "on-confict-clause, while its diagram draws <on-conflict-clause>",
    ),
}
DEFINITION_ADDITIONS: dict[tuple[str, str], tuple[str, str, str]] = {
    ("statements/copy.js", "copy-to-options"): (
        "RETURN_FILES { true | false }",
        "PARTITION_PATH <expr> | ORDER_BY ( <expr> [ ASC | DESC ] [ NULLS { FIRST | LAST } ] "
        "[ , <expr> [ ASC | DESC ] [ NULLS { FIRST | LAST } ] ... ] ) | "
        "PRESERVE_ORDER { true | false } | RETURN_FILES { true | false } | "
        "RETURN_STATS { true | false }",
        "copy.md's COPY ... TO options table lists PARTITION_PATH, ORDER_BY, PRESERVE_ORDER and "
        "RETURN_STATS, which the diagram's options choice omits; added in the table's order",
    ),
}
UNSEEN_RULES: tuple[str, ...] = (
    "CreateColumnList",
    "CreateStatement",
    "CreateViewStmt",
    "PivotEnumTarget",
)

TREE_SEQUENCE: str = "sequence"
TREE_OPTIONAL: str = "optional"
TREE_OPTIONAL_SEQUENCE: str = "optional_sequence"
TREE_CHOICE: str = "choice"
TREE_ONE_OR_MORE: str = "one_or_more"
TREE_ZERO_OR_MORE: str = "zero_or_more"
TREE_KEYWORD: str = "keyword"
TREE_PLACEHOLDER: str = "placeholder"
TREE_DEFINITION: str = "definition"
TREE_LEAVES: tuple[str, ...] = (TREE_KEYWORD, TREE_PLACEHOLDER, TREE_DEFINITION)
