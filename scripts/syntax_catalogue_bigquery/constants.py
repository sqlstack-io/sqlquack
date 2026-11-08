"""Pinned BigQuery sources and the rules that turn them into catalogue forms."""

from __future__ import annotations

import re

DOCS_ORIGIN: str = "https://docs.cloud.google.com"
REFERENCE_PREFIX: str = "/bigquery/docs/reference/"
MARKDOWN_SUFFIX: str = ".md.txt"
NAV_PAGE: str = "standard-sql/query-syntax"
NAV_LINK: re.Pattern[str] = re.compile(
    r'href="(/bigquery/docs/reference/(?:standard-sql/[A-Za-z0-9_-]+|system-variables'
    r'|system-procedures))(?:[#?][^"]*)?"'
)
PAGE_LINK: re.Pattern[str] = re.compile(
    r"\]\((?:https://(?:docs\.)?cloud\.google\.com)?(/bigquery/docs/reference/"
    r"(?:standard-sql/[A-Za-z0-9_-]+|system-variables|system-procedures))(?:[#?][^)]*)?\)"
)
USER_AGENT: str = "sqlquack-syntax-catalogue/1"
FETCH_WORKERS: int = 6
FETCH_ATTEMPTS: int = 4
FETCH_TIMEOUT_SECONDS: int = 40
HTTP_OK: int = 200
HTTP_NOT_FOUND: int = 404
UTC_FORMAT: str = "%Y-%m-%dT%H:%M:%SZ"

GOOGLESQL_REPOSITORY: str = "https://github.com/google/googlesql.git"
GOOGLESQL_TAG: str = "2026.11.1"
GOOGLESQL_COMMIT: str = "d48c569aebd63870ff6acecf08b6611015ef1d9d"
GRAMMAR_FILE: str = "googlesql/parser/googlesql.tm"
KEYWORDS_FILE: str = "googlesql/parser/keywords.cc"
LOOKAHEAD_FILE: str = "googlesql/parser/lookahead_transformer.cc"
OPTIONS_FILE: str = "googlesql/public/options.proto"
LICENSE_FILE: str = "LICENSE"
GRAMMAR_FILES: tuple[str, ...] = (
    GRAMMAR_FILE,
    KEYWORDS_FILE,
    LOOKAHEAD_FILE,
    OPTIONS_FILE,
    LICENSE_FILE,
)

STANDARD_SQL: str = "standard-sql/"
DOCS_DIR: str = "docs"
PAGES_DIR: str = "pages"
GRAMMAR_DIR: str = "googlesql"
FETCH_LOG: str = "fetch-log.jsonl"
INDEX_FILE: str = "index.json"
FETCH_RECORD: str = "fetch.json"
SOURCES_FILE: str = "sources.toml"
INDEXES_FILE: str = "indexes.toml"
INDEXES_HEADER: str = (
    "# The pinned GoogleSQL grammar files the omission audit reads: one [[index]] per file,\n"
    "# named by repository, tag, commit and path; written and verified by\n"
    "# scripts/fetch_syntax_docs_bigquery.py and rewritten by\n"
    "# scripts/build_syntax_catalogue_bigquery.py."
)
DEFAULT_ROOT: str = "coverage/syntax-bigquery"
SOURCES_HEADER: str = (
    "# Pinned GoogleSQL for BigQuery reference pages (the grammar pins are in indexes.toml);\n"
    "# written by\n"
    "# scripts/fetch_syntax_docs_bigquery.py (pins) and\n"
    "# scripts/build_syntax_catalogue_bigquery.py (each page's title, family, status and\n"
    "# decisions). The documentation is not versioned, so\n"
    "# it is pinned by fetch date: each page's Markdown export (<url>.md.txt) with its retrieval\n"
    "# time and sha256. The pages and the grammar checkout stay in the private cache."
)

FENCE: re.Pattern[str] = re.compile(r"^\s*```")
HEADING: re.Pattern[str] = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
BOLD_LABEL: re.Pattern[str] = re.compile(r"^\*\*([^*]+)\*\*\s*$")
INDENT: re.Pattern[str] = re.compile(r"^    ")
TABLE_ROW: re.Pattern[str] = re.compile(r"^\s*\|")
EXAMPLE_LABEL: re.Pattern[str] = re.compile(r"^(examples?|output|results?|example \d+)\b", re.I)
EXAMPLE_HEADING: re.Pattern[str] = re.compile(r"\bexamples?\b|^output$", re.I)
SYNTAX_LABEL: re.Pattern[str] = re.compile(r"syntax|signature", re.I)
SYNTAX_LEAD: re.Pattern[str] = re.compile(
    r"^\s*Syntax(?: options)?:\s*$|\bthe following syntax(?:es)?:\s*$"
    r"|\bThe syntax looks like this:\s*$|\bfollowing syntax to build [^:]*:\s*$"
)
SYNTAX_LEAD_LABEL: str = "Syntax:"
SYNTAX_HEADING: re.Pattern[str] = re.compile(r"\bsyntax\b", re.I)
POSITION_EXAMPLE: str = "example"
POSITION_SYNTAX: str = "syntax"
POSITION_SYNOPSIS: str = "synopsis"
POSITION_OTHER: str = "prose"

DOC_TOKEN: re.Pattern[str] = re.compile(
    r"(?P<backquoted>`[^`\n]*`)"
    r"|(?P<literal>[rRbB]?\"[^\"\n]*\"|[rRbB]?'[^'\n]*')"
    r"|(?P<elision>\.\.\.)"
    r"|(?P<dotted>(?:[A-Z][A-Z0-9_]*\.)+[A-Z][A-Z0-9_]*(?=\s*\())"
    r"|(?P<word>[A-Za-z_][A-Za-z0-9_]*)"
    r"|(?P<number>\d+(?:\.\d+)?)"
    r"|(?P<punct>::=|:=|=>|->|<=|>=|<>|!=|\|\||\|>|\S)"
)
ANGLE_LITERALS: frozenset[str] = frozenset({"<", ">"})
NAMED_ARROW: str = "=>"
DEFINITION_HEAD: re.Pattern[str] = re.compile(r"^\s*([a-z_][a-z0-9_]*)\s*(?:::=|:=|:)\s*()$")
DEFINITION_INLINE: re.Pattern[str] = re.compile(r"^\s*([a-z_][a-z0-9_]*)\s*(?:::=|:=|:)\s*(\S.*)$")
NOT_DEFINITION_HEADS: frozenset[str] = frozenset({"label_name"})
STATEMENT_END: re.Pattern[str] = re.compile(r"\s*;\s*$")
SQL_COMMENT: re.Pattern[str] = re.compile(r"--[^\n]*|/\*.*?\*/", re.S)
GRAMMAR_LIKE: re.Pattern[str] = re.compile(
    r"\[\s*\[|\[\s*,|\[\s*[A-Z][A-Z_]*(?:\s+[A-Z][A-Z_]*)*\s*\]|\{[^{}\n]*\|[^{}\n]*\}"
    r"|\.\.\.|^\s*[a-z_][a-z0-9_]*\s*(?:::=|:=|:)\s*$|\[\s*[a-z]+_[a-z0-9_]+[^\]\n]*\]",
    re.M,
)

FORM_PREFIX: str = "bq"
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
TRANSACTION_SESSION: str = "transaction_session"
SECURITY_ADMIN: str = "security_admin"
UTILITY: str = "utility"
FUNCTIONS: str = "functions"
EXPRESSIONS: str = "expressions"
STATEMENT_FAMILIES: frozenset[str] = frozenset(FAMILIES[:6])
HAND_FILE: str = "hand.toml"
AUDIT_FILE: str = "audit.toml"
CATALOGUE_DIR: str = "catalogue"

ML_PREFIX: str = "bigqueryml-syntax-"
ML_STATEMENT_PAGES: re.Pattern[str] = re.compile(
    r"^bigqueryml-syntax-(?:create(?:-.*)?|alter-model|drop-model|export-model)$"
)
FUNCTION_PAGE: re.Pattern[str] = re.compile(
    r"^(?:standard-sql/(?:[a-z0-9]+[_-])*functions|standard-sql/conditional_expressions"
    r"|standard-sql/table-functions-built-in|standard-sql/bigqueryml-syntax-.*)$"
)
PAGE_FAMILIES: dict[str, str] = {
    "standard-sql/query-syntax": QUERY,
    "standard-sql/pipe-syntax": QUERY,
    "standard-sql/graph-query-statements": QUERY,
    "standard-sql/graph-patterns": QUERY,
    "standard-sql/graph-sql-queries": QUERY,
    "standard-sql/dml-syntax": DML,
    "standard-sql/export-statements": DML,
    "standard-sql/load-statements": DML,
    "standard-sql/data-definition-language": DDL,
    "standard-sql/graph-schema-statements": DDL,
    "standard-sql/data-control-language": SECURITY_ADMIN,
    "standard-sql/procedural-language": UTILITY,
    "standard-sql/debugging-statements": UTILITY,
    "system-procedures": UTILITY,
    "standard-sql/bigqueryml-syntax-export-model": UTILITY,
}
ML_STATEMENT_FAMILY: str = DDL
SECTION_FAMILIES: dict[tuple[str, str], str] = {
    ("standard-sql/procedural-language", "Transactions"): TRANSACTION_SESSION,
    ("standard-sql/procedural-language", "`SET`"): TRANSACTION_SESSION,
}
LEAD_FAMILIES: tuple[tuple[tuple[str, ...], str], ...] = (
    (("GRANT",), SECURITY_ADMIN),
    (("REVOKE",), SECURITY_ADMIN),
    (("CREATE", "ROW", "ACCESS", "POLICY"), SECURITY_ADMIN),
    (("DROP", "ROW", "ACCESS", "POLICY"), SECURITY_ADMIN),
    (("DROP", "ALL", "ROW", "ACCESS", "POLICIES"), SECURITY_ADMIN),
    (("CREATE", "DATA_POLICY"), SECURITY_ADMIN),
    (("ALTER", "DATA_POLICY"), SECURITY_ADMIN),
    (("DROP", "DATA_POLICY"), SECURITY_ADMIN),
    (("CREATE", "CAPACITY"), SECURITY_ADMIN),
    (("CREATE", "RESERVATION"), SECURITY_ADMIN),
    (("CREATE", "ASSIGNMENT"), SECURITY_ADMIN),
    (("ALTER", "CAPACITY"), SECURITY_ADMIN),
    (("ALTER", "RESERVATION"), SECURITY_ADMIN),
    (("ALTER", "BI_CAPACITY"), SECURITY_ADMIN),
    (("ALTER", "ORGANIZATION"), SECURITY_ADMIN),
    (("ALTER", "PROJECT"), SECURITY_ADMIN),
    (("DROP", "CAPACITY"), SECURITY_ADMIN),
    (("DROP", "RESERVATION"), SECURITY_ADMIN),
    (("DROP", "ASSIGNMENT"), SECURITY_ADMIN),
)
SECTION_SUFFIXES: re.Pattern[str] = re.compile(r"\s+(?:statements?|clause|syntax|operator)$", re.I)
PAGE_BASES: dict[str, str] = {
    "standard-sql/data-types": "data-types",
    "standard-sql/lexical": "lexical",
    "standard-sql/operators": "operators",
    "standard-sql/bigqueryml-syntax-create": "create-model",
}
CALL_PRODUCTION: re.Pattern[str] = re.compile(
    r"^((?:[A-Z][A-Z0-9_]*\.)*[A-Z][A-Z0-9_]*)\s*\(", re.ASCII
)

STATUS_FORMS: str = "forms"
STATUS_NO_SYNTAX: str = "no_syntax_block"
STATUS_OUT_OF_SCOPE: str = "out_of_scope"
PAGE_STATUSES: frozenset[str] = frozenset({STATUS_NO_SYNTAX, STATUS_OUT_OF_SCOPE})
BLOCK_EXAMPLE: str = "example"
BLOCK_NOT_SYNTAX: str = "not_syntax"
BLOCK_RESTATED: str = "restated"
BLOCK_OUT_OF_SCOPE: str = "out_of_scope"
BLOCK_CORRECTED: str = "corrected"
BLOCK_NOTATION: str = "notation"
BLOCK_STATUSES: frozenset[str] = frozenset(
    {
        BLOCK_EXAMPLE,
        BLOCK_NOT_SYNTAX,
        BLOCK_RESTATED,
        BLOCK_OUT_OF_SCOPE,
        BLOCK_CORRECTED,
        BLOCK_NOTATION,
    }
)
DECISION_REVIEWED_NOTATION: str = (
    "a block outside a syntax section reviewed as notation and read as one (hand.toml)"
)
DECISION_BLOCKS: str = (
    "syntax and synopsis blocks read as notation: lower-case words and one-letter operands are"
    " placeholders, and a comma before an optional part is read inside it"
)
DECISION_CALLS: str = "each function's signatures form one call form"
DECISION_CORRECTED: str = "a reviewed block replaced by its corrected grammar (hand.toml)"
DECISION_DROPPED: str = "a reviewed block in a syntax position yields no form (hand.toml)"
DECISION_PROSE_EXAMPLES: str = (
    "code blocks after prose that use no notation are examples, as are blocks under examples"
)
DECISION_HAND: str = "catalogued by hand from {basis}"
HAND_BASES: frozenset[str] = frozenset({"table", "prose", "heading", "inline_code", "code_block"})

LOWER_KEYWORD_PLACEHOLDERS: frozenset[str] = frozenset(
    {
        "all",
        "and",
        "as",
        "asc",
        "by",
        "case",
        "cast",
        "desc",
        "distinct",
        "else",
        "end",
        "false",
        "from",
        "group",
        "having",
        "in",
        "is",
        "join",
        "like",
        "limit",
        "not",
        "null",
        "on",
        "or",
        "order",
        "over",
        "select",
        "struct",
        "then",
        "true",
        "union",
        "unnest",
        "using",
        "values",
        "when",
        "where",
        "with",
    }
)
SUSPECT_TEXT: re.Pattern[str] = re.compile(r"://|'''|^\s*\+-|^\s*\|.*\|\s*$|^\s*/\s*\*|\$\{", re.M)
CAMEL_PLACEHOLDER: re.Pattern[str] = re.compile(r"<(?=[A-Za-z0-9_]*[a-z])(?=[A-Za-z0-9_]*[A-Z])")
MARKDOWN_LINK: re.Pattern[str] = re.compile(r"\[([^\]\n]*)\]\((?:https?://|/)[^)\s]*\)")
BACKQUOTED_NAME: re.Pattern[str] = re.compile(r"[^A-Za-z0-9_.]+")
DOTTED_CALL: re.Pattern[str] = re.compile(
    r"(?<![\w.])((?:[A-Z][A-Z0-9_]*\.)+[A-Z][A-Z0-9_]*)(?=\s*\()", re.ASCII
)
BRACKET_OPENERS: str = "([{"
BRACKET_CLOSERS: str = ")]}"
CALL_LINE: re.Pattern[str] = re.compile(r"\s*(?:[A-Z][A-Z0-9_]*\.)*[A-Z][A-Z0-9_]*\(")
LINKED_PLACEHOLDER: re.Pattern[str] = re.compile(
    r"https://(?:docs\.)?cloud\.google\.com/[^\s#)\]]*?(?:#([A-Za-z_][A-Za-z0-9_]*)"
    r"|/([A-Za-z_][A-Za-z0-9_-]*))(?=[\s,)\]]|$)"
)
KEYWORDS_TABLE_START: str = "constexpr KeywordInfoPOD kAllKeywords[] = {"
KEYWORDS_TABLE_END: str = "\n};"
SPANNER_KEYWORDS_END: str = "// End of Spanner-specific keywords"
KEYWORD_ENTRY: re.Pattern[str] = re.compile(
    r'\{"([a-z_]+)",\s*(?:Token::\w+|ConditionallyReservedToken\{[^}]*\})\s*(?:,\s*(k\w+))?\s*\}'
)
KEYWORD_CLASSES: dict[str, str] = {
    "kReserved": "reserved",
    "kNotReserved": "nonreserved",
    "kConditionallyReserved": "conditional",
}
ARGUMENT_BULLET: re.Pattern[str] = re.compile(r"^\s*[-*]\s+`([A-Z][A-Z0-9_]*)`\s*:", re.M)
CODE_SPAN: re.Pattern[str] = re.compile(r"`([^`]+)`")
FENCED_BLOCK: re.Pattern[str] = re.compile(r"^ *(```|~~~)[^\n]*\n.*?^ *\1", re.M | re.S)
ESCAPED_TICK: str = "\\`"
PAGE_CODE_SPAN: re.Pattern[str] = re.compile(r"(?<!`)(`+)(?!`)(.+?)(?<!`)\1(?!`)")
OPERATOR_TABLE_HEADER: re.Pattern[str] = re.compile(r"^\|\s*Order of Precedence\s*\|")
TYPE_NAME: re.Pattern[str] = re.compile(
    r"SQL type name:\s*`([A-Z0-9_]+)`(?:\s*SQL aliases:\s*((?:`[A-Z0-9_]+`(?:,\s*)?)+))?"
)
TYPE_TEMPLATES: dict[str, str] = {
    "ARRAY": 'ARRAY "<" <element_type> ">"',
    "STRUCT": 'STRUCT "<" [ <field_name> ] <field_type> [ , ... ] ">"',
    "RANGE": 'RANGE "<" <range_element_type> ">"',
}
ANCHOR_KEY: str = "anchor"
DATA_TYPES_PAGE: str = "standard-sql/data-types"
OPERATORS_PAGE: str = "standard-sql/operators"
DEFINITION_PREFIX: str = "<"
DEFINITION_CLOSE: str = ">"
ML_FUNCTION_BASE: str = "bqml"
SECTION_DEPTH_TWO: frozenset[str] = frozenset({"standard-sql/pipe-syntax"})
STATUS_RESTATED: str = "restated"
DECISION_OPERATOR_TABLE: str = "operator precedence table rows read as operator forms"
DECISION_TYPE_TABLE: str = "data type summary table read as one form per type with its aliases"
DECISION_RESTATED: str = (
    "a grammar repeated byte for byte from an earlier page is listed in its also_sources"
)
NUMBERED_LINE: re.Pattern[str] = re.compile(r"^\s*\d+\.\s+(\S.*)$")
GQL_PAGES: frozenset[str] = frozenset(
    {"standard-sql/graph-query-statements", "standard-sql/graph-patterns"}
)
GQL_BASE_PREFIX: str = "gql-"
LETTER_OPERAND: re.Pattern[str] = re.compile(r"^(?:[A-Z][0-9]?|XN)$")
INTRODUCERS: frozenset[str] = frozenset({"AS", "=", "=>", "("})
DOUBLED_COMMA: re.Pattern[str] = re.compile(r", \[ , ")
COMMA_BEFORE_OPTIONAL: re.Pattern[str] = re.compile(r", \[ (?!,|\.\.\. \])")
STRUCT_OPEN: str = "STRUCT ( "
PAREN_DEPTH: dict[str, int] = {"(": 1, ")": -1}
STRUCT_FIELDS: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"\[ <([A-Z][A-Z0-9_]*)> \] AS <([a-z][a-z0-9_]*)>"), '"[" <{0}> "]" AS {1}'),
    (re.compile(r"('[A-Z][A-Z0-9_]*') AS <([a-z][a-z0-9_]*)>"), "{0} AS {1}"),
    (re.compile(r"(?<![<\w])([A-Z][A-Z0-9_]*) AS <([a-z][a-z0-9_]*)>"), "<{0}> AS {1}"),
)
STRUCT_FIELD: re.Pattern[str] = re.compile(
    r"(?:<[A-Z][A-Z0-9_]*>|'[A-Z][A-Z0-9_]*'|\"\[\" <[A-Z][A-Z0-9_]*> \"\]\") AS [A-Z][A-Z0-9_]*"
)
STRUCT_FIELD_FRAME: re.Pattern[str] = re.compile(r"[\s\[\]{}|,]*")
ARRAY_ARGUMENT: re.Pattern[str] = re.compile(r"=> \[ (<[^<>]+>) \]")
CALL_DEFAULT: re.Pattern[str] = re.compile(r"\bDEFAULT NULL\b")
QUOTED: re.Pattern[str] = re.compile(r"'[^'\n]*'")
MISPAIRED_QUOTE: re.Pattern[str] = re.compile(r"\s[|{}\[\]]\s|`")
SELECTED_CALL: re.Pattern[str] = re.compile(
    r"^SELECT (?:\* FROM )?((?:[A-Z][A-Z0-9_]*\.)*[A-Z][A-Z0-9_]* \(.*\))$"
)
DECISION_SELECTED_CALL: str = (
    "a function synopsis written inside SELECT or SELECT * FROM is read as the call itself"
)
DEFINITION_FORM_PAGES: frozenset[str] = frozenset(
    {"standard-sql/graph-patterns", "standard-sql/lexical"}
)
DECISION_DEFINITION_FORM: str = (
    "a block of definitions only documents the construct its first definition names:"
    " that definition is also a form"
)

TM_PARSER: str = ":: parser"
TM_END: str = "\n%%"
OPT_SUFFIX: str = "-opt"
SEPARATOR: str = "separator"
NODE_LIT: str = "lit"
NODE_TOKEN: str = "token"
NODE_REF: str = "ref"
NODE_SEQ: str = "seq"
NODE_CHOICE: str = "choice"
NODE_OPT: str = "opt"
NODE_STAR: str = "star"
NODE_PLUS: str = "plus"
NODE_LIST: str = "list"
NODE_SET: str = "set"
FEATURE_TEST: re.Pattern[str] = re.compile(r"\bFEATURE_[A-Z0-9_]+")
DISPATCH_CALL: re.Pattern[str] = re.compile(
    r'CaseEqual\(\s*(?:[^"()]|\([^()]*\))*?,\s*"([A-Za-z_]+)"\s*\)', re.S
)
DISPATCH_PREFIX: str = "_"
WILDCARD: str = "…"
KEYWORD_WORD: re.Pattern[str] = re.compile(r"^[A-Z][A-Z0-9_]*$")
FORM_TOKEN: re.Pattern[str] = re.compile(
    r"<[^<>]*>|'[^']*'|\"[^\"]*\"|\.\.\.|[A-Za-z_][A-Za-z0-9_]*|[\[\]{}|]|\S"
)
MAX_EXPANSION: int = 65536
MAX_SET: int = 60
MIN_CLAUSE_KEYWORDS: int = 2
INLINE_DEPTH: int = 4
STATEMENT_DISPATCH: tuple[str, ...] = (
    "unterminated_sql_statement",
    "sql_statement_body",
    "sql_statement_body_no_pipe_suffix",
    "sql_statement_body_maybe_pipe_suffix",
    "unterminated_script_statement",
    "unterminated_unlabeled_script_statement",
)
FUNCTION_INDEX_PAGE: str = "standard-sql/functions-all"
FUNCTION_INDEX_ENTRY: re.Pattern[str] = re.compile(r"^\| \[`([A-Za-z0-9_.]+)`\]", re.M)
REJECTED: str = "rejected"
REJECTING_ACTION: re.Pattern[str] = re.compile(r"\{\s*return\s+MakeSyntaxError\(")
TOKEN_ALIASES: dict[str, str] = {"NOT_SPECIAL": "NOT"}
OBJECT_VERBS: frozenset[str] = frozenset({"CREATE", "ALTER", "DROP", "EXPORT", "REPLACE"})
OPTIONS_OPEN: str = "OPTIONS"
OPERATOR_SHAPES: dict[str, str] = {
    "[NOT] BETWEEN": "<x> [ NOT ] BETWEEN <y> AND <z>",
    "[NOT] IN": "<x> [ NOT ] IN ( <y> [ , ... ] )",
}
SYNTAX_COLUMN: re.Pattern[str] = re.compile(r"(?:Function )?Syntax")
DECISION_SYNTAX_TABLES: str = (
    "operator tables' Syntax columns read as one form per code span, unless the precedence"
    " table already spells it; the precedence table's BETWEEN and IN rows take the operands"
    " their sections give"
)
GRAPH_HEADING: re.Pattern[str] = re.compile(r"\bgraph\b", re.I)
WORD_TOKEN: re.Pattern[str] = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
WALK_ROOTS: tuple[str, ...] = (
    "script",
    "sql_statement",
    "next_script_statement",
    "next_statement",
    "standalone_expression",
    "standalone_type",
)
GOOGLESQL_ONLY: str = "googlesql_only"
UNDOCUMENTED: str = "undocumented"
SPELLED_OTHERWISE: str = "spelled_otherwise"
OUTSIDE_PINNED: str = "outside_pinned"
GAP_KINDS: frozenset[str] = frozenset(
    {GOOGLESQL_ONLY, UNDOCUMENTED, SPELLED_OTHERWISE, OUTSIDE_PINNED}
)
ABSENT_KINDS: frozenset[str] = frozenset({GOOGLESQL_ONLY, UNDOCUMENTED, OUTSIDE_PINNED})
OUTSIDE_PINNED_REASON: str = "BigQuery syntax outside the pinned reference"
MENTIONED_ON: str = "mentioned_on"
GOOGLESQL_ONLY_REASON: str = "GoogleSQL grammar, not a BigQuery feature"
FEATURE_ENABLED: str = "enabled"
FEATURE_NOT_ENABLED: str = "not_enabled"
FEATURE_STATES: frozenset[str] = frozenset({FEATURE_ENABLED, FEATURE_NOT_ENABLED})
UNSEEN_RULES: tuple[str, ...] = (
    "braced_constructor",
    "cast_expression",
    "generalized_extension_path",
    "hint",
    "maybe_dashed_path_expression",
    "on_or_using_clause_lists",
    "options_assignment_operator",
    "slashed_path_expression",
)
HELPER_HEAD: re.Pattern[str] = re.compile(r"^\s+[\w:<>*&]+\s+([A-Z]\w*)\(", re.M)
TM_LINE_COMMENT: str = "//"
TM_HASH_COMMENT: str = "#"
TM_BLOCK_OPEN: str = "/*"
TM_BLOCK_CLOSE: str = "*/"
TM_STRING_QUOTE: str = '"'
TM_QUOTES: str = "\"'"
TM_ESCAPE: str = "\\"
TM_ACTION_OPEN: str = "{"
TM_ACTION_CLOSE: str = "}"
TM_LABEL_OPEN: str = "["
TM_LABEL_CLOSE: str = "]"
TM_DIRECTIVE: re.Pattern[str] = re.compile(r"%[a-z]*")
TM_PREC: str = "%prec"
TM_MARKER: str = "."
TM_NAME: re.Pattern[str] = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:-opt)?")
TM_TOKEN_NAME: re.Pattern[str] = re.compile(r"[A-Z][A-Z0-9_]*")
TM_GROUP_OPEN: str = "("
TM_GROUP_CLOSE: str = ")"
TM_ALTERNATIVE: str = "|"
TM_RULE_END: str = ";"
TM_RULE_COLON: str = ":"
TM_WRAPPERS: dict[str, str] = {"?": NODE_OPT, "*": NODE_STAR, "+": NODE_PLUS}
TOKEN_STRING: str = "string"
TOKEN_ACTION: str = "action"
TOKEN_DIRECTIVE: str = "directive"
TOKEN_NAME: str = "name"
TOKEN_PUNCT: str = "punct"
TOKEN_END: str = "end"
KW_TOKEN_PREFIX: str = "KW_"
KW_TOKEN_JOINER: str = "_"
FORM_ALTERNATIVE: str = "|"
FORM_OPTIONAL_OPEN: str = "["
FORM_OPTIONAL_CLOSE: str = "]"
FORM_CHOICE_OPEN: str = "{"
FORM_CHOICE_CLOSE: str = "}"
FORM_ELISION: str = "..."
FORM_PLACEHOLDER: re.Pattern[str] = re.compile(r"<([^<>]+)>")
BLOCK_PLACEHOLDER: re.Pattern[str] = re.compile(r"<([A-Za-z_][A-Za-z0-9_]*)>")
FORM_CLOSERS: tuple[str, ...] = (FORM_ALTERNATIVE, FORM_OPTIONAL_CLOSE, FORM_CHOICE_CLOSE)
DEFINITION_SEPARATOR: str = "::="
TYPE_ALIAS_SPAN: re.Pattern[str] = re.compile(r"`([A-Z0-9_]+)`")
NODE_WRAPPERS: tuple[str, ...] = (NODE_OPT, NODE_STAR, NODE_PLUS, NODE_LIST)
NODE_REQUIRED_WRAPPERS: tuple[str, ...] = (NODE_PLUS, NODE_LIST)
NODE_LISTS: tuple[str, ...] = (NODE_SEQ, NODE_CHOICE)
BRACKET_PAIRS: dict[str, str] = {")": "(", "]": "[", "}": "{"}
CODE_TICK: str = "`"
TABLE_CELL: str = "|"
UNARY_ARITY: str = "Unary"
IS_PREFIX: str = "IS "
PLACEHOLDER_OPEN: str = "<"
OPERATOR_TABLE_CELLS: int = 5
TYPE_TABLE_CELLS: int = 2
OPERATOR_NAMES: dict[str, str] = {
    "+": "plus",
    "-": "minus",
    "~": "bitwise-not",
    "*": "multiply",
    "/": "divide",
    "||": "concat",
    "<<": "shift-left",
    ">>": "shift-right",
    "&": "bitwise-and",
    "^": "bitwise-xor",
    "|": "bitwise-or",
    "=": "equal",
    "<": "less",
    ">": "greater",
    "<=": "less-equal",
    ">=": "greater-equal",
    "!=": "not-equal",
    "<>": "not-equal-sql",
}
DOC_WORD: str = "word"
DOC_BACKQUOTED: str = "backquoted"
DOC_PUNCT: str = "punct"
DOC_NAMING_KINDS: tuple[str, ...] = (DOC_BACKQUOTED, DOC_WORD)
QUOTE_CHARS: str = "'\"`"
LINE_COMMENT: str = "--"
HASH_COMMENT: str = "#"
PAREN_OPEN: str = "("
PAREN_CLOSE: str = ")"
OPTION_EQUALS: str = "="
