"""Where the DuckDB function catalogue goes and how it is read from the engine."""

from __future__ import annotations

from pathlib import Path

DATA_DIR: Path = Path("crates/sqlquack-semantics/src/duckdb/lineage/_helpers/duckdb_catalog")
BUILT_IN_FUNCTIONS_QUERY: str = (
    "SELECT DISTINCT lower(function_name) AS name, function_type AS kind, "
    "CASE WHEN function_type IN ('scalar', 'aggregate', 'window') "
    "THEN len(parameters) END AS arguments, varargs IS NOT NULL AS variadic "
    "FROM duckdb_functions() WHERE function_type <> 'pragma' "
    "AND regexp_full_match(function_name, '[A-Za-z_][A-Za-z0-9_]*') ORDER BY ALL"
)
PARSER_KEYWORDS_QUERY: str = "SELECT lower(keyword_name) FROM duckdb_keywords() ORDER BY 1"
TRANSFORMER_REWRITTEN_NAMES: tuple[str, ...] = ("ifnull", "try", "unpack", "grouping_id")
GROUP_BY_CONSTRUCT_NAMES: tuple[str, ...] = ("rollup", "cube")
PROBE_TABLE: str = "CREATE TABLE probe (a INTEGER, b VARCHAR)"
PROBE_ARGUMENTS: tuple[str, ...] = ("a", "a, 1", "b, b, 1")
UNKNOWN_FUNCTION_MARKERS: tuple[str, ...] = ("does not exist", "not in the catalog")
SYNTAX_MARKER: str = "Parser Error: syntax error"
