"""The dialects, roles and front doors the layout rules check names against, and their options."""

from __future__ import annotations

import re

from fensu import RuleOption

CRATE_PREFIX: str = "sqlquack"
CRATE_SEPARATOR: str = "-"
SCRIPT_SEPARATOR: str = "_"
PYTHON_SUFFIX: str = ".py"
PRODUCT_ROOT: str = "crates"
TOOLS_ROOT: str = "tools"
TOOL_SUFFIX: str = "check"
SHARED_SYNTAX_CRATE: str = "sqlquack-syntax"
SYNTAX_ROLE: str = "syntax"
DIALECT_ENUM: str = "Dialect"
CATALOG_WORD: str = "catalog"
RUNTIME_DEPENDENCY_KINDS: frozenset[str] = frozenset({"normal", "build"})
VARIANT_SEPARATOR: str = "="
MIN_CRATE_NAME_PARTS: int = 2
MAX_NAMED_DIALECTS: int = 1
NESTED_PATH_PARTS: int = 2
ENUM_BODY: str = r"\benum\s+{name}\b[^{{]*\{{(?P<body>[^}}]*)\}}"
VARIANT: re.Pattern[str] = re.compile(r"^\s*(?:#\[[^\]]*\]\s*)*([A-Z][A-Za-z0-9]*)\b", re.MULTILINE)
LINE_COMMENT: re.Pattern[str] = re.compile(r"//.*$", re.MULTILINE)
BLOCK_COMMENT: re.Pattern[str] = re.compile(r"/\*.*?\*/", re.DOTALL)
STRING_LITERAL: re.Pattern[str] = re.compile(r'"(?:\\.|[^"\\])*"')

DEFAULT_DIALECTS: tuple[str, ...] = (
    "snowflake",
    "duckdb",
    "postgres",
    "sqlite",
    "bigquery",
    "databricks",
    "clickhouse",
    "sqlserver",
)

DIALECTS: RuleOption[tuple[str, ...]] = RuleOption.string_list(
    name="dialects",
    default=DEFAULT_DIALECTS,
    description="Every SQL dialect SQLQuack names, as it appears in crate and script names.",
)

DEFAULT_ROLES: tuple[str, ...] = (
    "syntax",
    "parse",
    "semantics",
    "types",
    "blocks",
    "rules",
    "convert",
    "format",
    "lint",
    "fingerprint",
    "compare",
    "evidence",
    "policy",
    "python",
    "core",
    "derive",
)

ROLES: RuleOption[tuple[str, ...]] = RuleOption.string_list(
    name="roles",
    default=DEFAULT_ROLES,
    description="The roles a product crate name may start with, after sqlquack-.",
)

DEFAULT_FRONT_DOORS: tuple[str, ...] = ("sqlquack", "sqlquack-python", "sqlquack-parse")

FRONT_DOORS: RuleOption[tuple[str, ...]] = RuleOption.string_list(
    name="front_doors",
    default=DEFAULT_FRONT_DOORS,
    description=(
        "Crates that dispatch over every dialect, so they may depend on dialect-named crates "
        "without naming a dialect. Development checks under tools/ always may."
    ),
)

DEFAULT_DIALECT_VARIANTS: tuple[str, ...] = (
    "snowflake=Snowflake",
    "duckdb=DuckDB",
    "postgres=PostgreSQL",
    "sqlite=SQLite",
    "bigquery=BigQuery",
    "databricks=Databricks",
    "clickhouse=ClickHouse",
    "sqlserver=SqlServer",
)

DIALECT_VARIANTS: RuleOption[tuple[str, ...]] = RuleOption.string_list(
    name="dialect_variants",
    default=DEFAULT_DIALECT_VARIANTS,
    description="The Dialect enum variant that stands for each dialect, as dialect=Variant.",
)

NAMED_DIRECTORIES: RuleOption[tuple[str, ...]] = RuleOption.string_list(
    name="named_directories",
    default=("scripts", "tests/unit/scripts"),
    description="Directories whose entries must name every dialect they are specific to.",
)

SHARED_SYNTAX_MODULES: RuleOption[tuple[str, ...]] = RuleOption.string_list(
    name="shared_syntax_modules",
    default=("lib", "expressions", "functions", "guard", "lossless", "tokens", "traversal"),
    description=(
        "The only top-level modules and test folders the shared sqlquack-syntax crate may hold: "
        "the tree, tokens, spans and diagnostics every dialect parser builds on."
    ),
)
