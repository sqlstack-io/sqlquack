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
# The chapters of "The SQL Language" part whose sections document syntax outside the command
# reference; every synopsis block and function signature in them is catalogued or reviewed.
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

# Statement families of command pages, by refentry title.
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
COMMENT: re.Pattern[str] = re.compile(r"<!--.*?-->", re.S)
REPLACEABLE: re.Pattern[str] = re.compile(r"<replaceable\b[^>]*>(.*?)</replaceable>", re.S)
PARAM_TYPE: re.Pattern[str] = re.compile(r"<parameter>(.*?)</parameter>\s*<type>(.*?)</type>", re.S)
PARAMETER: re.Pattern[str] = re.compile(r"<parameter>(.*?)</parameter>", re.S)
TYPE_TAG: re.Pattern[str] = re.compile(r"<type>(.*?)</type>", re.S)
FUNCTION_TAG: re.Pattern[str] = re.compile(r"<function>(.*?)</function>", re.S)
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
# Data type tables catalogued one form per row: (table id, anchor, whether column 2 is aliases).
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
