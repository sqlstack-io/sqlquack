"""Pinned Databricks SQL sources and the rules that turn them into catalogue forms."""

from __future__ import annotations

import itertools
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
IDENTIFIER_RULES: frozenset[str] = frozenset(
    {"nonReserved", "ansiNonReserved", "strictNonReserved"}
)
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
EXAMPLE_HEADING: re.Pattern[str] = re.compile(r"(?i)^examples?\b")
EXAMPLE_PROMPT: re.Pattern[str] = re.compile(r"^\s*>", re.MULTILINE)
QUOTED_TEXT: re.Pattern[str] = re.compile(r"'[^'\n]*'|\"[^\"\n]*\"")
SYNTAX_NOTATION: re.Pattern[str] = re.compile(r"\[[^\[\]\n]*\]|\{[^{}]*\|[^{}]*\}|\.\.\.")
BLOCK_DIGEST_WIDTH: int = 12
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
NAME_TOKENS: frozenset[str] = frozenset({"IDENTIFIER", "EOF"})
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
STRUCTURE_SYMBOLS: frozenset[str] = frozenset({ANY_RUN, "(", ")", ",", ".", ""})
SYNONYM_TOKENS: dict[str, str] = {"BANG": "NOT", "EQ": "="}
SPELLING_SYNONYMS: dict[str, str] = {"==": "="}
CLAUSE_SUFFIX: str = "_clause"
TOKEN_SYNONYMS: dict[str, str] = {
    token: target for token, target in SYNONYM_TOKENS.items() if target.isalpha()
}
KIND_STATEMENT: str = "statement"
KIND_SPECIAL: str = "special"
INDEX_LINKED_KEY: str = "index_linked"
LINE_TOKEN: re.Pattern[str] = re.compile(
    r"\\[|\[\]{}]|<[A-Za-z_][^<>]*>|\.\.\.|\"[^\"]*\"|'[^']*'|<=>|>>>|>>|<<|==|<=|>=|<>|!=|!>|!<"
    r"|\|\||\|>|::|->|=>|[\[\]{}|]|[A-Za-z_][A-Za-z0-9_]*|\S"
)
PUNCTUATION: frozenset[str] = frozenset(itertools.chain.from_iterable(PUNCTUATION_TOKENS.values()))
ESCAPED_PIPE: str = "\\|"
PLACEHOLDER_NAME: re.Pattern[str] = re.compile(r"^<([^<>\s][^<>]*)>$")
REPEAT: str = "..."
CLOSERS: dict[str, str] = {"[": "]", "{": "}"}
OPTIONAL_OPEN: str = "["
CHOICE_BAR: str = "|"
BRACKET_ENDS: tuple[str, str] = ("]", "}")
LINE_STARTERS: tuple[str, ...] = ("(", ",", "{", "|")
EMPTY_ANGLE_LENGTH: int = 2
G4_LEXEME: re.Pattern[str] = re.compile(
    r"(?P<word>[A-Za-z_][A-Za-z0-9_]*)|(?P<quoted>'(?:\\.|[^'\\])*')|(?P<punct>\+=|[:;|()?*+=#~.@])"
)
G4_LITERAL: re.Pattern[str] = re.compile(r"'(?:\\.|[^'\\])*'")
G4_WORD: str = "word"
G4_QUOTED: str = "quoted"
G4_PUNCT: str = "punct"
G4_ACTION: str = "action"
G4_QUOTE: str = "'"
G4_QUOTES: str = "\"'"
G4_BACKSLASH: str = "\\"
G4_ACTION_OPEN: str = "{"
G4_ACTION_CLOSE: str = "}"
G4_PARSER_HEADER: tuple[str, str] = (G4_WORD, "parser")
G4_HEADER_SECTIONS: tuple[tuple[str, str], ...] = ((G4_WORD, "options"), (G4_PUNCT, "@"))
G4_RULE_END: str = ";"
G4_RULE_COLON: tuple[str, str] = (G4_PUNCT, ":")
G4_BAR: tuple[str, str] = (G4_PUNCT, "|")
G4_LABEL_MARK: tuple[str, str] = (G4_PUNCT, "#")
G4_GROUP_OPEN: tuple[str, str] = (G4_PUNCT, "(")
G4_GROUP_CLOSE: str = ")"
G4_NOT: tuple[str, str] = (G4_PUNCT, "~")
G4_WILDCARD: tuple[str, str] = (G4_PUNCT, ".")
G4_NON_GREEDY: tuple[str, str] = (G4_PUNCT, "?")
DEFINITION_SEPARATOR: str = " ::= "
