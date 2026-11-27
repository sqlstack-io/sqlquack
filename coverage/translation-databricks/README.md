# Databricks translation results measure

How much documented Databricks SQL syntax SQLQuack translates to DuckDB with results matching
Databricks, measured over the syntax catalogue in `coverage/syntax-databricks`. The files, outcomes,
form states and figures are the ones `coverage/translation/README.md` describes for Snowflake; this
file lists only what differs.

## Inputs

- Recorded statements: every case under the pinned evidence's `convert/databricks/cases/`, with the
  rows Databricks SQL returned for it. `sqlquack verify --evidence crates/sqlquack-policy/policy-databricks.toml
  --translation-statements FILE` replays them on DuckDB. A statement's constructs come from the
  case's own query, not from the input fixture every case shares (`WITH input AS (SELECT * FROM
  VALUES …)`). A statement the Databricks parser cannot read links to no form.
- `form-links.toml`: how operators, clauses, casts and the query shell every case runs in link to
  forms. Function calls link by name.
- `not-translatable.toml`: no group yet.
- `obligations/functions.toml`: one obligation per generated type-combination, NULL-argument and
  error-parity case of a live catalogue form, written from the cases' tags by
  `scripts/build_translation_obligations_databricks.py`; `make translation-files` fails when the
  committed file differs from its build. Boundary cases are not cited, because the case generator
  records no boundary subkind; the report lists them as uncited.
- `retired-obligations.toml`, `reviews/results.toml`: empty.
- `results-baseline.json`: every statement's outcome and every form's state, as the checker last
  measured them.

## Commands

```sh
make translation-files
uv run python -m scripts.build_translation_obligations_databricks --cases <pinned>/convert/databricks/cases
syntax-coverage results target/translation-statements-databricks.json --root coverage/syntax-databricks \
  --evidence <pinned>/syntax/databricks --cases <pinned>/convert/databricks/cases --write-baseline
```
