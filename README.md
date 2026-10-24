# SQLQuack

SQLQuack converts Snowflake SQL into DuckDB SQL, and refuses to guess.

Most SQL transpilers print *something* for every input. Often that output parses and runs on DuckDB but
means something different: `CONCAT` handles NULLs differently, regular expressions anchor
differently, decimal division keeps a different scale. SQLQuack takes the opposite approach. Every
conversion either comes from a rule verified against recorded Snowflake results, or it is refused
with a precise reason: the construct, where it is, and why.

**Status: early.** No conversion rules exist yet, so SQLQuack currently refuses every construct. What
works today:

- parsing Snowflake SQL with [polyglot](https://github.com/chio-labs/polyglot) and classifying every
  function, operator, type, cast, join and clause in it;
- a structured refusal for each construct: *missing rule*, *not expressible in DuckDB*, or
  *parse fidelity*;
- two re-parse checks: printing the Snowflake tree back to Snowflake, and checking that DuckDB SQL
  parses back to the tree it was printed from;
- `sqlquack census`, which reports which constructs a body of SQL uses and how much of it would
  convert as rules are added.

## Census

```console
$ cargo run --release -p sqlquack -- census path/to/sql
$ cargo run --release -p sqlquack -- census path/to/sql --json
```

Each `.sql` file is one query, or one model compiled by
[SQLBuild](https://github.com/chio-labs/sqlbuild); SQLBuild's reference calls are recognised. The
report contains counts and construct names only, never SQL text or file names.

## How it works

See [docs/architecture.md](docs/architecture.md).

## Development

`make check-ci` runs every static gate (rustfmt, clippy, fensu, ruff and ty) and `make test` runs
the tests. See [AGENTS.md](AGENTS.md) for the rules every change follows.

## License

Apache-2.0.
