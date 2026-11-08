# Databricks core corpus

The acceptance bar for SQLQuack's strict Databricks parser: SQL that real Databricks users write, each
statement tagged with the Databricks syntax-catalogue forms it uses
(`coverage/syntax-databricks/catalogue/*.toml`). It is meant to become a regression test: every
statement must parse, and together they must cover the forms listed.

| File | Statements | What |
| --- | --- | --- |
| `duckshop.jsonl` | 839 | SQLBuild's DuckShop benchmark project, rendered and ported to Databricks SQL |
| `extras.jsonl` | 146 | short, hand-written everyday Databricks statements |

Each line is one JSON object:

- `id`: stable. `duckshop.<path of the source file without .sql>`, with `#n` for the n-th `TEST`
  block of a file that holds several; `extras.<slug>`.
- `origin`: for DuckShop, `{repository, commit, path}` of the source file; for extras, a note that it
  is hand-written.
- `sql`: one statement, no trailing semicolon.
- `forms`: the sorted catalogue form ids the statement uses. Every id exists in the catalogue.
- `spark_refuses` (only when present): why Apache Spark's parser rejects a statement that the
  Databricks documentation shows, with the documentation page.

## Sources

- SQLBuild, <https://github.com/sqlstack-io/sqlbuild> at commit
  `cf07c4016ea4b8595dcc6d2d9e5dcfbb21021403`, project `benchmarks/duckshop/project`. The project is
  written in Snowflake SQL (its adapter is `snowflake`) with SQLBuild templating.
- Rendering used `sqb compile` from `sqlbuild` 0.192.0, offline (no warehouse): macros, constants
  and enums are expanded inline by the compiler.
- The extras are written for this corpus.

### What one DuckShop statement is

- Model (389): the compiled model query. `ref`, `source` and `seed` resolve to plain relation names
  with the same naming as `referee/corpora/duckshop.json`: `rel__stg_campaigns`,
  `rel__storefront__carts`, `rel__currencies`. Incremental cursor bounds are fixed to
  `TO_TIMESTAMP_NTZ('2026-01-01 00:00:00')` and `TO_TIMESTAMP_NTZ('2026-01-02 00:00:00')`.
- Unit test (424): the compiled test query, which inlines the fixtures, every model under test as a
  CTE, the expected rows and the comparison query. An empty fixture becomes
  `SELECT * FROM rel__<source> WHERE FALSE`.
- Scenario (12): scenarios are not compiled offline, so the fixture, expected and assertion CTEs are
  taken from the file with model reads resolved to relation names, and the statement ends with one
  row count per expected and assertion CTE.
- SQL and Python functions (10), singular audits (2), and two hooks rendered for the model that uses
  them: `COMMENT ON TABLE` for `fct_orders`, and the JavaScript `edge_running_peaks` function, rewritten
  as a SQL UDF with the same running maximum (`transform` over `array_max(slice(...))`).

## Porting rules

The port is mechanical where possible: sqlglot reads the rendered Snowflake SQL, a rewrite pass
replaces the constructs below with Databricks equivalents of the same meaning, and sqlglot's
Databricks generator prints the result. One model is ported by hand (below).

- Casts: Snowflake `::` and `CAST` print as `CAST(... AS ...)`. `VARCHAR` becomes `STRING`, `NUMBER(p, s)`
  `DECIMAL(p, s)`, `TIMESTAMP_TZ`/`TIMESTAMP_LTZ` `TIMESTAMP`, `TIMESTAMP_NTZ(n)` `TIMESTAMP_NTZ`,
  `OBJECT` and `VARIANT` `VARIANT`, untyped `ARRAY` `ARRAY<VARIANT>`.
- Semi-structured data: `col:path` stays (Databricks variant paths), with awkward keys written
  `['key']`. `OBJECT_CONSTRUCT`/`OBJECT_CONSTRUCT_KEEP_NULL` become
  `to_variant_object(named_struct(...))`; `ARRAY_CONSTRUCT` `array(...)`; `OBJECT_AGG`
  `to_variant_object(map_from_entries(collect_list(struct(k, v)) FILTER (WHERE ...)))`;
  `OBJECT_KEYS` `json_object_keys(to_json(...))`; `TYPEOF`/`IS_<type>` comparisons predicates over
  `schema_of_variant`; `STRIP_NULL_VALUE`/`IS_NULL_VALUE` `is_variant_null`; dynamic `GET`/`[...]`
  `try_element_at` (1-based for arrays). `LATERAL FLATTEN(input => x[, outer => TRUE]) AS f` becomes
  `LATERAL variant_explode[_outer](x) AS f`, with `f.index` read as `f.pos`.
- Arrays: `ARRAY_SIZE` `size`, `ARRAY_TO_STRING` `array_join`, `ARRAY_AGG ... WITHIN GROUP (ORDER BY)`
  `array_sort(collect_list|collect_set(...))` (or a sort over `named_struct` pairs when the order key
  is another column), `ARRAY_GENERATE_RANGE(a, b)` `sequence(a, b - 1)`, `ARRAYS_OVERLAP` for `&&`.
- Dates and times: Snowflake `DATEDIFF('day', a, b)` becomes `datediff(b, a)`; other units count
  boundaries, so `DATEDIFF(MONTH, date_trunc('MONTH', a), date_trunc('MONTH', b))`. Format strings
  (`TO_CHAR`, `TO_DATE`, `TRY_TO_TIMESTAMP_TZ` with a format) are converted to Spark patterns
  (`YYYY-MM-DD` to `yyyy-MM-dd`) and use `date_format`, `to_date`, `try_to_timestamp`.
  `DAYOFWEEKISO`/`YEAROFWEEKISO` become `EXTRACT(DAYOFWEEK_ISO|YEAROFWEEK FROM ...)`,
  `TIMESTAMP_FROM_PARTS` `make_timestamp_ntz`, `TO_TIMESTAMP(ms, 3)` `timestamp_millis`.
  `CONVERT_TIMEZONE`, `DATEADD`, `DATE_TRUNC` and `LAST_DAY` keep their names (Databricks has them).
- Numbers and text: `TRY_TO_NUMBER(x[, p, s])` becomes `TRY_CAST(x AS DECIMAL(p, s))`, `DIV0` an `IF`,
  `IFF` `IF`, `RATIO_TO_REPORT(x) OVER (...)` `try_divide(x, sum(x) OVER (...))`,
  `JAROWINKLER_SIMILARITY` (0 to 100) `CAST(round(jaro_winkler_similarity(...) * 100) AS INT)`,
  `TRUNC(x, n)` a sign-aware `floor`/`ceil`, `STRTOK_TO_ARRAY` `filter(split(...), x -> x <> '')`,
  `HEX_DECODE_STRING`/`TRY_BASE64_DECODE_*` `try_to_binary(..., 'HEX'|'BASE64')`.
- Regular expressions: Snowflake's `REGEXP_LIKE`/`RLIKE` match the whole string, so the pattern is
  anchored `^(?:...)$` unless it already is. `REGEXP_SUBSTR` with occurrence, group or `e` flag becomes
  `try_element_at(regexp_extract_all(...), n)`; flags become inline `(?i)`/`(?s)`; replacement
  back-references `\1` become `$1`.
- Positional columns: Snowflake's `$1`, `$2` (a column by its position in the relation) become the
  column names at those positions in the CTE the query reads.
- URLs and lookups: `PARSE_URL(u, 1)['host'|'path'|'parameters'][k]` becomes `parse_url(u, 'HOST'|'PATH'|'QUERY', k)`;
  `PARSE_JSON('{constant}')[k]` a `map(...)[k]`.
- Joins and predicates: `ASOF JOIN r MATCH_CONDITION (l.a >= r.b) ON keys` becomes
  `LEFT JOIN LATERAL (SELECT * FROM r WHERE keys AND l.a >= r.b ORDER BY r.b DESC LIMIT 1)`;
  `x <> ALL (subquery)` becomes `NOT x IN (subquery)`. `TABLE(GENERATOR(ROWCOUNT => n))` with `SEQ4()`
  becomes `range(n)` with `id`.
- Hand port: `int_category_paths` (and the unit tests that inline it) replaces `CONNECT BY` with a
  `WITH RECURSIVE` CTE producing the same depth, path and root columns.
- Functions: SQL UDFs become `CREATE OR REPLACE FUNCTION f(p T) RETURNS T RETURN <expr|query>`;
  Python UDFs keep their handler and add `return handler(args)` in a `LANGUAGE PYTHON AS $$ ... $$`
  body. Calls to the project's own functions (`udf__sku_family`, `luhn_valid`, table functions) stay
  as plain calls, unqualified.
- Comments in the compiled SQL are kept as `/* ... */`.

## Form ids

A script maps each statement mechanically, and checks every id against the catalogue:

- A call `name(` maps to `db.functions.<name with _ replaced by ->` if that id exists (or the plain-call
  id of a dotted family such as `db.functions.substring.substring`), with arity picking `datediff3`,
  `dateadd2` or `date-add3`, and `COUNT(*)`, `TRIM(... FROM ...)`, `POSITION(... IN ...)` picking their
  special forms. Type names in a type position, keywords such as `IN (`, `OVER (`, `FILTER (WHERE`,
  and `LIKE ANY (` are not calls.
- Operators and predicates (`||`, `::`, `:` paths, `[...]`, `<>`, `IS [NOT] NULL`, `BETWEEN`, `LIKE`,
  `CASE`, `AND`, `OR`, `NOT`, arithmetic) map to their `db.functions.*sign` and predicate ids.
  Comparisons and arithmetic count only when Spark's parsed plan shows them, so `=` in an
  assignment or property list, `<`/`>` in a type, and `SELECT *` do not.
- Clauses and statements map from keyword sequences, with a parenthesis-context stack so a window's
  `ORDER BY` is not a query `ORDER BY` and a pipe `AGGREGATE ... GROUP BY` is not a `GROUP BY` clause;
  table references and table-valued functions come from Spark's parsed plan
  (`UnresolvedRelation`, `UnresolvedTableValuedFunction`).
- Literal and type forms come from literal tokens (`'...'`, `1`, `1.5`, `0D`, `TRUE`, `DATE '...'`)
  and from type names after `::`, `CAST(... AS`, `RETURNS`, or in column definitions.
- Struct field access with `.` cannot be told from a qualified column name by tokens, so
  `db.functions.dotsign.dot` is listed by hand on the eight extras that use it.

## Validation

Every statement is parsed with Apache Spark 4.2.0's `parsePlan`, with ANSI mode on; no warehouse is
used. Statements that parse carry no `spark_refuses`; statements that Spark refuses carry it, and the
check fails on any mismatch.

```python
# python -I validate.py corpora/databricks-core/duckshop.jsonl corpora/databricks-core/extras.jsonl
import json, sys
from pyspark.sql import SparkSession  # pyspark 4.2.0

spark = (SparkSession.builder.master("local[1]").config("spark.ui.enabled", "false")
         .config("spark.sql.ansi.enabled", "true").getOrCreate())
parser = spark._jsparkSession.sessionState().sqlParser()
mismatches = 0
for path in sys.argv[1:]:
    for line in open(path):
        row = json.loads(line)
        try:
            parser.parsePlan(row["sql"])
            parsed = True
        except Exception:
            parsed = False
        mismatches += parsed == ("spark_refuses" in row)
sys.exit(1 if mismatches else 0)
```

Result: `duckshop.jsonl` 833 parsed, 6 refused (the Python UDFs: Spark has no `LANGUAGE PYTHON`
body); `extras.jsonl` 137 parsed, 9 refused (`CLUSTER BY AUTO`, `SHALLOW CLONE`, `COPY INTO`,
`USE CATALOG`, `OPTIMIZE`, two `VACUUM`, `RESTORE`, `GRANT`). Each refused statement is shown in the
Databricks SQL reference page named in its `spark_refuses`.

## Excluded DuckShop files

These files test Snowflake-only behaviour that has no faithful Databricks equivalent. Each model's
unit-test file (`tests/unit/intermediate/edge_cases/test_<model>.sql`) is excluded with it: 24 models
and 24 test files, 52 statements. All are in the project's edge-case area, which exists to probe
Snowflake semantics.

| Model (`models/intermediate/edge_cases/`) | Reason |
| --- | --- |
| `int_edge_amount_bit_reads` | `HASH_AGG` returns Snowflake's own order-independent hash; no Databricks function has the same values |
| `int_edge_basket_xml_reads` | `PARSE_XML`, `XMLGET`, `CHECK_XML` over Snowflake's XML-in-VARIANT; Databricks has no XML variant to read the same way |
| `int_edge_variant_xml_elements` | `XMLGET` over Snowflake's XML-in-VARIANT |
| `int_edge_check_xml_checks` | `CHECK_XML` returns Snowflake's XML parser error text |
| `int_edge_json_numbers` | `CHECK_JSON` returns Snowflake's JSON parser error text |
| `int_edge_category_walk` | tests `CONNECT BY` semantics (`LEVEL`, `PRIOR`, `CONNECT_BY_ROOT` over a cross join); Databricks has no `CONNECT BY` |
| `int_edge_check_formatted_dates` | `TO_DATE` with a Snowflake format string computed per row; cannot be converted statically |
| `int_edge_check_key_list_edits` | `OBJECT_INSERT` with Snowflake's update flag; no VARIANT object-edit function |
| `int_edge_payload_objects` | `OBJECT_INSERT`, `OBJECT_PICK`, `OBJECT_DELETE` |
| `int_edge_ratio_object_inserts` | `OBJECT_INSERT` |
| `int_edge_payload_attributes` | `GET_IGNORE_CASE` (case-insensitive key lookup) |
| `int_edge_ratio_ignore_case_reads` | `GET_IGNORE_CASE` |
| `int_edge_settlement_calendar` | `WEEK`/`YEAROFWEEK` follow Snowflake's `WEEK_START` and `WEEK_OF_YEAR_POLICY` parameters; Databricks has ISO weeks only |
| `int_edge_clock_arithmetic` | `SYSTEM$TYPEOF` returns Snowflake type names, which are the expected values |
| `int_edge_key_widths` | `SYSTEM$TYPEOF` |
| `int_edge_null_arithmetic` | `SYSTEM$TYPEOF` |
| `int_edge_check_note_unions` | `UNION ALL BY NAME` over differing columns; the Databricks set-operator reference documents no `BY NAME` |
| `int_edge_join_keys` | `UNION ALL BY NAME` |
| `int_edge_orders_all_channels` | `UNION ALL BY NAME` |
| `int_edge_row_deciders` | `UNION ALL BY NAME` |
| `int_edge_variant_matches` | `UNION ALL BY NAME` |
| `int_edge_payload_documents` | Snowflake path edge cases (an empty key, a `:*` key) with no Databricks variant-path spelling |
| `int_edge_payload_lines` | reads `FLATTEN`'s `PATH` column; `variant_explode` returns only `pos`, `key`, `value` |
| `int_edge_payload_paths` | `FLATTEN(..., RECURSIVE => TRUE)`; `variant_explode` has no recursive mode |

Not statements, so not in the corpus: macro, constant, enum and schema definition files (macros,
constants and enums are expanded inline where used), and generic audit templates (`not_null`,
`unique`, `has_rows` and the like; `sqb compile` instantiates them per column as one-line
`SELECT ... WHERE ... IS NULL` checks).
