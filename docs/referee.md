# The Snowflake referee

SQLQuack's analysis (parsing, name resolution, checks and inferred types) is judged against
Snowflake itself, not against another parser. The referee is a set of statements over empty
synthetic tables, each with the verdict Snowflake gave when it compiled the statement.

## Cases

Referee environments live in `referee/environments/`. The referee cases and outcomes live in the
evidence repository under `referee/snowflake/`, apart from conversion cases, so `verify` does not
read them.

- `referee/environments/<name>.json`: the session parameters and the `CREATE TEMPORARY TABLE`
  statements of one environment of empty tables.
- `referee/snowflake/cases/generated/` (evidence repository): written by
  `make generate-referee-cases` from
  `scripts/referee_cases/` (checked by `make referee-cases`). It holds every function the
  generator knows at each argument count from 0 to one past its maximum and with swept argument
  types, clause combinations, name-resolution scenarios (lateral aliases, shadowing, quoting and
  case, ambiguity, `USING`, `QUALIFY` and `GROUP BY`), statements over structured and `VECTOR`
  types, and single-mistake mutants of the statements written to be valid.
- `referee/snowflake/cases/corpora/` (evidence repository): imported by `scripts.import_referee_corpora` from compiled DuckShop and
  DuckShop 10K models, against empty tables of their declared columns, and from sqlglot's
  Snowflake tests, where only the syntax verdict is kept.

A case is one statement in one environment. Its hash covers the dialect, the hash of the
environment, the statement and whether only the syntax verdict counts; a changed case is a new
case.

## Recording

`make referee-pending REFEREE_ARGS="--select 'generated/*.json' --catalogue"` writes
`evidence/pending/referee/`. The recorder then, in one session per environment:

1. creates the environment's tables as temporary tables, which vanish with the session;
2. runs `EXPLAIN` on each statement, refusing anything but a single query;
3. describes each accepted statement without running it, keeping each result column's type,
   precision, scale and nullability.

Outcomes go to the evidence repository's `referee/snowflake/results/<case hash>.json` and are
append-only, like recorded
results. An outcome keeps Snowflake's error code and message, with the session's account, user,
role, warehouse, database and schema names replaced by their kind; a syntax-only case keeps a
message only for a syntax error. With `--catalogue`, the recorder also stores `SHOW FUNCTIONS`
for built-in functions under `referee/snowflake/functions/`, named by the hash of its rows.

## Comparison

`python -m scripts.referee_report --binary <sqlquack> --library <libduckdb dir> --out <dir>`
parses every statement with SQLQuack and classifies it: accepted by both, SQLQuack too strict,
SQLQuack too lenient on syntax, syntax error caught, binding error caught (and whether at parse),
or missed. It writes a Markdown summary and every disagreement as a replayable JSON line to the
output directory only.
