<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sqlstack-io/sqlquack/main/.github/sqlquack-logo-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/sqlstack-io/sqlquack/main/.github/sqlquack-logo-light.png">
    <img src="https://raw.githubusercontent.com/sqlstack-io/sqlquack/main/.github/sqlquack-logo-light.png" alt="SQLQuack" width="420">
  </picture>
  <br>
  <sub>by <a href="https://sqlstack.io">SQLStack</a></sub>
</p>

<p align="center">
  <strong>Run warehouse SQL locally. Verified results, or a clear no.</strong>
</p>

<p align="center">
  <a href="https://sqlquack.io">Website</a> ·
  <a href="https://sqlquack.io/docs/">Docs</a> ·
  <a href="https://sqlquack.io/playground/">Playground</a> ·
  <a href="https://sqlquack.io/docs/quickstart/">Quickstart</a>
</p>

SQLQuack checks warehouse SQL and translates it into DuckDB SQL, so you can run it locally and in
CI. Today it reads Snowflake SQL.

Most SQL transpilers print *something* for every input. Often that output runs on DuckDB but means
something different: `CONCAT` handles NULLs differently, regular expressions anchor differently,
decimal division keeps a different scale. SQLQuack converts only where a rule verified against
recorded Snowflake results covers the types involved. Everything else is refused with a code, the
exact span and the reason.

<p align="center">
  <img src="https://raw.githubusercontent.com/sqlstack-io/sqlquack/main/.github/demos/check-convert.gif" alt="sqlquack check reports a QUALIFY clause without a window function, with the span underlined and a fix; sqlquack convert turns a revenue query into DuckDB SQL, and refuses a query that needs HASH under the built-in policy" width="100%">
</p>

`sqlquack check` reports a `QUALIFY` that Snowflake would reject, with the span and a fix.
`sqlquack convert` turns a revenue query into DuckDB SQL, and refuses a query that needs `HASH`,
whose DuckDB value differs from Snowflake's.

## Quick start

> **Build from source for now.** SQLQuack has no published release yet: the `sqlquack` package on
> PyPI is a name reservation that does not contain the compiler. Until the first release, build it
> with a Rust toolchain:

```bash
git clone https://github.com/sqlstack-io/sqlquack
cd sqlquack
cargo build --release -p sqlquack
cargo run --release -p sqlquack -- check query.sql
cargo run --release -p sqlquack -- convert query.sql --schema schema.json
```

`check` needs no schema. `convert` needs the columns and Snowflake types of the relations a statement
reads, in a [schema file](https://sqlquack.io/docs/schema/). Or try SQL in the
[playground](https://sqlquack.io/playground/), which runs the same engine in your browser.

The binary links DuckDB's prebuilt shared library (`libduckdb.so`, `libduckdb.dylib` on macOS,
`duckdb.dll` on Windows), which the build downloads to
`target/duckdb-download/<target triple>/<DuckDB version>` and `cargo run` puts on the library path.
To run `target/release/sqlquack` directly, point the library path there:

```console
$ LD_LIBRARY_PATH=target/duckdb-download/x86_64-unknown-linux-gnu/v2.0.0 target/release/sqlquack convert query.sql
```

Use `DYLD_LIBRARY_PATH` on macOS and `PATH` on Windows.

## What works today

SQLQuack is early. The source dialect is Snowflake and the target is DuckDB. Parsing, checking and
translation are separate steps, each with its own coverage.

**Parse.** SQLQuack parses Snowflake SQL with its own parser, derived from
[polyglot](https://github.com/sqlstack-io/polyglot), which keeps every call as written. It
classifies every function, operator, type, cast, join and clause.

**Check.** `sqlquack check` reports SQL that Snowflake parses but then rejects, such as
`INTERSECT ALL`, `FOR UPDATE WAIT`, `QUALIFY` without a window function, or a `* ILIKE` `REPLACE`
or `RENAME` of a column the pattern removes. Each has its own `SQ02xx` code.

**Translate.** A first set of rules converts the relational core, exact NUMBER arithmetic,
comparisons, conditionals, casts, basic string functions, aggregates and window functions. DuckDB
parses and binds every converted statement before it is returned. Everything else is refused, or
stopped by a declared guard where DuckDB's result would differ. Each refusal names its kind:
*missing rule*, *outside a rule's type conditions*, *unresolved name*, *not expressible in DuckDB*,
*parse fidelity* or *conversion budget exceeded*. A `VALUES` list of more than 1,000 rows is refused
for now.

[Coverage](https://sqlquack.io/docs/coverage/) says how each part is measured.

Converted SQL is proven only where both sessions run in UTC: Snowflake's `TIMEZONE` and DuckDB's
`TimeZone` must be `'UTC'`. Statements whose result depends on the session time zone carry the
`session_timezone_utc` assumption; strict mode refuses them unless you confirm a UTC session with
`--utc-session`.

## Check

```console
$ cargo run --release -p sqlquack -- check query.sql
$ echo "SELECT 1 QUALIFY TRUE" | cargo run --release -p sqlquack -- check --json
$ cargo run --release -p sqlquack -- explain SQ0203
```

`check` parses Snowflake SQL and reports what Snowflake would reject, without a schema or a
conversion. It prints coded diagnostics on standard error, and `--json` prints
`{"ok": …, "diagnostics": [...]}`. It exits 0 when it finds nothing, 1 when it finds a problem and
2 for input without SQL. `explain` prints a diagnostic code's meaning, help and an example that
draws it; it takes the code, its name or a guard's former `E_GUARD_*` name, and exits 2 for an
unknown one.

## Convert

```console
$ cargo run --release -p sqlquack -- convert query.sql --schema schema.json
$ echo "SELECT 1 AS total" | cargo run --release -p sqlquack -- convert --json
```

`convert` prints the DuckDB SQL for Snowflake SQL from a file or standard input, and names the known
divergences and assumptions it relies on on standard error. A refused statement prints its refusals
instead. Each refusal is a coded diagnostic, underlined in the source as rustc does:

```console
error[SQ0004]: a name does not resolve
 --> query.sql:1:15
  |
1 | SELECT a FROM orders
  |               ^^^^^^ relation `ORDERS` has no known columns
  |
  = help: declare the relation and its columns in the schema (`--schema` on the command line)
```

With `--json`, each refusal carries the same diagnostic: `code`, `name`, `alias` (a parser guard's
former `E_GUARD_*` name), `message`, `span` (character `start` and `end`, one-based `line`,
`column`, `end_line` and `end_column`), `label`, `notes` and `help`. Codes are `SQ` and four digits,
listed with an explanation each in `crates/sqlquack-core/src/diagnostics/constants.rs` and in the
[diagnostics reference](https://sqlquack.io/docs/diagnostics/reference/). A code is never reused or
renumbered.

### Sessions

`--session NAME=VALUE` passes the caller's Snowflake session parameters (`function` takes it too):

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

`census` and `index` take no session parameters yet and convert under `utc-v1`.

### Policy

`convert`, `function`, `census` and `index` read `sqlquack.toml` (or `--policy`); without one they
use the built-in policy, which refuses every known divergence, as `--builtin-policy` asks for.
They share the policy flags: `--policy`, `--builtin-policy`, `--strict`, `--utc-session`,
`--confirm-declared-offsets`, `--python-runtime` and `--loud-divergences-only`, which relies only
on divergences whose converted SQL raises rather than returns another value. Every command with a
report takes `--format text|json`; `--json` is short for `--format json`.

HASH is refused by default: the converted SQL computes DuckDB's hash, which is consistent within one
run but never Snowflake's value. A project that relies on HASH only for joins, grouping,
deduplication or change detection can opt in. A policy file is complete on its own, so start from
this repository's [sqlquack.toml](sqlquack.toml): a file holding only `[divergences.hash]` is
rejected, because the comparator, session profiles and edge-case tables are required. Copy it into
the directory you run `sqlquack` from, and change `default` in its `[divergences.hash]` table from
`"refuse"` to `"warn"`:

```console
$ cp path/to/sqlquack/sqlquack.toml .
$ sed -i.bak '/^\[divergences.hash\]/,/^$/ s/^default = "refuse"/default = "warn"/' sqlquack.toml
$ grep -A3 '^\[divergences.hash\]' sqlquack.toml
[divergences.hash]
constructs = ["fn:HASH"]
default = "warn"
strict = "refuse"
```

A divergence the file leaves out is refused. See [Policy and sessions](https://sqlquack.io/docs/policy/).

### Functions and exit status

`function` converts a SQL project function's body, read from a file or from standard input with
`-`, and prints the macro as JSON. A refused body prints `{"converted": false, "name": …,
"refusals": [diagnostics]}` instead, with the same diagnostics on standard error.

`convert` and `function` exit 0 when they succeed, 1 when something is refused, and 2 when they
cannot run: bad arguments, an unreadable file, or input without SQL. `census` and `index` report
refused files rather than failing on them: they exit 0 once the report is printed, and 2 when they
cannot run.

## Census

```console
$ cargo run --release -p sqlquack -- census path/to/sql
$ cargo run --release -p sqlquack -- census path/to/sql --json
```

`census` reports which constructs a body of SQL uses and how much of it would convert as rules are
added. Each `.sql` file is one query, or one model compiled by
[SQLBuild](https://github.com/sqlstack-io/sqlbuild), whose reference calls are recognised. The
report holds counts and construct names only, never SQL text or file names; `--names` adds file,
model and relation names to the slowest files and the blockers.

- `--schema` takes a JSON file mapping relation names to `[column, Snowflake type]` pairs.
  `--chain-models` types each converted file as the relation its file name references, then
  converts again.
- Where the schema types a model that is itself refused, files reading it still convert from those
  columns. The report counts them as unit-testable (`convertible`) but not strict
  (`convertible_strict`, every model upstream converts too), and labels each
  `upstream_refused:<model>`, or `upstream_missing:<relation>` for a referenced model the scan holds
  no file for.
- Both counts are repeated without named assumptions (`convertible_no_assumptions`,
  `convertible_strict_no_assumptions`): a file relying on one, such as `session_timezone_utc` or
  `source_variant_json`, drops out, and a strict file also does when a model upstream relies on one.
- `--cursor-type timestamp|integer` replaces SQLBuild's cursor intrinsics with a bound as SQLBuild's
  Snowflake adapter renders it (timestamp bounds as text, integer bounds as integers); without it
  they stay unknown.
- `--assume-supported` (with `--chain-models`) takes construct keys or `rule.<id>`s and reports how
  many files would then be unit-testable and strict, what blocks the rest next, and a greedy plan of
  the few constructs each step should support together. Assumptions match at construct level: a
  `rule.<id>` expands to its construct keys, so it also clears other rules' refusals of the same
  construct. Plan steps support only missing rules and rules' type conditions.

Every `--assume-supported` figure is an upper bound: it counts only the refusals collected, matches
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

The `sqlquack` Python package lives in `python/`. Build it from source with `make python-wheel`; it
needs Python 3.12 or newer, runs the same code as the command line and ships the DuckDB library
beside itself. `check` and `convert` return frozen results with typed `Diagnostic`s, the text the
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

Set up the Python tooling once, without the Snowflake recorder:

```console
$ uv sync --locked --no-group recorder
```

`make check-ci RECORDER=0` runs the static checks (rustfmt, clippy, fensu, ruff, ty and the
append-only evidence guard), and `make test RECORDER=0` runs the Rust and Python tests.
`RECORDER=0` leaves the recorder out; only maintainers with a Snowflake account need it.

A change reaches `main` only after `make check-merge RECORDER=0` passes on its exact head commit:
the static checks and the Rust tests. That commit is then fast-forwarded onto `main`, so `main`
only ever holds a tree that passed.

`RECORDER=0 UV_NO_SYNC=1 make gate` runs more: it adds the replay of every recorded case, timed
conversion over pinned benchmark projects, which it downloads on first use, and a fuzz smoke run,
which needs a nightly toolchain with `cargo-fuzz`.

The README demo is recorded from a real run with the tapes in
[website/recordings](website/recordings/README.md).

## License

Apache-2.0.
