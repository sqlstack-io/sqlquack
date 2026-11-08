"""The DuckDB referee's fixed names: formats, capture, phases and the engine code of each error."""

from __future__ import annotations

from typing import Literal

ENGINE: str = "duckdb"
ENGINE_VERSION: str = "2.0.0"
RECEIPT_FORMAT: str = "sqlquack-syntax-receipt/2"
RUN_AUDIT_FORMAT: str = "sqlquack-duckdb-run-audit/1"
CAPTURE: str = "execute_autocommit_in_throwaway_database"
PARSER_CHECK: str = "whole_text"
RUN_PREFIX: str = "duckdb-2.0.0-"
TIMEOUT_SECONDS: float = 30.0

ACCEPTED: str = "accepted"
SYNTAX_ERROR: str = "syntax_error"
SEMANTIC_ERROR: str = "semantic_error"
RUNTIME_ERROR: str = "runtime_error"
UNSUPPORTED_FEATURE: str = "unsupported_feature"
OUTCOME_ACCEPTED: str = "accepted"
OUTCOME_ERROR: str = "error"
BASIS_PARSE: str = "extract_statements raised {name}"
BASIS_RAN: str = "ran"
BASIS_EXECUTE: str = "execution raised {name}"
BASIS_BIND: str = "execution raised {name}, and so does EXPLAIN, so binding raised it"
BASIS_TIMEOUT: str = (
    "the {seconds:g} s budget for setup, parsing, execution and EXPLAIN ran out after the "
    "parse-only pass, and the child process running the case was stopped"
)
MESSAGE_TIMEOUT: str = "no result within {seconds:g} s"
EXPLAIN: str = "EXPLAIN "
QUERY_CONTEXT: str = "\n\nLINE "
PARSER_ERROR: str = "ParserException"
START_METHOD: Literal["spawn"] = "spawn"
MESSAGE_SET_UP: str = "set up"
MESSAGE_PARSED: str = "parsed"
STAGE_SETUP: str = "setup"
STAGE_PARSE: str = "the parse-only pass"
MESSAGE_DONE: str = "done"
MESSAGE_REFUSED: str = "refused"
TIMEOUT_CODE: str = "TMOUT"
PERMISSION_CODE: str = "PERMS"
EXTERNAL_ACCESS_DISABLED: str = "disabled by configuration"

ENGINE_CODES: dict[str, str] = {
    "ParserException": "PARSE",
    "BinderException": "BINDR",
    "CatalogException": "CATLG",
    "ConversionException": "CONVR",
    "NotImplementedException": "NOIMP",
    "InvalidInputException": "INVIN",
    "OutOfRangeException": "RANGE",
    "ConstraintException": "CONST",
    "PermissionException": "PERMS",
    "TransactionException": "TRANS",
}
PHASES: dict[str, str] = {
    "ParserException": SYNTAX_ERROR,
    "BinderException": SEMANTIC_ERROR,
    "CatalogException": SEMANTIC_ERROR,
    "ConversionException": RUNTIME_ERROR,
    "NotImplementedException": UNSUPPORTED_FEATURE,
    "InvalidInputException": RUNTIME_ERROR,
    "OutOfRangeException": RUNTIME_ERROR,
    "ConstraintException": RUNTIME_ERROR,
    "PermissionException": RUNTIME_ERROR,
    "TransactionException": RUNTIME_ERROR,
}
EXTENSION_LOAD_CODE: str = "EXTLD"
EXTENSION_LOAD_PREFIX: str = (
    "An error occurred while trying to automatically install the required extension"
)
BASE_ERROR: str = "Error"
COMPARED_KEYS: tuple[str, ...] = ("phase", "engine_code", "parse_only_sqlstate", "capture_method")
COMMAND_RUN: str = "run"
COMMAND_COMPARE: str = "compare"
SHOWN_DIFFERENCES: int = 20
