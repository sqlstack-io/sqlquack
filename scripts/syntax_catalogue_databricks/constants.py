"""Pinned Databricks SQL sources and the rules that turn them into catalogue forms."""

from __future__ import annotations

import re

DBSQL_VERSION: str = "2026.44"
ENGINE_LABEL: str = f"Databricks SQL {DBSQL_VERSION} (serverless SQL warehouse, Current channel)"
SPARK_TAG: str = "v4.2.0"
SPARK_COMMIT: str = "32f7299601108917fb01920a54e084595b7b3bf8"
SPARK_RAW: str = f"https://raw.githubusercontent.com/apache/spark/{SPARK_COMMIT}/"
PARSER_DIR: str = "sql/api/src/main/antlr4/org/apache/spark/sql/catalyst/parser/"
PARSER_FILE: str = PARSER_DIR + "SqlBaseParser.g4"
LEXER_FILE: str = PARSER_DIR + "SqlBaseLexer.g4"
HELPER_FILES: tuple[str, ...] = (
    "sql/catalyst/src/main/scala/org/apache/spark/sql/catalyst/parser/AstBuilder.scala",
    "sql/api/src/main/scala/org/apache/spark/sql/catalyst/parser/DataTypeAstBuilder.scala",
    "sql/core/src/main/scala/org/apache/spark/sql/execution/SparkSqlParser.scala",
)
SPARK_FILES: tuple[str, ...] = (PARSER_FILE, LEXER_FILE, *HELPER_FILES)

SITE: str = "https://docs.databricks.com"
SITEMAP_URL: str = f"{SITE}/aws/en/sitemap.xml"
MANUAL_PREFIX: str = f"{SITE}/aws/en/sql/language-manual/"
SITEMAP_LOC: re.Pattern[str] = re.compile(r"<loc>([^<]+)</loc>")
USER_AGENT: str = "sqlquack-syntax-catalogue/1"
FETCH_TIMEOUT_SECONDS: int = 120
FETCH_WORKERS: int = 2
FETCH_ATTEMPTS: int = 4
FETCH_PAUSE_SECONDS: float = 0.2
UTC_FORMAT: str = "%Y-%m-%dT%H:%M:%SZ"
FETCH_RECORD: str = "fetch.json"
PAGES_DIR: str = "pages"
SPARK_DIR: str = "spark"
SITEMAP_FILE: str = "sitemap.xml"
PAGE_SUFFIX: str = ".html"
INDEX_PAGE: str = "index"

FORM_PREFIX: str = "db"
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
)

ARTICLE: re.Pattern[str] = re.compile(r"<article.*?</article>", re.S)
TITLE_CODE: re.Pattern[str] = re.compile(r"<code>(.*?)</code>", re.S)
OPERATOR_TITLE: re.Pattern[str] = re.compile(r"\b(?:operator|predicate)$")
# Lower-case connective words an operator or predicate page writes as part of its syntax.
OPERATOR_WORDS: frozenset[str] = frozenset(
    {"not", "and", "or", "is", "null", "true", "false", "unknown", "from", "distinct", "in"}
)
TITLE: re.Pattern[str] = re.compile(r"<h1[^>]*>(.*?)</h1>", re.S)
HEADING_OR_BLOCK: re.Pattern[str] = re.compile(
    r"<(h[2-4])[^>]*?\bid=\"?([^\" >]*)[^>]*>(.*?)</\1>|<pre[^>]*>(.*?)</pre>", re.S
)
APPLIES_TO: re.Pattern[str] = re.compile(r"<p><strong>Applies to:</strong>(.*?)</p>", re.S)
BREAK: re.Pattern[str] = re.compile(r"<br\s*/?>")
TAG: re.Pattern[str] = re.compile(r"<[^>]+>")
BLOCK_HEADINGS: frozenset[str] = frozenset({"syntax", "literals"})

UNICODE_ELLIPSIS: str = "\u2026"
ELISION: str = "..."
LINE_COMMENT: re.Pattern[str] = re.compile(r"\s--\s.*$|^\s*--.*$")
DEFINITION_HEAD_LINE: re.Pattern[str] = re.compile(r"^\s*([a-z][A-Za-z0-9_]*)\s*:?\s*$")
OPENING: frozenset[str] = frozenset({"[", "{", "("})
CLOSING: frozenset[str] = frozenset({"]", "}", ")"})
DOC_TOKEN: re.Pattern[str] = re.compile(
    r"'[^']*'|\"[^\"]*\"|<[A-Za-z_][A-Za-z0-9_]*>|\.\.\.|/\*\+|\*/|=>|->|::|<=>|>>>|>>|<<|==|"
    r"<=|>=|<>|!=|\|>|\|\||"
    r"[A-Za-z_][A-Za-z0-9_]*|\d+(?:\.\d+)?|\S"
)
REPEAT_COMMA: re.Pattern[str] = re.compile(r"\[\s*,\s*\.\.\.\s*\]")
REPEAT_PLAIN: re.Pattern[str] = re.compile(r"\[\s*\.\.\.\s*\]")
CALL_OPEN: str = "("
NAMED_ARROW: str = "=>"
TYPE_OPENERS: frozenset[str] = frozenset({"ARRAY", "MAP", "STRUCT", "OBJECT"})

QUERY: str = "query"
DML: str = "dml"
DDL: str = "ddl"
SESSION: str = "transaction_session"
SECURITY: str = "security_admin"
UTILITY: str = "utility"
SCRIPTING: str = "scripting"
FUNCTIONS: str = "functions"
EXPRESSIONS: str = "expressions"
SOURCE_PREFIX: str = "doc."
SLUG_PREFIXES: tuple[str, ...] = (
    "sql-ref-syntax-qry-",
    "sql-ref-syntax-ddl-",
    "sql-ref-syntax-dml-",
    "sql-ref-syntax-aux-",
    "sql-ref-syntax-txn-",
    "sql-ref-syntax-",
    "sql-ref-",
    "delta-",
    "security-",
)
FAMILY_BY_PREFIX: dict[str, str] = {
    "control-flow/": SCRIPTING,
    "data-types/": EXPRESSIONS,
    "functions/": FUNCTIONS,
    "information-schema/": UTILITY,
    "parameters/": SESSION,
    "security-": SECURITY,
    "delta-": DDL,
    "sql-ref-syntax-qry-": QUERY,
    "sql-ref-syntax-qry-explain": UTILITY,
    "sql-ref-syntax-dml-": DML,
    "sql-ref-syntax-ddl-": DDL,
    "sql-ref-syntax-ddl-use": SESSION,
    "sql-ref-syntax-ddl-declare-variable": SESSION,
    "sql-ref-syntax-ddl-drop-variable": SESSION,
    "sql-ref-syntax-aux-": UTILITY,
    "sql-ref-syntax-aux-conf-mgmt-": SESSION,
    "sql-ref-syntax-aux-set-": SESSION,
    "sql-ref-syntax-aux-execute-immediate": SCRIPTING,
    "sql-ref-syntax-txn-": SESSION,
    "sql-ref-syntax-comment": EXPRESSIONS,
    "sql-ref-privileges": SECURITY,
    "sql-ref-principal": SECURITY,
    "sql-ref-partition": DDL,
    "sql-ref-scripting": SCRIPTING,
    "sql-ref-syntax-window-functions-frame": EXPRESSIONS,
}
FAMILY_BY_PAGE: dict[str, str] = {
    "delta-delete-from": DML,
    "delta-update": DML,
    "delta-merge-into": DML,
    "delta-copy-into": DML,
    "delta-cache": UTILITY,
    "delta-describe-history": UTILITY,
    "delta-optimize": UTILITY,
    "delta-vacuum": UTILITY,
    "delta-fsck": UTILITY,
    "delta-generate": UTILITY,
    "delta-reorg-table": UTILITY,
    "delta-restore": UTILITY,
    "": UTILITY,
    "how-to-use": UTILITY,
}

HAND_FILE: str = "hand.toml"
HAND_FORM_KEYS: frozenset[str] = frozenset(
    {"page", "anchor", "kind", "label", "grammar", "basis", "article_sha256"}
)
HAND_PAGE_KEYS: frozenset[str] = frozenset({"page", "status", "reason", "article_sha256"})
HAND_READING_KEYS: frozenset[str] = frozenset({"page", "written", "read", "reason"})
# section: a syntax block under a heading the build does not read, found by the notation-block
# check of the omission audit.
HAND_BASES: frozenset[str] = frozenset({"prose", "table", "list", "example", "section"})
HAND_KINDS: frozenset[str] = frozenset({"statement", "call", "special"})
STATUS_FORMS: str = "forms"
STATUS_NO_SYNTAX: str = "no_syntax_block"
STATUS_OUT_OF_SCOPE: str = "out_of_scope"
STATUS_RESTATED: str = "restated"
PAGE_DISPOSITIONS: frozenset[str] = frozenset({STATUS_NO_SYNTAX, STATUS_OUT_OF_SCOPE})
STATEMENT_FAMILIES: frozenset[str] = frozenset(
    {QUERY, DML, DDL, SESSION, SECURITY, UTILITY, SCRIPTING}
)
DECISION_SYNTAX: str = "the page's Syntax blocks transcribed into the catalogue notation"
DECISION_HAND: str = "catalogued by hand from {basis}"
DECISION_APPLIES: str = "applies to: {applies}"
DECISION_RUNTIME_ONLY: str = (
    "documented for Databricks Runtime only; the pinned engine is a Databricks SQL warehouse"
)
DECISION_READING: str = "a syntax line read by hand (hand.toml [[reading]])"
FORMS_HEADER: str = (
    "# Documented Databricks SQL syntax forms, family {family}; generated by "
    "scripts/build_syntax_catalogue_databricks.py."
)
SOURCES_FILE: str = "sources.toml"
ARTICLES_FILE: str = "articles.toml"
INDEXES_FILE: str = "indexes.toml"
CATALOGUE_DIR: str = "catalogue"
APPLIES_SQL: str = "Databricks SQL"
CLAUSE_WORD: str = "clause"
CLAUSE_WORDS: int = 3
KEYWORD_TOKEN: re.Pattern[str] = re.compile(r"[A-Z][A-Z0-9_]*")
STATEMENT_WORD: re.Pattern[str] = re.compile(r"^\s*([A-Z][A-Z_]+)\b")

# The omission audit (scripts/audit_syntax_catalogue_databricks.py).
AUDIT_FILE: str = "audit.toml"
NODE_SEQ: str = "seq"
NODE_CHOICE: str = "choice"
NODE_OPTIONAL: str = "optional"
NODE_STAR: str = "star"
NODE_PLUS: str = "plus"
NODE_TOKEN: str = "token"
NODE_REF: str = "ref"
NODE_EMPTY: str = "empty"
G4_LABEL_ASSIGN: frozenset[str] = frozenset({"=", "+="})
G4_RULE_HEAD: re.Pattern[str] = re.compile(r"^([a-z][A-Za-z0-9_]*)\s*\n?\s*:", re.M)
KEYWORD_LIST: re.Pattern[str] = re.compile(
    r"//--SPARK-KEYWORD-LIST-START(.*?)//--SPARK-KEYWORD-LIST-END", re.S
)
LEXER_TOKEN: re.Pattern[str] = re.compile(r"^([A-Z][A-Z0-9_]*)\s*:(.*?);", re.M | re.S)
LEXER_WORD: re.Pattern[str] = re.compile(r"'([A-Z][A-Z0-9_]*)'")
START_RULES: tuple[str, ...] = (
    "compoundOrSingleStatement",
    "singleExpression",
    "singleTableIdentifier",
    "singleMultipartIdentifier",
    "singleFunctionIdentifier",
    "singleDataType",
    "singleTableSchema",
    "singlePathElementList",
    "singleRoutineParamList",
)
LEADING_RULES: frozenset[str] = frozenset({"statement"})
# Rules that list keywords usable as identifiers: they spell an identifier, as IDENTIFIER does.
IDENTIFIER_RULES: frozenset[str] = frozenset(
    {"nonReserved", "ansiNonReserved", "strictNonReserved"}
)
# Punctuation and operator tokens the audit compares, each with every spelling the lexer gives it.
PUNCTUATION_TOKENS: dict[str, tuple[str, ...]] = {
    "LEFT_PAREN": ("(",),
    "RIGHT_PAREN": (")",),
    "COMMA": (",",),
    "DOT": (".",),
    "EQ": ("=",),
    "NSEQ": ("<=>",),
    "NEQ": ("<>",),
    "NEQJ": ("!=",),
    "LT": ("<",),
    "LTE": ("<=", "!>"),
    "GT": (">",),
    "GTE": (">=", "!<"),
    "SHIFT_LEFT": ("<<",),
    "SHIFT_RIGHT": (">>",),
    "SHIFT_RIGHT_UNSIGNED": (">>>",),
    "PLUS": ("+",),
    "MINUS": ("-",),
    "ASTERISK": ("*",),
    "SLASH": ("/",),
    "PERCENT": ("%",),
    "TILDE": ("~",),
    "AMPERSAND": ("&",),
    "PIPE": ("|",),
    "CONCAT_PIPE": ("||",),
    "OPERATOR_PIPE": ("|>",),
    "HAT": ("^",),
    "DOUBLE_COLON": ("::",),
    "ARROW": ("->",),
    "FAT_ARROW": ("=>",),
    "COLON": (":",),
}
ANY_RUN: str = "\u2026"
MAX_SET: int = 60
MAX_EXPANSION: int = 65536
INLINE_DEPTH: int = 3
KEYWORD_SYMBOL: re.Pattern[str] = re.compile(r"^[A-Z][A-Z0-9_]*$")
DEFINITION_PREFIX: re.Pattern[str] = re.compile(r"^\s*<[^<>]+>\s*::=\s*")
HELPER_WORD_LINE: re.Pattern[str] = re.compile(
    r"case |==|!=|equalsIgnoreCase|\.equals\(|Seq\(|Set\(|contains\("
)
HELPER_WORD_EXCLUDED_LINE: re.Pattern[str] = re.compile(r"messageParameters|Map\(|^\s*(\*|//)")
HELPER_WORD: re.Pattern[str] = re.compile(r"\"([A-Za-z_][A-Za-z0-9_]*)\"")
FUNCTION_INDEX_PAGES: tuple[str, ...] = (
    "sql-ref-functions-builtin-alpha",
    "sql-ref-h3-geospatial-functions-alpha",
    "sql-ref-st-geospatial-functions-alpha",
)
INDEX_LINK: re.Pattern[str] = re.compile(
    r"href=\"?(/aws/en/sql/language-manual/functions/[^\"\s>#?]+)"
)
REVIEW_KEYWORD: str = "keyword"
REVIEW_RULE: str = "rule"
REVIEW_TOKEN: str = "token"
REVIEW_WORD: str = "word"
REVIEW_PAGE: str = "page"
REVIEW_BLOCK: str = "block"
# A code block under a heading the build does not read, other than an Examples heading or an
# example run (a line opening with the > prompt), that writes syntax notation outside its string
# literals: an optional or repeated part, a choice in braces, or an ellipsis. Each needs a
# [[block]] row saying where its syntax is catalogued or why it is no syntax.
EXAMPLE_HEADING: re.Pattern[str] = re.compile(r"(?i)^examples?\b")
EXAMPLE_PROMPT: re.Pattern[str] = re.compile(r"^\s*>", re.MULTILINE)
QUOTED_TEXT: re.Pattern[str] = re.compile(r"'[^'\n]*'|\"[^\"\n]*\"")
SYNTAX_NOTATION: re.Pattern[str] = re.compile(r"\[[^\[\]\n]*\]|\{[^{}]*\|[^{}]*\}|\.\.\.")
BLOCK_DIGEST_WIDTH: int = 12
# Walked rules with syntax that no [[gap]] row can report; each needs an [[unseen]] row.
# Lexer tokens the walked grammar uses whose spelling the gap walk does not compare: literals,
# quoted names and strings, code bodies, hints, subscripts, markers and separators. Each needs a
# reviewed [[unseen]] row naming where the pages document it; a missing row, or a listed token the
# walked grammar no longer uses, is a problem.
UNSEEN_TOKENS: tuple[str, ...] = (
    "BACKQUOTED_IDENTIFIER",
    "BEGIN_DOLLAR_QUOTED_STRING",
    "BIGDECIMAL_LITERAL",
    "BIGINT_LITERAL",
    "DECIMAL_VALUE",
    "DOLLAR_QUOTED_STRING_BODY",
    "DOUBLEQUOTED_STRING",
    "DOUBLE_LITERAL",
    "END_DOLLAR_QUOTED_STRING",
    "EXPONENT_VALUE",
    "FLOAT_LITERAL",
    "HENT_END",
    "HENT_START",
    "INTEGER_VALUE",
    "LEFT_BRACKET",
    "QUESTION",
    "RIGHT_BRACKET",
    "SEMICOLON",
    "SMALLINT_LITERAL",
    "STRING_LITERAL",
    "TINYINT_LITERAL",
)
# Lexer tokens that are names or the end of input: they have no spelling of their own to review.
NAME_TOKENS: frozenset[str] = frozenset({"IDENTIFIER", "EOF"})
# Rules that spell a name: inside another rule they spell nothing, as a catalogue placeholder
# does, and their own alternatives are still walked (IDENTIFIER ( ... ) is checked there).
NAME_RULES: frozenset[str] = frozenset(
    {
        "number",
        "integerValue",
        "intervalValue",
        "stringLit",
        "singleStringLit",
        "singleStringLitWithoutMarker",
        "parameterMarker",
        "namedParameterMarker",
        "tableAlias",
        "identifierReference",
        "catalogIdentifierReference",
        "multipartIdentifier",
        "tableIdentifier",
        "functionIdentifier",
        "qualifiedName",
        "identifier",
        "strictIdentifier",
        "simpleIdentifier",
        "simpleStrictIdentifier",
        "errorCapturingIdentifier",
    }
)
# Symbols that only frame other parts; a sequence of these alone checks nothing of its own, so the
# gap walk skips it. Every other symbol, keyword or operator, is checked.
STRUCTURE_SYMBOLS: frozenset[str] = frozenset({ANY_RUN, "(", ")", ",", ".", ""})
# Tokens the audit reads as another token; each needs a reviewed [[synonym]] row in audit.toml.
SYNONYM_TOKENS: dict[str, str] = {"BANG": "NOT", "EQ": "="}
# Spellings of one lexer token that the audit reads as the token's first spelling.
SPELLING_SYNONYMS: dict[str, str] = {"==": "="}
CLAUSE_SUFFIX: str = "_clause"
