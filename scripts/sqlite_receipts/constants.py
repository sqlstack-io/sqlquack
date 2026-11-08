"""The SQLite referee's fixed names: the pinned engine, capture, phases and error classes."""

from __future__ import annotations

import re
from typing import Literal

ENGINE: str = "sqlite"
ENGINE_VERSION: str = "3.54.1"
SOURCE_ID: str = (
    "2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76"
)
RECEIPT_FORMAT: str = "sqlquack-syntax-receipt/2"
RUN_AUDIT_FORMAT: str = "sqlquack-sqlite-run-audit/1"
CAPTURE: str = "execute_in_rolled_back_transaction"
CAPTURE_AUTOCOMMIT: str = "execute_autocommit_in_throwaway_database"
CAPTURES: frozenset[str] = frozenset({CAPTURE, CAPTURE_AUTOCOMMIT})
PARSER_CHECK: str = "whole_text"
RUN_PREFIX: str = "sqlite-3.54.1-"
TIMEOUT_SECONDS: float = 30.0
FETCH_RECORD: str = "fetch.json"
ENGINE_DIR: str = "engine"
LIBRARY_FILE: str = "libsqlite3.so"
LIBRARY_KEY: str = "library_sha256"

SQLITE_OK: int = 0
SQLITE_ROW: int = 100
SQLITE_DONE: int = 101
OPEN_FLAGS: int = 0x02 | 0x04
BEGIN: str = "BEGIN"
ROLLBACK: str = "ROLLBACK"

ACCEPTED: str = "accepted"
SYNTAX_ERROR: str = "syntax_error"
SEMANTIC_ERROR: str = "semantic_error"
RUNTIME_ERROR: str = "runtime_error"
UNSUPPORTED_FEATURE: str = "unsupported_feature"
OUTCOME_ACCEPTED: str = "accepted"
OUTCOME_ERROR: str = "error"
BASIS_PARSE: str = "sqlite3_prepare_v2 of statement {index} raised {code}"
BASIS_PREPARE: str = (
    "sqlite3_prepare_v2 of statement {index}, after the statements before it ran, raised {code}"
)
BASIS_STEP: str = "sqlite3_step of statement {index} raised {code}"
BASIS_RAN: str = "ran in a rolled-back transaction"
BASIS_RAN_AUTOCOMMIT: str = "ran in autocommit in a throwaway in-memory database"
BASIS_TIMEOUT: str = (
    "the {seconds:g} s budget for setup, the parse-only pass and execution ran out after every "
    "statement had prepared, and the child process running the case was stopped"
)
MESSAGE_TIMEOUT: str = "no result within {seconds:g} s"
MESSAGE_FORMAT: str = "{name} ({code}): {message}"
START_METHOD: Literal["forkserver"] = "forkserver"
MESSAGE_SET_UP: str = "set up"
MESSAGE_PARSED: str = "parsed"
STAGE_PARSE: str = "setup and the parse-only pass"
STAGE_EXECUTE: str = "execution"
MESSAGE_DONE: str = "done"
MESSAGE_REFUSED: str = "refused"
TIMEOUT_CODE: str = "TMOUT"
PASS_PARSE: str = "parse"
PASS_PREPARE: str = "prepare"
PASS_STEP: str = "step"
COMPARED_KEYS: tuple[str, ...] = ("phase", "engine_code", "parse_only_sqlstate", "capture_method")
COMMAND_RUN: str = "run"
COMMAND_COMPARE: str = "compare"
COMMAND_CHECK: str = "check"
EXPECT_ACCEPT: str = "accept"
EXPECT_REJECT: str = "reject"
SHOWN_DIFFERENCES: int = 20
CHILD_DIR_PREFIX: str = "sqlite-referee-"
BASIS_EMPTY: str = "sqlite3_prepare_v2 found no statement in the text, which SQLite accepts"
BASIS_PARSE_LATE: str = (
    "sqlite3_prepare_v2 of statement {index}, after the statements before it ran, raised the "
    "parser's {code}"
)
PRIMARY_MASK: int = 0xFF
SQLITE_ERROR: int = 1
SEMANTIC_CODE: str = "E{code:04d}"
UNSUPPORTED_CODE: str = "NOFTR"

SYNTAX_MESSAGES: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r'near ".*": syntax error', re.DOTALL), "SYNTX"),
    (re.compile(r"incomplete input"), "INCMP"),
    (re.compile(r'unrecognized token: ".*"', re.DOTALL), "UNTOK"),
    (re.compile(r"unknown table option: .*", re.DOTALL), "GRAMR"),
    (re.compile(r"(?:ORDER BY|LIMIT) clause should come after \S+(?: ALL)? not before"), "GRAMR"),
    (re.compile(r'syntax error after column name ".*"', re.DOTALL), "GRAMR"),
    (re.compile(r"unknown join type: .*", re.DOTALL), "GRAMR"),
    (
        re.compile(
            r"the (?:INDEXED BY|NOT INDEXED) clause is not allowed on UPDATE or DELETE "
            r"statements within triggers"
        ),
        "GRAMR",
    ),
    (re.compile(r"duplicate WITH table name: .*", re.DOTALL), "GRAMR"),
    (re.compile(r"unsupported frame specification"), "GRAMR"),
    (re.compile(r"variable number must be between \?1 and \?\d+"), "GRAMR"),
    (re.compile(r"too many terms in compound SELECT"), "GRAMR"),
    (re.compile(r"too many columns on .*", re.DOTALL), "GRAMR"),
    (re.compile(r"missing datatype for \S+"), "GRAMR"),
    (re.compile(r"AUTOINCREMENT is only allowed on an INTEGER PRIMARY KEY"), "GRAMR"),
)
TEXT_ONLY_CODE: str = "GRAMR"
_NAME: str = r"(?:\"(?:[^\"]|\"\")*\"|\[[^\]]*\]|`(?:[^`]|``)*`|'(?:[^']|'')*'|[^\s.(\"\[`']+)"
CREATE_TABLE_COLUMNS: re.Pattern[str] = re.compile(
    rf"\s*CREATE\s+(?:TEMP\s+|TEMPORARY\s+)?TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?"
    rf"(?:{_NAME}\s*\.\s*)?{_NAME}\s*\(",
    re.IGNORECASE,
)
TEXT_ONLY_MESSAGES: tuple[tuple[re.Pattern[str], re.Pattern[str] | None], ...] = (
    (re.compile(r"must have at least one non-generated column"), None),
    (re.compile(r"PRIMARY KEY missing on table .*", re.DOTALL), None),
    (re.compile(r"AUTOINCREMENT not allowed on WITHOUT ROWID tables"), None),
    (re.compile(r"temporary table name must be unqualified"), None),
    (re.compile(r"temporary trigger may not have qualified name"), None),
    (re.compile(r'unknown column ".*" in foreign key definition', re.DOTALL), None),
    (
        re.compile(r"foreign key on .* should reference only one column of table .*", re.DOTALL),
        None,
    ),
    (
        re.compile(
            r"number of columns in foreign key does not match the number of columns in the "
            r"referenced table"
        ),
        None,
    ),
    (re.compile(r"\d+ columns assigned \d+ values"), None),
    (re.compile(r"duplicate column name: .*", re.DOTALL), None),
    (re.compile(r"default value of column \[.*\] is not constant", re.DOTALL), None),
    (re.compile(r'table ".*" has more than one primary key', re.DOTALL), None),
    (re.compile(r"generated columns cannot be part of the PRIMARY KEY"), None),
    (re.compile(r"expressions prohibited in PRIMARY KEY and UNIQUE constraints"), None),
    (
        re.compile(
            r"(?:subqueries|parameters) prohibited in (?:CHECK constraints|generated columns)"
        ),
        None,
    ),
    (re.compile(r"no such column: .*", re.DOTALL), CREATE_TABLE_COLUMNS),
)
RUNTIME_MESSAGES: tuple[re.Pattern[str], ...] = (
    re.compile(r"Safety level may not be changed inside a transaction"),
    re.compile(r"not a writable directory"),
    re.compile(r"Recursion limit"),
    re.compile(r"Expression tree is too large \(maximum depth \d+\)"),
    re.compile(r"too many columns in result set"),
    re.compile(r"too many SQL variables"),
    re.compile(r"cannot (?:commit|rollback) - no transaction is active"),
)
SEMANTIC_MESSAGES: tuple[re.Pattern[str], ...] = (
    re.compile(
        r"no such (?:table|column|view|trigger|index|window|function|collation sequence): .*",
        re.DOTALL,
    ),
    re.compile(r'unknown database ".*"', re.DOTALL),
    re.compile(r"ambiguous column name: .*", re.DOTALL),
    re.compile(r"table .* has \d+ columns but \d+ values were supplied", re.DOTALL),
    re.compile(r"table .* has no column named .*", re.DOTALL),
    re.compile(r"\d+ values for \d+ columns"),
    re.compile(r"sub-select returns \d+ columns - expected \d+"),
    re.compile(r"row value misused"),
    re.compile(r"wrong number of arguments to function .*\(\)", re.DOTALL),
    re.compile(r".*\(\) may not be used as a window function", re.DOTALL),
    re.compile(r"second argument to LIKELIHOOD\(\) must be a constant between 0\.0 and 1\.0"),
    re.compile(r"no tables specified"),
    re.compile(r"cannot join using column .* - column not present in both tables", re.DOTALL),
    re.compile(r"target object/alias may not appear in FROM clause: .*", re.DOTALL),
    re.compile(
        r"(?:\d+(?:st|nd|rd|th) )?ON CONFLICT clause does not match any PRIMARY KEY or UNIQUE"
        r" constraint"
    ),
    re.compile(r"trigger .* already exists", re.DOTALL),
    re.compile(
        r"cannot create (?:BEFORE|AFTER|INSTEAD OF) trigger on (?:table|view): .*", re.DOTALL
    ),
    re.compile(r"unable to identify the object to be reindexed"),
)
UNSUPPORTED_MESSAGES: tuple[re.Pattern[str], ...] = (
    re.compile(r"no such function: (?:soundex|sqlite_offset|regexp)", re.IGNORECASE),
)
