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

## How it works

See [docs/architecture.md](docs/architecture.md).

## Development

`make check-ci` runs every static gate (rustfmt, clippy, fensu, ruff, ty and the append-only
evidence guard) and `make test` runs the tests. CI runs both with `RECORDER=0`, which leaves the
recorder out. See [AGENTS.md](AGENTS.md) for the rules every change follows.

## License

Apache-2.0.
