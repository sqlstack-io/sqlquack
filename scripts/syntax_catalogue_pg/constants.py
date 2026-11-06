"""Settings for building the documented PostgreSQL syntax catalogue."""

from __future__ import annotations

import re

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
ID_PREFIX: str = "pg"
PG_VERSION: str = "18.6"
TARBALL_NAME: str = "postgresql-18.6.tar.bz2"
TARBALL_URL: str = "https://ftp.postgresql.org/pub/source/v18.6/postgresql-18.6.tar.bz2"
TARBALL_SHA256: str = "555610c24d53e4316da5b7d3fc25c279d96856d5e0e23ee308c328c5fa881d9f"
TREE_ROOT: str = "postgresql-18.6"
DOCS_URL: str = "https://www.postgresql.org/docs/18/{id}.html"
SGML_DIR: str = "doc/src/sgml"
REF_DIR: str = "doc/src/sgml/ref"
GRAM_Y: str = "src/backend/parser/gram.y"
KWLIST_H: str = "src/include/parser/kwlist.h"
INDEX_FILES: tuple[str, ...] = (
    "doc/src/sgml/reference.sgml",
    "doc/src/sgml/ref/allfiles.sgml",
    "doc/src/sgml/filelist.sgml",
    "doc/src/sgml/postgres.sgml",
    GRAM_Y,
    KWLIST_H,
)
CHAPTERS: tuple[str, ...] = (
    "syntax.sgml",
    "ddl.sgml",
    "dml.sgml",
    "queries.sgml",
    "datatype.sgml",
    "json.sgml",
    "array.sgml",
    "rowtypes.sgml",
    "rangetypes.sgml",
    "func.sgml",
    "typeconv.sgml",
    "indices.sgml",
    "textsearch.sgml",
    "mvcc.sgml",
    "perform.sgml",
    "parallel.sgml",
)
FUNC_FILE: str = "func.sgml"
CHAPTER_FAMILIES: dict[str, str] = {
    "syntax.sgml": "expressions",
    "ddl.sgml": "ddl",
    "dml.sgml": "dml",
    "queries.sgml": "query",
    "datatype.sgml": "expressions",
    "json.sgml": "expressions",
    "array.sgml": "expressions",
    "rowtypes.sgml": "expressions",
    "rangetypes.sgml": "expressions",
    "func.sgml": "functions",
    "typeconv.sgml": "expressions",
    "indices.sgml": "ddl",
    "textsearch.sgml": "functions",
    "mvcc.sgml": "transaction_session",
    "perform.sgml": "utility",
    "parallel.sgml": "utility",
}

KIND_STATEMENT: str = "statement"
KIND_DEFINITION: str = "definition"
KIND_CALL: str = "call"
KIND_SPECIAL: str = "special"
STATUS_FORMS: str = "forms"
STATUS_RESTATED: str = "restated"
STATUS_NO_SYNTAX: str = "no_syntax_block"
STATUS_OUT_OF_SCOPE: str = "out_of_scope"
STATUS_CATALOGUED: str = "catalogued"
DEFAULT_ANCHOR: str = "Synopsis"
SYNOPSIS_HEADING: str = "Synopsis"
SYNOPSIS_DIV: str = "refsynopsisdiv"
ACTION_PLACEHOLDER: str = "action"
OPENERS: str = "[{("
CLOSERS: str = "]})"
BAR: str = "|"

DECISION_STATEMENTS: str = "synopsis split into one form per statement at unindented lines"
DECISION_ACTIONS: str = "statement taking `action [, ...]` split into one form per action"
DECISION_DEFINITIONS: str = "`where X is:` phrases read as sub-grammar definitions"
DECISION_SIGNATURES: str = "signatures of one function name in one section form one call form"
DECISION_SPECIAL: str = "signature with its own keyword grammar catalogued as a special form"
DECISION_OPERATORS: str = "operator signatures grouped by operator and section"
DECISION_HAND: str = "catalogued by hand from {basis}"
DECISION_RESTATED: str = (
    "every synopsis block restates syntax catalogued on another page; listed in also_sources"
)
DECISION_BLOCKS: str = "synopsis blocks dispositioned in hand.toml"
DECISION_MERGED: str = "identical grammar documented twice on the page kept once"

HAND_BASES: frozenset[str] = frozenset(
    {"synopsis", "table", "prose", "heading", "inline_code", "example", "func_signature"}
)
HAND_KINDS: frozenset[str] = frozenset({KIND_STATEMENT, KIND_CALL, KIND_SPECIAL})
BLOCK_DISPOSITIONS: frozenset[str] = frozenset({"form", "restated", "fragment"})
SIGNATURE_DISPOSITIONS: frozenset[str] = frozenset({"out_of_scope"})

QUERY_TITLES: re.Pattern[str] = re.compile(r"^(SELECT|SELECT INTO|VALUES)$")
DML_TITLES: re.Pattern[str] = re.compile(r"^(INSERT|UPDATE|DELETE|MERGE|COPY|TRUNCATE)$")
SESSION_TITLES: re.Pattern[str] = re.compile(
    r"^(ABORT|BEGIN|COMMIT|COMMIT PREPARED|END|PREPARE TRANSACTION|RELEASE SAVEPOINT|ROLLBACK|"
    r"ROLLBACK PREPARED|ROLLBACK TO SAVEPOINT|SAVEPOINT|SET|SET CONSTRAINTS|SET TRANSACTION|SHOW|"
    r"RESET|START TRANSACTION|DISCARD|LOCK|SET ROLE|SET SESSION AUTHORIZATION)$"
)
UTILITY_TITLES: re.Pattern[str] = re.compile(
    r"^(EXPLAIN|CALL|DO|EXECUTE|PREPARE|DEALLOCATE|DECLARE|FETCH|MOVE|CLOSE|LISTEN|NOTIFY|"
    r"UNLISTEN|LOAD|VACUUM|ANALYZE|CLUSTER|REINDEX|CHECKPOINT|REFRESH MATERIALIZED VIEW)$"
)
ADMIN_TITLES: re.Pattern[str] = re.compile(
    r"^(GRANT|REVOKE|ALTER DEFAULT PRIVILEGES|REASSIGN OWNED|DROP OWNED|SECURITY LABEL|"
    r"ALTER SYSTEM|(CREATE|ALTER|DROP) (ROLE|USER|GROUP|POLICY|USER MAPPING))$"
)

SYNOPSIS: re.Pattern[str] = re.compile(r"<synopsis>(.*?)</synopsis>", re.S)
REFSYNOPSIS: re.Pattern[str] = re.compile(r"<refsynopsisdiv>(.*?)</refsynopsisdiv>", re.S)
REFENTRY_ID: re.Pattern[str] = re.compile(r'<refentry id="([^"]+)"')
REFENTRY_TITLE: re.Pattern[str] = re.compile(r"<refentrytitle>(.*?)</refentrytitle>", re.S)
SECTION_OPEN: re.Pattern[str] = re.compile(r'<(sect[1-5]|simplesect|chapter)\s+id="([^"]+)"')
SECTION_TAG: re.Pattern[str] = re.compile(
    r'<(/?)(sect[1-5]|chapter|refsect[1-3]|refsynopsisdiv)\b(?:\s+id="([^"]+)")?[^>]*>'
)
PHRASE_KEEP: re.Pattern[str] = re.compile(r"<phrase>(.*?)</phrase>", re.S)
ENTITY_FILE: re.Pattern[str] = re.compile(r'<!ENTITY\s+(\S+)\s+SYSTEM\s+"([^"]+)">')
COMMANDS_PART: re.Pattern[str] = re.compile(r'<reference id="sql-commands">(.*?)</reference>', re.S)
ENTITY_REF: re.Pattern[str] = re.compile(r"&([A-Za-z][\w.-]*);")
SECT1: re.Pattern[str] = re.compile(r'<sect1\s+id="([^"]+)"[^>]*>.*?</sect1>', re.S)
CHAPTER_ID: re.Pattern[str] = re.compile(r'<(chapter|sect1)\s+id="([^"]+)"')
TITLE: re.Pattern[str] = re.compile(r"<title>(.*?)</title>", re.S)
PHRASE_LINE: re.Pattern[str] = re.compile(r"^\s*<phrase>(.*?)</phrase>\s*$")
WHERE_PHRASE: re.Pattern[str] = re.compile(
    r"^(?:(?:where|and|or)\s+)?(?P<names>.+?)(?:\s+in\s+.+?)?\s+"
    r"(?P<verb>is one of|can be one of|is|can be|are)\s*:$",
    re.I,
)
FUNC_SIGNATURE: re.Pattern[str] = re.compile(r'<para role="func_signature">(.*?)</para>', re.S)
INDEXTERM: re.Pattern[str] = re.compile(r"<indexterm\b.*?</indexterm>", re.S)
RETURNVALUE: re.Pattern[str] = re.compile(r"<returnvalue>.*?</returnvalue>", re.S)
RETURN_ONWARD: re.Pattern[str] = re.compile(r"<returnvalue>.*", re.S)
COMMENT: re.Pattern[str] = re.compile(r"<!--.*?-->", re.S)
REPLACEABLE: re.Pattern[str] = re.compile(r"<replaceable\b[^>]*>(.*?)</replaceable>", re.S)
PARAM_TYPE: re.Pattern[str] = re.compile(
    r"<parameter>(.*?)</parameter>\s*<(type|replaceable)>(.*?)</\2>", re.S
)
PARAMETER: re.Pattern[str] = re.compile(r"<parameter>(.*?)</parameter>", re.S)
TYPE_TAG: re.Pattern[str] = re.compile(r"<type>(.*?)</type>", re.S)
FUNCTION_TAG: re.Pattern[str] = re.compile(r"<function>(.*?)</function>", re.S)
ANGLES: dict[int, str] = str.maketrans({"\x00": "<", "\x01": ">", "\x04": "<", "\x05": ">"})
LITERAL_ANGLES: dict[int, str] = str.maketrans({"<": "\x04", ">": "\x05"})
LITERAL_TAG: re.Pattern[str] = re.compile(r"<literal>(.*?)</literal>", re.S)
TAG: re.Pattern[str] = re.compile(r"<[^>]+>")
ENTITIES: dict[str, str] = {
    "&lt;": "<",
    "&gt;": ">",
    "&amp;": "&",
    "&zwsp;": "",
    "&nbsp;": " ",
    "&mdash;": "-",
    "&ndash;": "-",
    "&quot;": '"',
    "&pi;": "pi",
    "&eacute;": "e",
}
HEADER_SIGNATURES: frozenset[str] = frozenset(
    {
        "Function",
        "Operator",
        "Predicate",
        "Function/Operator",
        "Function signature",
        "Operator/Method",
        "Predicate/Value",
    }
)
CALL_NAME: re.Pattern[str] = re.compile(r"^\s*([A-Za-z_][\w$.]*)\s*\(")
FUNCTION_WORD: re.Pattern[str] = re.compile(r"^[A-Za-z_][\w$.]*$")
OPERATOR_CHARS: dict[str, str] = {
    "+": "plus",
    "-": "minus",
    "*": "star",
    "/": "slash",
    "%": "percent",
    "^": "caret",
    "<": "lt",
    ">": "gt",
    "=": "eq",
    "~": "tilde",
    "!": "bang",
    "@": "at",
    "#": "hash",
    "&": "amp",
    "|": "bar",
    "?": "qmark",
    "`": "tick",
    ":": "colon",
    "[": "lbracket",
    "]": "rbracket",
    ".": "dot",
}
TYPE_TABLES: dict[str, tuple[tuple[str, str, bool], ...]] = {
    "datatype": (("datatype-table", "Data Types table", True),),
    "datatype-oid": (("datatype-oid-table", "Object Identifier Types table", False),),
    "datatype-pseudo": (("datatype-pseudotypes-table", "Pseudo-Types table", False),),
    "datatype-character": (
        ("datatype-character-special-table", "Special Character Types table", False),
    ),
}
USER_AGENT: str = "sqlquack-syntax-catalogue-pg/1"
UTC_FORMAT: str = "%Y-%m-%dT%H:%M:%SZ"

FUNCTIONS_FAMILY: str = "functions"
FUNCTIONS_ID_PREFIX: str = "functions-"
DISPOSITION_FORM: str = "form"
DISPOSITION_CALLS: str = "calls"
DISPOSITION_DEFINITION: str = "definition"
DISPOSITION_RESTATED: str = "restated"
ANCHOR_FIELD: str = "anchor"
CLASH_SIZE: int = 2
RETURN_TAIL: re.Pattern[str] = re.compile(r"\)\s*(?:returns\s+)?[A-Za-z_][\w \[\]]*$")
TYPE_ROW: re.Pattern[str] = re.compile(r"<row>\s*<entry>(.*?)</entry>\s*<entry>(.*?)</entry>", re.S)
TABLE: str = r'<table id="{id}">(.*?)</table>'
TBODY: re.Pattern[str] = re.compile(r"<tbody>(.*?)</tbody>", re.S)

FETCH_RECORD: str = "fetch.json"
AUDIT_FILE: str = "audit.toml"

RULES_START: str = "\n%%\n"
KEYWORD_LINE: re.Pattern[str] = re.compile(r'^PG_KEYWORD\("([^"]+)", (\w+), (\w+), (\w+)\)', re.M)
RULE_HEAD: re.Pattern[str] = re.compile(r"^([a-zA-Z_]\w*):", re.M)
TOKEN: re.Pattern[str] = re.compile(r"'(?:\\.|[^'])'|[A-Za-z_]\w*|%prec\s+\w+")
PUNCTUATION_TOKENS: dict[str, str] = {
    "TYPECAST": "::",
    "DOT_DOT": "..",
    "COLON_EQUALS": ":=",
    "EQUALS_GREATER": "=>",
    "LESS_EQUALS": "<=",
    "GREATER_EQUALS": ">=",
    "NOT_EQUALS": "<>",
}
SINGLE_QUOTE: str = "'"
QUOTE_CHARS: str = "\"'"
BACKSLASH: str = "\\"
OPEN_BRACE: str = "{"
CLOSE_BRACE: str = "}"
OPEN_BRACKET: str = "["
CLOSE_BRACKET: str = "]"
NAMED_ARGUMENT: str = "=>"
CAST_OPERATOR: str = "::"
WORD_JOINER: str = "_"

LABEL_WORDS: int = 4
CHOICE: re.Pattern[str] = re.compile(r"\{([^{}]*)\}")
PLAIN_CALL_WORDS: frozenset[str] = frozenset({"VARIADIC", "DEFAULT"})
ALTERNATIVE_VERBS: frozenset[str] = frozenset({"is one of", "can be one of", "can be"})

BLOCK_KINDS: frozenset[str] = frozenset(
    {"form", "calls", "definition", "restated", "fragment", "hand"}
)
BLOCK_KEYS: frozenset[str] = frozenset(
    {
        "source",
        "block_sha256",
        "first_line",
        "disposition",
        "kind",
        "family",
        "label",
        "labels",
        "whole",
        "names",
        "listed",
        "forms",
        "reason",
    }
)
FORM_KEYS: frozenset[str] = frozenset(
    {"source", "anchor", "kind", "label", "family", "grammar", "basis", "page_sha256"}
)
SECTION_KEYS: frozenset[str] = frozenset({"source", "anchor", "reason"})
PAGE_KEYS: frozenset[str] = frozenset({"source", "reason"})
FOLD_KEYS: frozenset[str] = frozenset({"source", "forms", "reason"})

STATEMENT_RULE: str = "stmt"
TOPLEVEL_RULE: str = "toplevel_stmt"
EXPRESSION_RULES: tuple[str, ...] = (
    "a_expr",
    "b_expr",
    "c_expr",
    "func_expr_common_subexpr",
    "func_expr_windowless",
    "AexprConst",
    "SimpleTypename",
    "GenericType",
    "Numeric",
    "Bit",
    "Character",
    "ConstDatetime",
    "ConstInterval",
    "json_aggregate_func",
    "xmltable",
    "json_table",
    "table_ref",
    "joined_table",
    "select_clause",
    "simple_select",
)
MAX_SET: int = 60
LEAD_TOKEN: re.Pattern[str] = re.compile(r"<[^<>]*>|\{|[A-Za-z_][A-Za-z0-9_]*")
MIN_CLAUSE_KEYWORDS: int = 2
KEYWORD_LISTS: frozenset[str] = frozenset(
    {
        "unreserved_keyword",
        "col_name_keyword",
        "type_func_name_keyword",
        "reserved_keyword",
        "bare_label_keyword",
    }
)
PLACEHOLDER: re.Pattern[str] = re.compile(r"<([^<>]+)>")
DEFINITION_HEAD: re.Pattern[str] = re.compile(r"^\\s*<[^<>]+>\\s*::=\\s*")
MAX_EXPANSION: int = 64
MAX_KEYWORD_ALTERNATIVE: int = 4

EXTRA_LEADS: dict[str, frozenset[str]] = {
    "SELECT": frozenset({"TABLE", "WITH"}),
    "SET": frozenset({"RESET"}),
}
