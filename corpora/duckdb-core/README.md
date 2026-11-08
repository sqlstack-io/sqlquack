# DuckDB core corpus (WIP)

**Work in progress, not yet usable as a regression test.** This will be the acceptance bar for
SQLQuack's strict DuckDB parser: SQLBuild's DuckShop benchmark project ported to native DuckDB
2.0.0 SQL, plus common extras, with each statement tagged with the DuckDB syntax-catalogue forms it
uses (`coverage/syntax-duckdb/catalogue/*.toml`). It follows `corpora/databricks-core`.

## Current state

`duckshop.jsonl` holds 877 statements: 413 models, 452 unit tests, 10 functions and 2 singular
audits of SQLBuild <https://github.com/sqlstack-io/sqlbuild> at commit
`cf07c4016ea4b8595dcc6d2d9e5dcfbb21021403`, project `benchmarks/duckshop/project` (written in
Snowflake SQL). They were rendered offline with `sqb compile` from `sqlbuild` 0.192.0, with `ref`,
`source` and `seed` resolved to `rel__<name>` relations as in `corpora/databricks-core`, and
incremental cursor bounds fixed to `TIMESTAMP '2026-01-01 00:00:00'` and
`TIMESTAMP '2026-01-02 00:00:00'`. Each was then ported with sqlglot 30.30.0 (Snowflake to DuckDB) plus a rewrite pass:
`LATERAL FLATTEN` becomes `json_each`/`json_tree` over `to_json(...)` (`.index` read from
`key`, `OUTER => TRUE` as `LEFT JOIN LATERAL ... ON TRUE`), `ASOF JOIN ... MATCH_CONDITION (c)
ON k` becomes `ASOF JOIN ... ON (c) AND k`, `OBJECT_INSERT` becomes `json_merge_patch` with
`json_object`, untyped `ARRAY`, `OBJECT` and `VARIANT` casts become `JSON`, the `TO_TIMESTAMP_*`
and `IS_<type>` functions, `OBJECT_AGG`, `ARRAY_SIZE` and `RATIO_TO_REPORT` get DuckDB
equivalents, and the functions are hand-ported to DuckDB macros (`CREATE MACRO`, table macros
for the table functions).

Validation runs every statement on DuckDB 2.0.0: sources and seeds as empty tables of their
declared types, functions as macros, models as views (DuckDB binds a view when it is created),
then every unit test and audit executed. At this commit 824 of 877 bind and run. "Runs" means the
statement executed; a unit test's rows are not yet compared with its expected rows.

47 statements carry `excluded` with the reason: Snowflake behaviour DuckDB has no equivalent for,
each excluded with the unit tests of the model that uses it. They are HASH_AGG, the XML functions,
CONNECT BY (the only two statements DuckDB cannot parse), `GET_IGNORE_CASE`, per-row `TO_DATE`
formats, the object-editing functions, `FLATTEN`'s `PATH` column, `SYSTEM$TYPEOF` and `TYPEOF`'s
type names, Snowflake's week parameters, blank-filled number formats, per-value time-zone offsets,
positional column references with derived column names, and regular expressions RE2 refuses.
`rpt_edge_case_model_keys` reads seven excluded models, so the model is excluded but its unit test,
which supplies them as fixtures, is kept.

6 unit tests carry `port_unfinished` with DuckDB's error: three whose fixture lacks a column the
model reads (the rendered Snowflake fixture lacks it too), two whose expected timestamps fall
outside DuckDB's nanosecond range, and one that casts Snowflake's boolean words in a JSON string.

## Extras

`extras.jsonl` holds the 4,133 distinct statements of `corpora/polyglot-check/sqlglot.json` and
`corpora/polyglot-check/polyglot_tests.json` written for DuckDB or for both dialects (sqlglot
v30.14.0's fixtures, polyglot's tests at `7d5fe2c`), each with its `origin` (corpus file, entry
index and the test it came from) and DuckDB 2.0.0's verdict from `duckdb.extract_statements`:
`accepted`, or `refused` with the first line of DuckDB's message. 2,473 are accepted and 1,660
refused. Duplicate statements keep their first entry.

Still to do: the scenarios and hooks, and form ids.
