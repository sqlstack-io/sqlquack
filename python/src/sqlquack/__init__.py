"""Check Snowflake SQL and convert it into DuckDB SQL, as the sqlquack command line does."""

from __future__ import annotations

from sqlquack._native import __version__
from sqlquack.api.exceptions import SqlQuackError
from sqlquack.api.main.check import check
from sqlquack.api.main.convert import convert
from sqlquack.api.main.udf import udf
from sqlquack.api.models import CheckResult, Conversion, ConvertOptions, Diagnostic, Span

__all__ = [
    "CheckResult",
    "Conversion",
    "ConvertOptions",
    "Diagnostic",
    "Span",
    "SqlQuackError",
    "__version__",
    "check",
    "convert",
    "udf",
]
