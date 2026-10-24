# Architecture

## Principle

SQLQuack never passes a construct through unconverted. Identical syntax in two dialects is not
evidence of identical meaning, so every node of the output must come from a rule that is verified
against recorded Snowflake results, under stated type conditions. Anything else is refused.

## Pipeline

```
source SQL ─ parse (polyglot, source dialect) ─▶ tree
  ├─ fidelity: print back in the source dialect, re-parse, compare    (else refuse: parse fidelity)
  ├─ classify: every node, function, cast type, join kind and clause  (one construct each)
  ├─ convert:  each construct must be claimed by a rule               (else refuse: missing rule,
  │                                                                     or not expressible)
  ├─ print:    polyglot's DuckDB printer
  └─ check:    re-parse the DuckDB SQL and compare with the intended tree (else refuse: printer)
```

Today the rule catalogue is empty, so the convert step refuses every construct. The source dialect is
a parameter of every entry point; Snowflake is the only one implemented.

## Crates

| Crate | Role |
|---|---|
| `sqlquack-core` | Parsing with complexity guards, construct classification, refusals, the tree comparator and both re-parse checks. |
| `sqlquack-snowflake` | The Snowflake typing layer (empty). Polyglot leaves many Snowflake expressions untyped, so exact types (`NUMBER(p, s)`, timestamp kinds, semi-structured values) are computed here. |
| `sqlquack-rules` | The rule catalogue and declared refusals (empty). |
| `sqlquack-duckdb` | DuckDB building blocks shared by rules (empty). |
| `sqlquack-policy` | The policy file `sqlquack.toml`: schema, validation and type-pattern matching. |
| `sqlquack-compare` | The strict comparator between a recorded result and a DuckDB replay or a fresh recording. |
| `sqlquack-evidence` | Cases, case hashes, pending files, stored results, DuckDB replay, verification and audits. |
| `sqlquack` | The `sqlquack` command line and the library entry points: the pipeline, the census and the evidence commands. |

DuckDB is pinned exactly (`duckdb = "=1.20000.0"`, DuckDB v2.0.0). Its build script downloads the
matching prebuilt `libduckdb`, so binaries are run with `cargo run`, which sets the library path.

The recorder, `sqlquack-record`, is a separate Python package under `recorder/`. Rust and Python
share files only.

Polyglot is pinned to one revision with `default-features = false`. Its `transpile` feature stays
off: none of its own conversions are used.

## Constructs

A construct is a key `<family>:<name>`:

- `node:<kind>`: a polyglot AST node kind, such as `node:select`, `node:eq` or `node:coalesce`;
- `fn:<NAME>`: a function polyglot keeps as a generic call, by upper-case name; calls to project
  functions are one construct, `fn:<udf>`;
- `cast:<TYPE>`: the base target type of a cast, as polyglot prints it in the source dialect;
- `join:<kind>`: a join kind;
- `clause:<name>`: a query clause or set-operation form, such as `clause:qualify` or
  `clause:union_all`;
- `syntax:<name>`: a spelling recorded separately, such as `syntax:double_colon_cast`.

Clause classification destructures polyglot's query structs field by field, so a polyglot upgrade
that adds a clause fails to compile until the clause is classified.

Each construct carries a source position. Polyglot keeps positions only on identifiers, columns,
tables, stars and some calls; any other node takes the first position inside it, or else its
parent's.

## Refusals

Every refusal has a category, a construct (absent when a whole statement is refused), a position and
a reason:

- **missing rule**: no verified rule converts the construct yet;
- **not expressible in DuckDB**: declared in the catalogue with a reason and evidence;
- **parse fidelity**: the SQL does not parse, or printing it back changes its tree.

## Tree comparison

Both re-parse checks compare trees as JSON after removing a declared list of spelling-only fields
(source positions, comments, `::` versus `CAST`, optional keywords and similar) and applying a
declared list of normalisations (function-name case, dollar-quoted strings, comment wrappers). Each
entry weakens the checks, so each has a reason and a test showing that two spellings parse to trees
that differ only there. A mismatch reports the first differing JSON path.

## Source fidelity

A statement is printed back in its own dialect and re-parsed. If the tree is unchanged it is
faithful. If it changes once and then settles, the change is a canonicalisation (for example `TOP`
becoming `LIMIT`); canonicalisations are refused until a recorded case proves both forms agree on
Snowflake. A tree that keeps changing is refused as unstable.

## Census

`sqlquack census` runs the pipeline over every `.sql` file and reports, per construct, how many files
use it, plus how many files would convert as the top N constructs get rules, in frequency order and
in greedy order (each next rule is the one that completes the most files). SQLBuild reference calls
(`__ref`, `__source`, `__seed`, `__udf`, `__table_fn`, cursor bounds) are replaced with neutral
placeholders before parsing. The report holds counts and construct keys only.

## Evidence

Recorded warehouse results are the only arbiter of correctness. Recording runs outside CI, through
the maintainer; results are append-only and named by the hash of their case, and everything that
could be weakened to make a failing case pass lives in one reviewed policy file.

### Cases

Case files live under `cases/<category>/<construct>.toml`. Each file names an edge-case category
from the policy, the construct it exercises and a session profile, then lists cases. A case has
typed input columns, input rows written as source literals, and either an `expression` over the
columns or a `query` over the `input` relation with a declared row order (`ordered` or `bag`).
Every case carries tags, and between them a file's cases must cover every input its category
requires.

A case expands to one source statement: the input rows as a `WITH input AS (...)` of casts, then
the expression (`SELECT <expression> AS value FROM input ORDER BY ordinal`) or the query.

### Hashes

The case hash is the SHA-256 of the canonical JSON of the evidence format version, the source
dialect, the session profile's Snowflake parameters and the expanded source SQL. Descriptions,
tags and file locations are not hashed. Any edit that can change the result changes the hash, so
it becomes a new case with a new result.

### Flow

1. `sqlquack pending` writes `evidence/pending/<hash>.json` for every case without a result.
2. The maintainer runs `sqlquack-record`, which runs the pending SQL in batches, one Snowflake
   session per set of session parameters, and writes `evidence/results/<hash>.arrow` (the
   connector's Arrow result, unconverted) and `<hash>.json` (query ID, warehouse, connector and
   DuckDB versions, recording time, column descriptions, row count and the Arrow file's SHA-256).
   A SQL error is recorded as the outcome instead; session and connection errors stop the run
   without writing anything. When any statement of a batch fails, Snowflake reports the whole
   request as error 100132 (SQLSTATE P0000), so the batch's cases are rerun one at a time and each
   real error is classified on its own. Existing results are never overwritten.
3. `sqlquack verify` converts each recorded case, replays it on DuckDB in a fresh in-memory
   database under the profile's time zone and the policy's NULL order, and compares. A refusal is
   always acceptable; a difference fails.
4. `sqlquack audit --sample 10%` writes pending files for a reproducible sample of recorded cases
   into `evidence/audit/pending`; once recorded into `evidence/audit/results`,
   `sqlquack audit --compare` checks that the fresh results equal the stored ones exactly.

### Comparison

Values are decoded into one canonical form: exact decimals with trailing zeros removed, so an
integer and a `DECIMAL(38,0)` with the same value are equal; floats bitwise, with every NaN one
value; dates in days; times and timestamps in nanoseconds. NULL equals only NULL. Column names
match case-insensitively by default, because Snowflake upper-cases unquoted names. A pair of
column types must be admitted by a declared type equality, such as `NUMBER(p,s)` = `DECIMAL(p,s)`.
Audits compare two recordings and allow no tolerance at all.

### Integrity

CI runs `make evidence-guard`: any modified or deleted file under `evidence/results` fails, and an
added result must be named by a case hash, have exactly the recorder's metadata keys, name
`sqlquack-record` as its recorder and carry an Arrow file matching its hash.
