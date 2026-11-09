"""Where the DuckDB function catalogue goes and how it is read from the engine."""

from __future__ import annotations

from pathlib import Path

DATA_DIR: Path = Path("crates/sqlquack-semantics/src/lineage/_helpers/checks/duckdb_catalog")
BUILT_IN_FUNCTIONS_QUERY: str = (
    "SELECT DISTINCT lower(function_name) AS name, function_type AS kind, "
    "CASE WHEN function_type IN ('scalar', 'aggregate', 'window') "
    "THEN len(parameters) END AS arguments, varargs IS NOT NULL AS variadic "
    "FROM duckdb_functions() WHERE function_type <> 'pragma' "
    "AND NOT contains(function_name, ',') ORDER BY ALL"
)
PARSER_KEYWORDS_QUERY: str = "SELECT lower(keyword_name) FROM duckdb_keywords() ORDER BY 1"
TRANSFORMER_REWRITTEN_NAMES: tuple[str, ...] = ("ifnull", "try", "unpack", "grouping_id")
SYNTAX_FORM_PROBES: dict[str, str] = {
    "!": "SELECT a! FROM probe",
    "array": "SELECT ARRAY(SELECT a FROM probe)",
    "cube": "SELECT a FROM probe GROUP BY CUBE (a)",
    "extract": "SELECT EXTRACT(YEAR FROM DATE '2026-01-01') FROM probe",
    "grouping sets": "SELECT a, b FROM probe GROUP BY GROUPING SETS ((a), (b))",
    "rollup": "SELECT a FROM probe GROUP BY ROLLUP (a)",
}
PROBE_TABLE: str = "CREATE TABLE probe (a INTEGER, b VARCHAR)"
PROBE_ARGUMENTS: tuple[str, ...] = ("a", "a, 1", "b, b, 1")
UNKNOWN_FUNCTION_MARKERS: tuple[str, ...] = ("does not exist", "not in the catalog")
SYNTAX_MARKER: str = "Parser Error: syntax error"
