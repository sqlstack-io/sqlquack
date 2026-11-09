"""Where the catalogue extract goes and the pg_catalog queries that produce it."""

from __future__ import annotations

from pathlib import Path

DATA_DIR: Path = Path("crates/sqlquack-semantics/src/postgres/lineage/_helpers/postgres_catalog")
QUERIES: dict[str, str] = {
    "pg_proc": (
        "SELECT p.proname AS name, p.prokind AS kind, p.proretset AS returns_set, "
        "format_type(p.prorettype, NULL) AS return_type, "
        "COALESCE((SELECT string_agg(format_type(t, NULL), ',' ORDER BY o) "
        "FROM unnest(p.proargtypes::oid[]) WITH ORDINALITY AS a(t, o)), '') AS argument_types, "
        "CASE WHEN p.provariadic = 0 THEN '' "
        "ELSE format_type(p.provariadic, NULL) END AS variadic, "
        "p.pronargdefaults AS defaults, p.proisstrict AS strict, p.oid AS oid "
        "FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace "
        "WHERE n.nspname = 'pg_catalog' ORDER BY p.proname, p.oid"
    ),
    "pg_cast": (
        "SELECT format_type(c.castsource, NULL) AS source, "
        "format_type(c.casttarget, NULL) AS target, "
        "c.castcontext AS context, c.castmethod AS method FROM pg_cast c ORDER BY 1, 2"
    ),
    "pg_type": (
        "SELECT t.typname AS name, format_type(t.oid, NULL) AS formatted, t.typtype AS kind, "
        "t.typcategory AS category, t.typispreferred AS preferred, "
        "CASE WHEN t.typelem = 0 THEN '' ELSE format_type(t.typelem, NULL) END AS element "
        "FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace "
        "WHERE n.nspname = 'pg_catalog' ORDER BY t.typname"
    ),
    "pg_operator": (
        "SELECT o.oprname AS name, o.oprkind AS kind, "
        "CASE WHEN o.oprleft = 0 THEN '' ELSE format_type(o.oprleft, NULL) END AS left_type, "
        "CASE WHEN o.oprright = 0 THEN '' ELSE format_type(o.oprright, NULL) END AS right_type, "
        "format_type(o.oprresult, NULL) AS result_type FROM pg_operator o ORDER BY 1, 3, 4"
    ),
}

TYPES_FIXTURE: Path = Path(
    "crates/sqlquack-semantics/src/postgres/profile/tests/fixtures/postgres_engine_types.json"
)
SYNTAX_TYPES_FIXTURE: Path = Path(
    "crates/sqlquack-semantics/src/postgres/profile/tests/fixtures/postgres_syntax_types.json"
)
SYNTAX_CASES: Path = Path("coverage/syntax-postgres/cases")
QUERY_HEADS: tuple[str, ...] = ("SELECT", "WITH", "VALUES", "TABLE")
ACCEPTED: str = "accept"
MATRIX_METHOD: str = (
    "format_type of a temporary view's first column over the empty table t, "
    "recorded by scripts/record_postgres_types.py"
)
SYNTAX_METHOD: str = (
    "format_type of a temporary view's first column for each accepted query-shaped case of "
    "coverage/syntax-postgres/cases, over the empty table t, recorded by "
    "scripts/record_postgres_types.py --syntax-cases"
)
TABLE_COLUMNS: tuple[tuple[str, str], ...] = (
    ("c_smallint", "smallint"),
    ("c_integer", "integer"),
    ("c_bigint", "bigint"),
    ("c_num10_2", "numeric(10,2)"),
    ("c_num20_5", "numeric(20,5)"),
    ("c_numeric", "numeric"),
    ("c_real", "real"),
    ("c_double", "double precision"),
    ("c_text", "text"),
    ("c_varchar", "varchar(20)"),
    ("c_char", "char(3)"),
    ("c_boolean", "boolean"),
    ("c_date", "date"),
    ("c_timestamp", "timestamp"),
    ("c_timestamptz", "timestamptz"),
    ("c_time", "time"),
    ("c_interval", "interval"),
    ("c_json", "json"),
    ("c_jsonb", "jsonb"),
    ("c_uuid", "uuid"),
    ("c_bytea", "bytea"),
    ("c_int_array", "integer[]"),
    ("c_text_array", "text[]"),
)
NUMERIC_COLUMNS: tuple[str, ...] = (
    "c_smallint",
    "c_integer",
    "c_bigint",
    "c_num10_2",
    "c_num20_5",
    "c_numeric",
    "c_real",
    "c_double",
)
TEXT_COLUMNS: tuple[str, ...] = ("c_text", "c_varchar", "c_char")
TYPE_MARKER: str = "@@type@@"
TYPE_ERROR_MARKER: str = "@@error@@"
TYPE_FIELD: str = "type"

ARITHMETIC: tuple[str, ...] = ("+", "-", "*", "/", "%")
LITERALS: tuple[str, ...] = ("7", "3000000000", "1.5", "1e3", "'x'", "true", "NULL")
UNIFY: tuple[str, ...] = (
    "c_smallint",
    "c_integer",
    "c_bigint",
    "c_num10_2",
    "c_numeric",
    "c_real",
    "c_double",
    "c_text",
    "c_varchar",
    "c_char",
    "c_date",
    "c_timestamp",
    "'x'",
    "1",
    "NULL",
)
COMPARISONS: tuple[str, ...] = (
    "c_integer = c_bigint",
    "c_integer < c_double",
    "c_text <> 'x'",
    "c_date >= c_timestamp",
    "c_boolean AND c_integer > 1",
    "NOT c_boolean",
    "c_integer IS NULL",
    "c_integer BETWEEN 1 AND 3",
    "c_integer IN (1, 2)",
    "c_text LIKE 'a%'",
    "c_text ILIKE 'a%'",
    "c_text ~ 'a'",
    "c_text SIMILAR TO 'a%'",
    "c_boolean IS TRUE",
    "c_integer IS DISTINCT FROM c_bigint",
    "EXISTS (SELECT 1)",
    "c_integer = ANY (c_int_array)",
    "c_jsonb ? 'a'",
    "c_jsonb @> '{}'",
    "c_int_array && c_int_array",
)
AGGREGATES: tuple[str, ...] = ("sum", "avg", "min", "max", "count", "stddev", "variance")
AGGREGATE_INPUTS: tuple[str, ...] = (
    *NUMERIC_COLUMNS,
    *TEXT_COLUMNS,
    "c_boolean",
    "c_date",
    "c_timestamp",
    "c_interval",
)
DATETIME: tuple[str, ...] = (
    "c_date + 1",
    "c_date - 1",
    "c_date - c_date",
    "c_date + INTERVAL '1 day'",
    "c_timestamp + INTERVAL '1 day'",
    "c_timestamptz - INTERVAL '1 day'",
    "c_timestamp - c_timestamp",
    "c_timestamptz - c_timestamptz",
    "c_date - c_timestamp",
    "c_time + INTERVAL '1 hour'",
    "c_time - c_time",
    "c_date + c_time",
    "c_interval + c_interval",
    "c_interval * 2",
    "c_interval / 2",
    "-c_interval",
    "INTERVAL '1 day'",
    "c_timestamp AT TIME ZONE 'UTC'",
    "c_timestamptz AT TIME ZONE 'UTC'",
)
CAST_TARGETS: tuple[tuple[str, str], ...] = (
    ("'1'", "smallint"),
    ("'1'", "integer"),
    ("'1'", "int"),
    ("'1'", "bigint"),
    ("'1'", "numeric"),
    ("'1'", "numeric(10,2)"),
    ("'1'", "decimal(12,4)"),
    ("'1'", "numeric(5)"),
    ("'1'", "real"),
    ("'1'", "float"),
    ("'1'", "float(10)"),
    ("'1'", "double precision"),
    ("'1'", "text"),
    ("'1'", "varchar"),
    ("'1'", "varchar(10)"),
    ("'1'", "char(3)"),
    ("'1'", "char"),
    ("'true'", "boolean"),
    ("'2026-01-02'", "date"),
    ("'2026-01-02'", "timestamp"),
    ("'2026-01-02'", "timestamptz"),
    ("'2026-01-02'", "timestamp(3)"),
    ("'03:04:05'", "time"),
    ("'1 day'", "interval"),
    ("'abc'", "bytea"),
    ("'00000000-0000-0000-0000-000000000000'", "uuid"),
    ("'{}'", "json"),
    ("'{}'", "jsonb"),
    ("'{1}'", "integer[]"),
    ("c_integer", "text"),
    ("c_integer", "bigint"),
    ("c_text", "varchar(5)"),
    ("c_num10_2", "integer"),
)
SCALARS: tuple[str, ...] = (
    "upper(c_varchar)",
    "lower(c_text)",
    "length(c_varchar)",
    "char_length(c_text)",
    "octet_length(c_text)",
    "substring(c_varchar, 1, 2)",
    "substring(c_text FROM 1 FOR 2)",
    "substr(c_text, 1, 2)",
    "trim(c_varchar)",
    "btrim(c_text)",
    "ltrim(c_text)",
    "rtrim(c_char)",
    "replace(c_varchar, 'a', 'b')",
    "left(c_text, 2)",
    "right(c_text, 2)",
    "lpad(c_text, 5, '0')",
    "position('a' IN c_text)",
    "strpos(c_text, 'a')",
    "split_part(c_text, ',', 1)",
    "initcap(c_text)",
    "md5(c_text)",
    "reverse(c_text)",
    "repeat(c_text, 2)",
    "concat(c_text, c_integer)",
    "concat_ws(',', c_text, c_integer)",
    "format('%s', c_integer)",
    "regexp_replace(c_text, 'a', 'b')",
    "to_char(c_date, 'YYYY')",
    "to_char(c_num10_2, '999')",
    "to_date('2026', 'YYYY')",
    "to_timestamp(0)",
    "to_number('1', '9')",
    "make_date(2026, 1, 2)",
    "round(c_num10_2)",
    "round(c_num10_2, 1)",
    "round(c_double)",
    "round(c_integer)",
    "ceil(c_double)",
    "floor(c_num10_2)",
    "abs(c_smallint)",
    "abs(c_num10_2)",
    "sign(c_integer)",
    "trunc(c_double)",
    "sqrt(c_integer)",
    "power(c_integer, 2)",
    "ln(c_double)",
    "exp(c_integer)",
    "mod(c_integer, 2)",
    "mod(c_bigint, c_integer)",
    "div(c_numeric, 2)",
    "log(c_double)",
    "date_trunc('month', c_date)",
    "date_trunc('month', c_timestamp)",
    "date_trunc('month', c_timestamptz)",
    "extract(year FROM c_date)",
    "extract(epoch FROM c_timestamp)",
    "date_part('year', c_date)",
    "age(c_timestamp)",
    "age(c_timestamp, c_timestamp)",
    "now()",
    "clock_timestamp()",
    "c_jsonb -> 'a'",
    "c_jsonb ->> 'a'",
    "c_json -> 'a'",
    "c_jsonb #>> '{a}'",
    "jsonb_build_object('a', c_integer)",
    "json_build_object('a', c_integer)",
    "to_jsonb(c_integer)",
    "jsonb_typeof(c_jsonb)",
    "jsonb_array_length(c_jsonb)",
    "c_int_array[1]",
    "c_text_array[1]",
    "array_length(c_int_array, 1)",
    "cardinality(c_int_array)",
    "ARRAY[c_integer, 2]",
    "array_to_string(c_int_array, ',')",
    "string_to_array(c_text, ',')",
    "array_append(c_int_array, 1)",
    "gen_random_uuid()",
    "current_database()",
    "version()",
    "row_number() OVER (ORDER BY c_integer)",
    "rank() OVER (ORDER BY c_integer)",
    "dense_rank() OVER (ORDER BY c_integer)",
    "ntile(4) OVER (ORDER BY c_integer)",
    "percent_rank() OVER (ORDER BY c_integer)",
    "lag(c_num10_2) OVER (ORDER BY c_integer)",
    "lead(c_varchar) OVER (ORDER BY c_integer)",
    "first_value(c_date) OVER (ORDER BY c_integer)",
    "sum(c_integer) OVER ()",
    "count(*) OVER ()",
    "bool_and(c_boolean)",
    "string_agg(c_text, ',')",
    "string_agg(c_varchar, ',')",
    "array_agg(c_integer)",
    "jsonb_agg(c_integer)",
    "count(*)",
    "count(DISTINCT c_text)",
)
NILADIC: tuple[str, ...] = (
    "current_date",
    "current_time",
    "current_timestamp",
    "localtime",
    "localtimestamp",
    "current_user",
    "session_user",
    "current_role",
    "current_schema",
    "current_catalog",
    "user",
)
SHAPES: tuple[tuple[str, str], ...] = (
    ("scalar subquery", "SELECT (SELECT max(c_varchar) FROM t) AS v FROM t"),
    ("derived table", "SELECT v FROM (SELECT c_num10_2 AS v FROM t) AS d"),
    ("cte", "WITH d AS (SELECT c_char AS v FROM t) SELECT v FROM d"),
    ("values", "SELECT v FROM (VALUES (1), (2.5)) AS d (v)"),
    ("union of literals", "SELECT 'a' AS v UNION SELECT 'bc'"),
)
