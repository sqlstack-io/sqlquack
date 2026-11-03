"""Settings for importing Snowflake capture runs as syntax-coverage receipts."""

from __future__ import annotations

RECEIPT_FORMAT: str = "sqlquack-syntax-receipt/1"
PHASES: frozenset[str] = frozenset(
    {"accepted", "syntax_error", "semantic_error", "runtime_error", "unsupported_feature"}
)
SYNTAX_ERRNO: int = 1003
UNSUPPORTED_ERRNO: int = 2
COMPILATION_PREFIX: str = "SQL compilation error"
UNCAUGHT_PREFIX: str = "Uncaught exception of type"
UNSUPPORTED_PREFIX: str = "Unsupported feature"
# Snowflake's 100000-series codes are raised while a statement executes.
RUNTIME_ERRNO_FLOOR: int = 100000
SCRIPTING_SUBMISSION: str = "anonymous block, as-is"
EXPLAIN_METHOD: str = "EXPLAIN USING TEXT, once"
CAPTURE_EXECUTE: str = "execute"
CAPTURE_EXPLAIN: str = "compile_only_explain"
CAPTURE_CANONICAL: str = "execute_canonical_recorder"
# Diagnostic requests in these categories read account metadata; they are not syntax cases.
CONTROLLER_CATEGORIES: frozenset[str] = frozenset({"metadata"})
