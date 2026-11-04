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

Converted SQL is proven only where both sessions run in UTC: Snowflake's `TIMEZONE` and DuckDB's
`TimeZone` must be `'UTC'`. Statements whose result depends on the session time zone carry the
`session_timezone_utc` assumption; strict mode refuses them unless you confirm a UTC session with
`--utc-session`.

## Check

```console
$ cargo run --release -p sqlquack -- check query.sql
$ echo "SELECT 1 QUALIFY TRUE" | cargo run --release -p sqlquack -- check --json
```

`check` parses Snowflake SQL and reports what Snowflake would reject, without a schema or a
conversion. It prints coded diagnostics on standard error, and `--json` prints
`{"ok": …, "diagnostics": [...]}`. It exits 0 when it finds nothing, 1 when it finds a problem and
2 for input without SQL.

## Convert

```console
$ cargo run --release -p sqlquack -- convert query.sql --schema schema.json
$ echo "SELECT 1 AS total" | cargo run --release -p sqlquack -- convert --json
```

The binary links DuckDB's prebuilt shared library (`libduckdb.so`, `libduckdb.dylib` on macOS,
`duckdb.dll` on Windows), which `cargo run` puts on the library path. To run
`target/release/sqlquack` directly, point the library path at the directory the build downloaded it
to, `target/duckdb-download/<target triple>/<DuckDB version>`:

```console
$ LD_LIBRARY_PATH=target/duckdb-download/x86_64-unknown-linux-gnu/v2.0.0 target/release/sqlquack convert query.sql
```

Use `DYLD_LIBRARY_PATH` on macOS and `PATH` on Windows. The Python module ships the library beside
itself and needs neither.

`convert` prints the DuckDB SQL for Snowflake SQL from a file or standard input, and names the known
divergences and assumptions it relies on on standard error. A refused statement prints its refusals
instead and exits 1; input holding no statement exits 2. Each refusal is a coded diagnostic,
underlined in the source as rustc does:

```console
error[SQ0004]: a name does not resolve
 --> query.sql:1:15
  |
1 | SELECT a FROM orders
  |               ^^^^^^ relation `ORDERS` has no known columns
  |
  = help: declare the relation and its columns in the schema (`--schema` on the command line)
```

With `--json`, each refusal also carries the same diagnostic: `code`, `name`, `alias` (a parser
guard's former `E_GUARD_*` name), `message`, `span` (character `start` and `end`, one-based `line`,
`column`, `end_line` and `end_column`), `label`, `notes` and `help`. Codes are `SQ` and four digits,
listed with an explanation each in `crates/sqlquack-core/src/diagnostics/constants.rs`; a code is
never reused or renumbered. SQL that Snowflake parses but then rejects, such as `INTERSECT ALL`,
`FOR UPDATE WAIT`, `QUALIFY` without a window function, or a `* ILIKE` `REPLACE` or `RENAME` of
a column the pattern removes, is refused with its own `SQ02xx` code instead of being converted.
`--session NAME=VALUE` passes the caller's Snowflake session parameters
(`function` takes it too):

- Conversion runs under the recorded session profile they match (`utc-v1`, or `utc-qiic-v1` for
  `QUOTED_IDENTIFIERS_IGNORE_CASE = true`) and refuses, naming the parameters, when they match none.
  A parameter left unset takes Snowflake's documented default, as `utc-v1` pins it.
- A parameter no profile covers is refused by name, so a misspelt name never falls back to the
  defaults. Only parameters that cannot change a result are accepted and ignored:
  `QUERY_TAG`, `USE_CACHED_RESULT`, `STATEMENT_TIMEOUT_IN_SECONDS`,
  `STATEMENT_QUEUED_TIMEOUT_IN_SECONDS`, `ABORT_DETACHED_QUERY` and `CLIENT_SESSION_KEEP_ALIVE`
  (`RESULT_NEUTRAL_SESSION_PARAMETERS` gives the reason for each). An empty name, or a name given
  twice with different values, exits 2.
- `TIMEZONE = UTC`, or no `TIMEZONE`, leaves time-zone-dependent statements to the
  `session_timezone_utc` assumption. Any other `TIMEZONE` refuses them, even with `--utc-session`.

`--strict`, `--utc-session` and `--json` work as for the census. `census` and `index` take no
session parameters yet and convert under `utc-v1`.

`convert`, `function`, `census` and `index` read `sqlquack.toml` (or `--policy`); without one they
exit 2 unless `--builtin-policy` asks for the built-in default, which refuses every known divergence.
They share the policy flags (`--policy`, `--builtin-policy`, `--strict`, `--utc-session`,
`--confirm-declared-offsets`, `--python-runtime` and `--loud-divergences-only`, which relies only
on divergences whose converted SQL raises rather than returns another value). Every command with a report takes
`--format text|json`; `--json` is short for `--format json`.

HASH is refused by default: the converted SQL computes DuckDB's hash, which is consistent within one
run but never Snowflake's value. A project that relies on HASH only for joins, grouping,
deduplication or change detection can opt in by setting `default = "warn"` under
`[divergences.hash]` in its `sqlquack.toml`.

`function` converts a SQL project function's body, read from a file or from standard input with
`-`, and prints the macro as JSON. A refused body prints `{"converted": false, "name": …,
"refusals": [diagnostics]}` instead, with the same diagnostics on standard error.

`convert`, `function`, `census` and `index` exit 0 when they succeed, 1 when something is refused,
and 2 when they cannot run: bad arguments, an unreadable file, or input without SQL.

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
too), and lists each with an `upstream_refused:<model>` label for every refused model upstream,
or `upstream_missing:<relation>` for a referenced model the scan holds no file for. Both counts are
repeated without named assumptions (`convertible_no_assumptions`,
`convertible_strict_no_assumptions`): a file relying on one, such as `session_timezone_utc` or
`source_variant_json`, drops out, and a strict file also does when a model upstream relies on one.
`--cursor-type timestamp|integer` replaces SQLBuild's cursor intrinsics with a bound as SQLBuild's
Snowflake adapter renders it (timestamp bounds as text, integer bounds as integers); without it
they stay unknown.
`--assume-supported` (with `--chain-models`) takes construct keys or `rule.<id>`s and reports how
many files would then be unit-testable and strict, what blocks the rest next, and a greedy plan of
the few constructs each step should support together. Assumptions match at construct level: a
`rule.<id>` expands to its construct keys, so it also clears other rules' refusals of the same
construct, such as every refused `node:cast`. Plan steps support only missing rules and rules' type
conditions. Every such figure is an upper bound: it counts only the refusals collected, matches
constructs rather than the exact condition refused, and supporting a construct may uncover others.
Each figure says how many of its unit-testable files may hold refusals collecting did not reach, and
a key matching no collected refusal is warned about.

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

## Python

```python
import sqlquack

sqlquack.check(sql="SELECT 1 QUALIFY TRUE").diagnostics[0].code  # "SQ0203"
result = sqlquack.convert(
    sql="SELECT 1 AS total", options=sqlquack.ConvertOptions(builtin_policy=True)
)
```

The `sqlquack` package (in `python/`, built with `make python-wheel`) runs the same code as the
command line. `check` and `convert` return frozen results with typed `Diagnostic`s, the text the
command line renders, and, for `convert`, its exact `--json` report. `ConvertOptions` has one field
per `convert` flag. Where the command line exits 2, they raise `SqlQuackError`.

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
recorder out.

`make check-merge RECORDER=0` is what a merge to main must pass on its exact head: the static gate
and the Rust tests.

`make gate` runs everything CI runs: the static gate, the tests, the replay of every recorded case,
the exact parser check and timed conversion over DuckShop and DuckShop 10K, and a fuzz smoke run.
It needs a nightly toolchain with `cargo-fuzz` for the fuzz step, and fetches and compiles the
pinned DuckShop projects itself on first use (`make duckshop-compiled`, pinned by `SQLBUILD_REF`
and `SQLBUILD_VERSION`). Run it with `RECORDER=0 UV_NO_SYNC=1` in a clean checkout of the commit:

```console
$ uv sync --locked --no-group recorder
$ RECORDER=0 UV_NO_SYNC=1 make gate VERIFY_FLAGS="--require-verified --strict-errors"
```

A change lands on `main` as one squashed commit whose tree is exactly a tree that passed
`make gate`.

## License

Apache-2.0.
