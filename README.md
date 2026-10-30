# SQLQuack

SQLQuack converts Snowflake SQL into DuckDB SQL, and refuses to guess.

Most SQL transpilers print *something* for every input. Often that output parses and runs on DuckDB but
means something different: `CONCAT` handles NULLs differently, regular expressions anchor
differently, decimal division keeps a different scale. SQLQuack takes the opposite approach. Every
conversion either comes from a rule verified against recorded Snowflake results, or it is refused
with a precise reason: the construct, where it is, and why.

**Status: early.** A first set of rules converts the relational core, exact NUMBER arithmetic,
comparisons, conditionals, casts, basic string functions, aggregates and window functions;
everything else is refused. What works today:

- parsing Snowflake SQL with SQLQuack's own parser, derived from
  [polyglot](https://github.com/sqlstack-io/polyglot), which keeps every call as written, and
  classifying every function, operator, type, cast, join and clause in it;
- a structured refusal for each construct: *missing rule*, *outside a rule's type conditions*,
  *unresolved name*, *not expressible in DuckDB*, *parse fidelity*, or *conversion budget
  exceeded*;
- DuckDB's own parse-and-bind check of every converted statement;
- `sqlquack census`, which reports which constructs a body of SQL uses and how much of it would
  convert as rules are added;
- `sqlquack index`, which maps every building block, typing function and rule to the cases and
  SQL files whose conversions rely on it.

## Census

```console
$ cargo run --release -p sqlquack -- census path/to/sql
$ cargo run --release -p sqlquack -- census path/to/sql --json
```

Each `.sql` file is one query, or one model compiled by
[SQLBuild](https://github.com/sqlstack-io/sqlbuild); SQLBuild's reference calls are recognised. The
report contains counts and construct names only, never SQL text or file names; `--names` adds
file, model and relation names to the slowest files and the blockers. Conversion needs the
column types of the relations a file reads: `--schema` takes a JSON file mapping relation names to
`[column, Snowflake type]` pairs, and `--chain-models` types each converted file as the relation
its file name references, then converts again. Where the schema file types a model that is itself
refused, files reading it still convert from those columns: the report counts them as
unit-testable (`convertible`) but not strict (`convertible_strict`, every model upstream converts
too), and lists each with an `upstream_refused:<model>` label for every refused model upstream.
`--assume-supported` takes construct keys or `rule.<id>`s and reports how many files would then be
unit-testable and strict, what blocks the rest next, and a greedy plan of the few constructs each
step should support together. Every such figure is an upper bound: it counts only the refusals
collected, and supporting a construct may uncover others.

## Index

```console
$ cargo run -p sqlquack -- index                       # the evidence cases
$ cargo run -p sqlquack -- index path/to/sql --schema schema.json --json
```

Every conversion records the rules it applied and, per rule, the DuckDB building blocks and
Snowflake typing functions it called. `sqlquack index` inverts those records: building block →
rules → cases and files, and rule → cases and files, with refused constructs listed the same way.
Every list is sorted, so the output of two revisions can be diffed to see which cases a change to
one building block affects.

## Evidence

Every rule is verified against results recorded on Snowflake and replayed on DuckDB:

```console
$ cargo run -p sqlquack -- pending          # write pending case files
$ uv run sqlquack-record                    # maintainers only: record them on Snowflake
$ cargo run -p sqlquack -- verify           # replay recorded cases on DuckDB and compare
```

The recorder uses the Snowflake connector's own configuration: the connection named by
`--connection` or `SQLQUACK_SNOWFLAKE_CONNECTION`, otherwise the connector's default connection
in `~/.snowflake/connections.toml`. `SNOWFLAKE_ACCOUNT`, `SNOWFLAKE_USER`, `SNOWFLAKE_ROLE`,
`SNOWFLAKE_AUTHENTICATOR` and `SNOWFLAKE_HOST` override it, and `--warehouse` or
`SQLQUACK_SNOWFLAKE_WAREHOUSE` selects the warehouse. Tokens are cached, so an OAuth
authorization-code login is interactive only when the cached token has expired.

The policy that governs comparison lives in [sqlquack.toml](sqlquack.toml).

For human review, `cargo run -p sqlquack -- review --out review` writes one sqllogictest-style
`.slt` file per case file: each case's description, verdict, rules and divergence warnings, the
Snowflake SQL, the DuckDB SQL or the refusal, and the recorded result with its Snowflake column types
and exact values. The export is for reading only; recorded Arrow results stay the evidence.

## How it works

See [docs/architecture.md](docs/architecture.md).

## Development

`make check-ci` runs every static gate (rustfmt, clippy, fensu, ruff, ty and the append-only
evidence guard) and `make test` runs the tests. CI runs both with `RECORDER=0`, which leaves the
recorder out. See [AGENTS.md](AGENTS.md) for the rules every change follows.

## License

Apache-2.0.
