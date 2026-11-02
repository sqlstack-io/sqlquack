"""Where referee files live, what the generators name their statements, and their data files."""

from __future__ import annotations

REFEREE_DIRECTORY: str = "referee"
GENERATED_DIRECTORY: str = "generated"
ENVIRONMENTS_DIRECTORY: str = "environments"
DATA_DIRECTORY: str = "data"
ENVIRONMENTS_DATA: str = "environments.toml"
FUNCTIONS_DATA: str = "functions.toml"
NAMES_DATA: str = "names.toml"
STRUCTURED_DATA: str = "structured.toml"
REFEREE_FORMAT: int = 1
SOURCE_DIALECT: str = "snowflake"
GENERATED_ENVIRONMENT: str = "generated"
STRUCTURED_ENVIRONMENT: str = "generated_structured"
FUNCTIONS_ORIGIN: str = "referee_functions@1"
CLAUSES_ORIGIN: str = "referee_clauses@1"
NAMES_ORIGIN: str = "referee_names@1"
MUTANTS_ORIGIN: str = "referee_mutants@1"
STRUCTURED_ORIGIN: str = "referee_structured@1"
VALID_KIND: str = "valid"
SCENARIO_KIND: str = "scenario"
ARITY_KIND: str = "arity"
TYPES_KIND: str = "types"
MUTANT_KIND: str = "mutant"
WINDOW_SUFFIX: str = " OVER (ORDER BY c_number)"
SCALAR_SHAPE: str = "scalar"
WINDOW_SHAPE: str = "window"
ARGUMENT_SEPARATOR: str = ", "
PLACEHOLDER_ARGUMENT: str = "1"
QUOTE_CHARACTERS: frozenset[str] = frozenset({"'", '"'})
CALL_OPEN: str = "("
JOIN_KEYWORD: str = " JOIN "
ALL_GROUP_KEY: str = "all"
TYPE_COLUMN_PREFIX: str = "c_"
SHORTEST_MISSPELLING: int = 3
MISSPELLING_SUFFIX: str = "x"
TEMPORAL_PREFIXES: tuple[str, ...] = ("TIMESTAMP", "DATE", "TIME")
BOOLEAN_TYPE: str = "BOOLEAN"
SHARED_COLUMNS: frozenset[str] = frozenset(
    {"customer_id", "region", "order_id", "product_id", "name"}
)
TYPE_COLUMN_REPLACEMENTS: tuple[str, str] = ("c_boolean", "c_array")
ORDERS_COLUMN_REPLACEMENTS: tuple[str, str] = ("placed_at", "details")
STATEMENT_LINE_PREFIX: str = '\n  {"'
