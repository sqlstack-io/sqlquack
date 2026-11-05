"""Constants of the formatter conformance measurement."""

from __future__ import annotations

CASES_DIR: str = "cases"
SQLCLEAN_CONFIG: str = 'dialect = "duckdb"\n\n[format]\nindent = 4\nline_width = 100\n'
DIFFS_SHOWN: int = 5
QUERY_KEY: str = "query"
COLUMNS_KEY: str = "columns"
INPUT_RELATION: str = "INPUT"
QUOTE: str = '"'
