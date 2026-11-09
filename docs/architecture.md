# Architecture

The tokenizer, parser and syntax tree, and how they move from polyglot into SQLQuack, are
designed in [syntax.md](syntax.md).

## Principle

SQLQuack never passes a construct through unconverted. Identical syntax in two dialects is not
evidence of identical meaning, so every node of the output must come from a rule that is verified
against recorded Snowflake results, under stated type conditions. Anything else is refused.

## Pipeline

```
source SQL ─ parse (sqlquack-syntax, source dialect, calls as written) ─▶ tree   (else refuse: parse fidelity)
  ├─ classify: every node, function, cast type, join kind and clause  (one construct each)
  ├─ convert:  each construct must be claimed by a rule               (else refuse: missing rule,
  │                                                                     or not expressible)
  ├─ print:    SQLQuack's DuckDB building blocks
  └─ check:    DuckDB parses and binds the printed SQL
```

Today the rule catalogue is empty, so the convert step refuses every construct. The source dialect is
a parameter of every entry point; Snowflake is the only one implemented.

## Crates

| Crate | Role |
|---|---|
| `sqlquack-syntax` | The shared syntax tree, lossless tokens, spans, complexity guards and parse diagnostics that every dialect parser builds on. It names no dialect. Derived from polyglot; see [syntax.md](syntax.md). |
| `sqlquack-syntax-snowflake` | The Snowflake parser, with strict and tolerant modes, building the shared tree. It also reads DuckDB, PostgreSQL and SQLite as polyglot does. |
| `sqlquack-derive` | The derive that generates the tree's child traversal. |
| `sqlquack-syntax-bigquery`, `-clickhouse`, `-databricks`, `-duckdb`, `-postgres`, `-sqlite`, `-sqlserver` | Native parsers, one per dialect, each following its engine's own grammar and building the shared `sqlquack-syntax` tree. Each README lists what it covers and its gaps. |
| `sqlquack-parse` | The one way the other crates reach a parser. The Snowflake parser's entry points are always there; each native parser is a module behind the feature of its dialect's name. |
| `sqlquack-diagnostics` | Diagnostics, the failure taxonomy, refusals, constructs and conversion provenance. Snowflake's error taxonomy and rejections sit in `src/snowflake/` behind the `snowflake` feature. |
| `sqlquack-semantics` | SQL semantics apart from conversion: dialect profiles (Snowflake and DuckDB), relation and column name resolution in query scopes, column types with the marks VARIANT values carry, and lineage (each output's value sources, plus the query's `row_sources`: the columns filters, join keys, groups, CONNECT BY, PIVOT FOR, ORDER BY under a limit and table-function inputs decide rows by, with `rows_resolved` false when one of those could not be traced). Snowflake's typing layer is in `src/snowflake/`: polyglot leaves many Snowflake expressions untyped, so exact types (`NUMBER(p, s)`, timestamp kinds, semi-structured values) are computed there. It builds without the conversion, DuckDB and evidence crates. |
| `sqlquack-convert` | Conversion to DuckDB: the rule catalogue, the rules with their type conditions and declared refusals (`src/snowflake/`), and the pipeline itself, `analyze`: parse, classify, convert each statement or refuse it. It builds without DuckDB's engine, so the bind check in the last step is skipped where it is linked alone; every crate that enables the engine gets the check, and `make engine-free` fails when this crate reaches DuckDB. |
| `sqlquack-blocks` | DuckDB building blocks shared by rules, and, behind the `engine` feature, DuckDB's own parse-and-bind check of every converted statement. |
| `sqlquack-policy` | The policy file `sqlquack.toml`: schema, validation and type-pattern matching. |
| `sqlquack-format` | The SQL formatter: lays SQL out over lines, changing only the whitespace between tokens and keyword case. A layout guide prints the authored tokens as a parse-tree pretty printer would; the output is refused unless it keeps every token, keeps comments attached and is idempotent. It reaches parsers through `sqlquack-parse`: for SQL Server the Transact-SQL scanner supplies the tokens and its parser validates the input; for BigQuery the GoogleSQL scanner does the same; and for PostgreSQL the native scanner decides keywords and its parser validates the input. |
| `sqlquack-lint` | The lint engine and its rules over SQLQuack's tokens and syntax trees, ported from SQLClean, which uses it. |
| `sqlquack-fingerprint` | Change fingerprints of SQL that ignore whitespace, comments and keyword case, over SQLQuack's tokenizers; SQLClean uses them. |
| `sqlquack-compare` | The strict comparator between a recorded result and a DuckDB replay or a fresh recording. |
| `sqlquack-evidence` | Cases, case hashes, pending files, stored results, DuckDB replay, verification and audits. |
| `sqlquack` | The `sqlquack` command line and the library entry points: the pipeline, `run` over fixtures, the census, the index and the evidence commands. |
| `sqlquack-python` | The Python module `sqlquack._native`, built into the PyPI package `sqlquack`. |

Development tools live under `tools/`. No shipped crate depends on them:

| Crate | Role |
|---|---|
| `sqlquack-polyglot-check` | Proves `sqlquack-syntax` parses exactly as polyglot does, on every corpus. |
| `sqlquack-coverage-check` | The syntax catalogues of every dialect root under `coverage/` checked against the parsers (binary `syntax-coverage`), the translation-results measure and the nesting-depth probe. See [syntax-coverage.md](syntax-coverage.md). |
| `sqlquack-format-check` | Proves that formatting DuckDB SQL leaves the output columns DuckDB binds it to unchanged, on the format corpus. |

Fensu layer targets keep the rules out of every crate but the command line ones and
`sqlquack-convert`, and keep the conversion, DuckDB and evidence crates out of syntax, diagnostics
and semantics.

Dialect code lives only in each crate's `src/<dialect>/` folder, behind a feature of the same
name, and no dialect is a default feature (fensu rule XSQ007). Shared code names no dialect: it asks
the dialect's profile through the crate's `src/dialects` registry, which keeps the run-time dialect
values of the public APIs and returns an error naming the dialect when its feature is not built in.
Semantics, format, lint, fingerprint, diagnostics and convert take one feature per dialect they
support, and `sqlquack-blocks` takes `engine` to link DuckDB; a consumer enables the ones it uses.

DuckDB is pinned exactly (`duckdb = "=1.20000.0"`, DuckDB v2.0.0). Its build script downloads the
matching prebuilt `libduckdb`, so binaries are run with `cargo run`, which sets the library path.

The recorder, `sqlquack-record`, is a separate Python package under `recorder/`. Rust and Python
share files only.

Polyglot is pinned to one revision with `default-features = false`. Its `transpile` feature stays
off: none of its own conversions are used.

## Constructs

A construct is a key `<family>:<name>`:

- `node:<kind>`: a syntax tree node kind, such as `node:select`, `node:eq` or `node:extract`;
- `fn:<NAME>`: a call, by its upper-case name as written; the tree keeps every Snowflake call as
  written (see `docs/syntax.md`, step 5), so `fn:IFNULL` and `fn:COALESCE` are distinct; calls to
  project functions are one construct, `fn:<udf>`;
- `cast:<TYPE>`: the base target type of a cast, as polyglot prints it in the source dialect;
- `join:<kind>`: a join kind;
- `clause:<name>`: a query clause or set-operation form, such as `clause:qualify` or
  `clause:union_all`;
- `syntax:<name>`: a spelling recorded separately, such as `syntax:double_colon_cast`.

Clause classification destructures polyglot's query structs field by field, so a polyglot upgrade
that adds a clause fails to compile until the clause is classified.

Each construct carries a source position. Source SQL is parsed located (`CallMode::FaithfulLocated`,
see `docs/syntax.md`), so operators, casts, calls, predicates, aliases, subqueries, queries, joins,
CTEs and statements carry their own source range besides the names polyglot positions; any other
node takes the first position inside it, or else its parent's.

## Refusals

Every refusal has a category, a construct (absent when a whole statement is refused), a position and
a reason. The position is the source range of the construct refused: conversion places a refusal
at the innermost expression, FROM source, join or query being converted when it arose, so a call
inside a CTE or subquery is placed at the call. Positions come from the converted tree's own
nodes; a node a rule builds from the tree, as a call converted through its typed spelling, places
its copies of tree nodes as their originals. A refusal of the whole statement, such as one beyond
the conversion budget, is placed at the statement, and SQL that does not parse at the failure or
else at the whole input.

- **missing rule**: no verified rule converts the construct yet;
- **not expressible in DuckDB**: declared in the catalogue with a reason and evidence;
- **parse fidelity**: the SQL does not parse;
- **divergence refused**: converting needs a known divergence that the policy refuses in this mode,
  such as under `--strict` or `--builtin-policy`; the reason names the divergence;
- **conversion budget exceeded**: converting the statement took longer, or emitted more DuckDB SQL
  relative to its source, than the `[conversion_budget]` in `sqlquack.toml` allows. The reason gives
  the measured time or size. A conversion that grows pathologically is refused, never kept.

## Spelling pairs

The tree keeps every call as written, so each spelling reaches a rule only through its own entry in
`CALL_SPELLINGS`, backed by recorded cases that use it. Where two spellings mean the same thing,
recorded pairs prove it: a case file under `cases/fidelity/` declares `equivalence = "<id>"`, one of
the pairs in `SPELLING_PAIRS` (`sqlquack-convert`), and gives every case a second, `canonical`
spelling. Both are recorded, and `sqlquack verify` requires the two results to be identical. A rule
that converts both spellings of a pair cites the pair in its provenance.

| Id | Spelling | Second spelling |
|---|---|---|
| `variant_path_to_get_path` | `v:a.b[0]` | `GET_PATH(v, 'a.b[0]')` |
| `text_type_to_varchar` | `TEXT`, `STRING`, `STRING(n)` | `VARCHAR`, `VARCHAR(n)` |
| `subscript_to_get` | `v[i]`, `v[k]` with an index or key computed per row | `GET(v, i)`, `GET(v, k)` |
| `position_to_charindex` | `POSITION(s, t [, n])`, `POSITION(s IN t)` | `CHARINDEX(s, t [, n])` |
| `rlike_to_regexp_like` | `t RLIKE p` | `REGEXP_LIKE(t, p)` |
| `mod_function_to_operator` | `MOD(a, b)` | `a % b` |
| `from_values_to_subquery` | `FROM VALUES (...) AS v(c)` | `FROM (VALUES (...)) AS v(c)` |
| `len_to_length` | `LEN(s)` | `LENGTH(s)` |
| `to_number_to_cast` | `TO_NUMBER(s [, p [, s]])`, `TO_DECIMAL`, `TO_NUMERIC` and their `TRY_` forms | `CAST(s AS NUMBER(p, s))`, `TRY_CAST(...)` |
| `to_varchar_to_cast` | `TO_VARCHAR(x)`, `TO_CHAR(x)` | `CAST(x AS VARCHAR)` |
| `to_double_to_cast` | `TO_DOUBLE(s)`, `TRY_TO_DOUBLE(s)` | `CAST(s AS FLOAT)`, `TRY_CAST(s AS FLOAT)` |
| `timediff_to_datediff` | `TIMEDIFF(unit, a, b)` | `DATEDIFF(unit, a, b)` |
| `timestampdiff_to_datediff` | `TIMESTAMPDIFF(unit, a, b)` | `DATEDIFF(unit, a, b)` |
| `typed_literal_to_cast` | `DATE 's'`, `TIMESTAMP 's'`, `TIME 's'` | `CAST('s' AS DATE)`, `CAST('s' AS TIMESTAMP)`, `CAST('s' AS TIME)` |
| `object_agg_value_to_variant` | `OBJECT_AGG(k, x)` of a NUMBER `x` | `OBJECT_AGG(k, TO_VARIANT(x))` |
| `part_functions_to_date_part` | `YEAR(x)`, `QUARTER(x)`, `MONTH(x)`, `DAY(x)`, `HOUR(x)`, `MINUTE(x)`, `SECOND(x)` | `DATE_PART(unit, x)` |

Spellings other engines accept that are not in Snowflake's function reference
(`NOT_SNOWFLAKE_FUNCTIONS`, such as `CHAR_LENGTH`, `IF` or `STARTS_WITH`) are refused with the
reason "not in Snowflake's function reference". The refusal claims only that absence; no
recording shows Snowflake rejecting them.

The parser stores `v:"a.b"` and `v:a.b` as the same path although they name different keys, so a
rule for `:` paths must read the path from the source tokens.

## Conversion

Conversion works on the parsed tree, statement by statement, in three layers:

- **Typing** (`sqlquack-semantics`, `src/snowflake/`): exact Snowflake result types, each function derived from
  recorded results, such as `NUMBER(p,s)` for `+ - * /`, the unification of CASE, COALESCE, IFF,
  NVL and set operation branches, and cast targets.
- **Building blocks** (`sqlquack-blocks`): DuckDB fragments with Snowflake semantics, such as exact
  decimal division rounding half away from zero, NULL-propagating concatenation, explicit NULL
  ordering, or text conversions that accept only spellings Snowflake is recorded to read.
- **Rules** (`sqlquack-convert`, `src/snowflake/`): one rule per construct family, each stating the type conditions it
  needs and refusing outside them. Names resolve through `sqlquack-semantics` against a schema
  of typed relations and the statement's CTEs, as Snowflake reads them and refusing names DuckDB
  cannot tell apart; anything unresolved is refused.

DuckDB parses and binds the emitted SQL as a final check.

### Schema interface

SQLBuild hands SQLQuack one neutral schema: typed relations and the signatures of project
functions, keyed by the names statements use once resolved (unquoted names upper-cased):

```json
{
  "relations": {"ORDERS": [["ORDER_ID", "NUMBER(38,0)"], ["NOTE", "VARCHAR"]]},
  "functions": {
    "IS_GIFT": {"language": "python", "arguments": [["NOTE", "VARCHAR"]], "returns": "BOOLEAN"},
    "OPEN_ORDERS": {
      "language": "sql",
      "arguments": [["MINIMUM", "NUMBER(10,2)"]],
      "returns": {"table": [["ORDER_ID", "NUMBER(38,0)"]]}
    }
  }
}
```

A file holding only the relations map, the earlier form, still loads. A relation or function using
a type SQLQuack does not know is left out, so statements using it are refused.

A column may also be written as an object, `{"name": "PAYLOAD", "type": "VARIANT",
"typed_leaves": true}`. `typed_leaves` marks a VARIANT column that may hold DATE, TIME, timestamp
or BINARY values written by a process outside the project. Statements comparing such a column by
its text are refused rather than relying on `source_variant_json` (see Known divergences).

A TIMESTAMP_TZ column of a relation may declare the offset all its values hold, `"offset":
"+00:00"` (or `"UTC"`; no other offset is supported, and declaring one, or declaring an offset for
another type or a function's column, is a usage error). The column then carries the same UTC mark
as `CONVERT_TIMEZONE('UTC', x)`, through CTEs, set operations, COALESCE, CASE and windows, and an
operation depending on the offset (its wall time, date, parts, arithmetic, or a VARIANT holding it)
relies on the named assumption `declared_offset:<RELATION.COLUMN>`. The policy's
`declared_offset` divergence is an assumption: allowed with a warning by default, refused in strict
mode unless the caller confirms the declarations (`--confirm-declared-offsets`). Operations that
do not depend on the offset, such as comparing instants or CONVERT_TIMEZONE of the value, rely on
nothing. Undeclared TIMESTAMP_TZ columns keep refusing offset-dependent operations.

- A scalar call is typed by its signature (`rule.project_function_call`): as many arguments as
  parameters, each of the parameter's type, NULL, or a NUMBER widened to a NUMBER or FLOAT
  parameter; the emitted call `"NAME"(...)` is cast to the declared return type. Python functions
  are opaque calls and take and return only VARCHAR, NUMBER, FLOAT and BOOLEAN until recordings
  prove how other values cross into and out of Snowflake.
- `TABLE(f(...))` of a table function is a relation of the declared columns, each cast to its
  type (`rule.project_table_function`); its arguments may not read columns.
- A SQL function's body converts through the same rules as a model (`analyze_function`): a scalar
  body becomes `CAST((body) AS type)`, a table body a query, with the parameters a relation in
  the outer scope. The result is a DuckDB macro body over parameters `p1`, `p2`, ...: the
  parameters, cast to their types, joined LATERAL to the converted body. SQLBuild creates the
  macro under the name calls use.
- DuckDB's check of a statement defines a stand-in macro of each called function's shape.

The census rewrites `__udf("name")` and `__table_fn("name")` to `__sqlquack_udf__name` and
`__sqlquack_table_fn__name`, so a schema can declare them; calls are classified as `fn:<udf>`.
Until SQLBuild writes the `functions` section itself, the census-only script
`scripts/add_census_functions.py` reads the `CREATE FUNCTION` headers SQLBuild compiles under
`target/compiled/functions` and adds them under those names; it is not part of the crates.

### Known divergences

The policy lists known divergences, each with the constructs it covers and a treatment in the
default and in strict mode. A rule that may diverge asks the policy first: allowed, the conversion
records the divergence and its construct, and reports carry a warning naming both; otherwise the
construct is refused. Each divergence declares its `failure`: `loud` where the converted SQL raises
wherever DuckDB could differ and never returns another value, proven by a case expecting it or a
test of its guard raising; `value` where it can return another value, as for HASH, RANDOM, the
clock or any assumption. `convert --json` reports each relied-on divergence's failure under
`divergence_failures`, and `--loud-divergences-only` refuses every `value` divergence, in any mode
and whatever the caller confirms. `numeric_range` covers SUM and AVG totals beyond 38 digits, `+`, `-` and `*` whose
capped result type may hold a 39-digit value (Snowflake keeps up to 128 bits under NUMBER(38)), and
divisions whose exact intermediate exceeds 128 bits, where DuckDB raises an error and Snowflake
returns a value. For `*`, the generated `*_range_edge` cases (products between 10^38 and 2^127)
recorded Snowflake returning rows where DuckDB raises. A
loud divergence never covers DuckDB returning a different value. `timestamp_range` covers TIMESTAMP_NTZ(7-9) values
outside DuckDB's nanosecond range (1677-2262), and `timestamp_tz_submicro` covers TIMESTAMP_TZ and
TIMESTAMP_LTZ(7-9) values with a non-zero sub-microsecond part, which DuckDB's microsecond
TIMESTAMPTZ cannot hold; both raise at run time. Naive timestamps pair by precision: 0-6 with
TIMESTAMP, 7-9 with TIMESTAMP_NS. Lowering a timestamp's precision truncates toward the past, as
Snowflake does, rather than using DuckDB's rounding casts. CONVERT_TIMEZONE from a zone other than UTC
reads a repeated wall time as its earlier instant and a skipped one with the offset before the
transition, as Snowflake does; DuckDB alone takes the later instant, so `duckdb.zone_to_zone`
tries the offset in effect a day earlier first. `json_fidelity` covers semi-structured values, held as DuckDB `JSON`
(decision 3). DuckDB keeps non-integer numbers as doubles, reads a different set of lenient
spellings, accepts repeated keys and keeps keys in input order; Snowflake keeps exact decimals,
reads single quotes, NaN and undefined, rejects repeated keys and writes keys sorted. So
`duckdb.json_guarded` raises for any text DuckDB might read differently: text DuckDB does not
read as JSON, numbers with an exponent, an all-zero fraction, six leading fraction zeros, more
than 15 decimal digits or 39 integer digits, letters and trailing commas outside strings,
surrogate escapes and repeated keys at any depth. PARSE_JSON and TRY_PARSE_JSON raise alike, so
TRY_PARSE_JSON never returns NULL on one side and a value on the other, with one exception that
no reader can read: for TRY_PARSE_JSON, `duckdb.json_guarded` gives NULL for markup and for a
document cut off before its first value closes, when only double-quoted strings could hide a
bracket (recorded). VARIANT columns of base
tables pass the same guard. VARIANT columns of source tables are assumed to hold JSON values, as loaded from JSON:
`source_variant_json` names that assumption. Snowflake keeps a DATE, TIME, timestamp or BINARY
typed inside a VARIANT, unequal to its text (recorded), where DuckDB's JSON holds only the text.
So VARIANT `=` and `<>`, PARTITION BY a VARIANT, ARRAY_CONTAINS, ARRAY_POSITION, ARRAY_EXCEPT,
ANY_VALUE, MAX_BY, MIN_BY and ARRAY_AGG without WITHIN GROUP, which tell VARIANTs apart by their
text, convert values read from a source or schema VARIANT only by relying on it, and every such
statement carries it in its provenance, its review warnings and the census tally. Typed values
written into a source VARIANT by a process outside the project are not detected; mark the column
`typed_leaves` in the schema file to have those statements refused, and strict mode refuses every
statement relying on the assumption. Values a statement builds itself are tracked instead: a
comparison by text of a value not proven free of typed leaves is refused where a DATE or timestamp
enters a VARIANT in the statement, or where it reads a model's VARIANT column that may hold one. Documents the guard passes convert exactly. Snowflake writes a
VARIANT as text with every object's keys in UTF-8 byte order, at every depth, and array elements
in order (batch 12 recorded mixed case, non-ASCII and supplementary-plane keys, prefixes, empty
containers, escaped keys and thirteen levels of nesting). The writer behind `::VARCHAR`, TO_VARCHAR,
TO_JSON, HASH and VARIANT result columns keeps DuckDB's text for sorted documents and resorts
others through DuckDB's `VARIANT`, which keeps every object's keys in byte order; it raises when
that would rewrite a number, such as an integer beyond 64 bits. Its SQL is one expression whose
size does not depend on nesting, so composed VARIANT expressions grow linearly. Strings take the recorded
escapes: `\"`, `\\`, `\b`, `\t`, `\n`, `\f` and `\r`, and `\u00xx` in lower case for every other
control character and DEL; `/` and U+2028 are written as they are. VARIANT
result columns convert (user decision of 2026-10-25, superseding that part of decision 3):
`duckdb.json_output` writes them as DuckDB `JSON` text written this way, and verify compares them
with Snowflake's JSON text without whitespace outside strings. OBJECT_CONSTRUCT, OBJECT_INSERT and
OBJECT_DELETE rebuild objects with keys in byte order, as Snowflake keeps them; keys are matched
exactly and never read as paths. OBJECT_CONSTRUCT and OBJECT_CONSTRUCT_KEEP_NULL of `*` or
`alias.*`, with an optional EXCLUDE list, take the columns as keys named as resolved. A NUMBER
with a scale in a VARIANT is written as Snowflake writes it (trailing fraction zeros dropped, a
whole value without a point); DuckDB's JSON then holds a whole value as an integer where Snowflake
keeps a DECIMAL, so a statement where one enters is refused where it reads TYPEOF of a VARIANT
not proven free of it, and so is a statement reading a model's VARIANT column that may hold one
(`decimal_leaves`); comparisons hold, as Snowflake compares a whole decimal equal to the integer. Other values Snowflake keeps typed inside a VARIANT
(timestamps with a time zone, times, binary) are refused until recordings prove how they read
back. VARIANT keys in ORDER BY, GROUP BY and PARTITION BY are refused.

`variant_typed_values` covers DATE, TIMESTAMP_NTZ and TIMESTAMP_LTZ values put in a VARIANT (by
TO_VARIANT, `::VARIANT` or an object constructor): DuckDB stores the JSON string Snowflake writes
for them (batch 8 recorded `YYYY-MM-DD`, `YYYY-MM-DD HH:MI:SS.FFF` and an LTZ suffix ` Z` in UTC),
so text output, paths and casts back match, and raises where that text drops sub-millisecond
digits or a non-UTC offset. TYPEOF and IS_DATE-style checks of a string in such a text raise
instead of answering VARCHAR or false. TIMESTAMP_TZ (DuckDB keeps no offset), TIME and BINARY (its
upper-case hex text is indistinguishable from ordinary strings, `''` included) stay refused.
IS_OBJECT, IS_ARRAY and IS_BOOLEAN test the JSON kind; OBJECT_KEYS, OBJECT_PICK, ARRAY_CONSTRUCT
(a SQL NULL element, undefined in Snowflake, raises) and ARRAY_EXCEPT of string arrays keep keys
in byte order; `::OBJECT` and `::ARRAY` of a VARIANT raise for another kind. LATERAL FLATTEN becomes a lateral subquery over
`json_each` (`duckdb.json_flatten`), after a FROM source or after JOIN ... ON clauses; its PATH
raises for object keys that are not plain names until their spelling is recorded. LATERAL
SPLIT_TO_TABLE on a non-empty literal delimiter becomes a lateral subquery over `string_split`
(`duckdb.split_to_rows`). An aliased LATERAL subquery directly in the FROM list becomes a DuckDB
LATERAL subquery reading the sources before it. `percentile_interpolation` covers PERCENTILE_CONT between two different values:
Snowflake's interpolated results carry rounding error in the last digits that no exact or double
formula reproduces, so the converted SQL returns a value only where the fraction lands on a row
(or between equal values) and raises otherwise. `date_text_spelling` covers TRY_TO_DATE and
TRY_CAST to DATE of text in a spelling no recording shows.

`hash` is a `value` divergence, as are RANDOM, UUID_STRING, the sequences and the clock, which
differ by nature, and every assumption. Unlike them it is refused by default, since a wrong hash
looks like a right one; a project that needs only internal consistency opts in with
`default = "warn"` under `[divergences.hash]` in its policy. HASH
converts to DuckDB's 64-bit hash shifted into the signed range, NUMBER(19,0), which is
deterministic and equal for equal arguments in the same order within one run but never
Snowflake's value. Integer NUMBERs hash by value whatever their precision, and a VARIANT by its
JSON text with keys in byte order (raising under `json_fidelity` where DuckDB keeps another order);
NUMBERs with a scale, FLOAT and timestamps are refused until recordings show which of their values
Snowflake hashes alike. Its evidence records only these properties (determinism, equality,
argument order, NULL handling and the result type), never the values. The warning names what
breaks: literal hash values in expected output, hashes compared with Snowflake-computed ones,
`HASH(x) % n` bucketing and ordering by a hash. RANDOM, UUID_STRING and sequences are to follow
the same pattern.

`sqlquack census --strict` and
`sqlquack index --strict` refuse what strict mode refuses.

### Provenance

`ProvenanceLog::rule` returns a scope; while it is held, every building block and typing function
called is attributed to that rule. A conversion's provenance holds the equivalences, rules,
building blocks and typing functions it used, plus each rule's own uses, all sorted. A refusal
records the refused construct. `sqlquack verify --json` reports both per case, the census reports
how many converted files use each identifier, and `sqlquack index` inverts the records into
identifier → rules → cases and files.

## Census

`sqlquack census` runs the pipeline over every `.sql` file and reports, per construct, how many files
use it, plus how many files would convert as the top N constructs get rules, in frequency order and
in greedy order (each next rule is the one that completes the most files). SQLBuild reference calls
(`__ref`, `__source`, `__seed`, `__udf`, `__table_fn`, cursor bounds) are replaced with neutral
placeholders before parsing; each model, source or seed reference becomes a relation named after its
arguments, so a project's source file can type it. The report holds counts and construct keys only. With
`SQLQUACK_GUARDS_DIR` set, census also writes each converted file's known divergences there, and
with `SQLQUACK_CONVERTED_DIR` set, each converted file's DuckDB SQL, named after its path.
Progress goes to stderr every few seconds, and the conversion times of the slowest files, over every
pass, are listed there at the end. `--names` adds file, model and relation names: the slowest
files' paths, the blockers' model names and the relation in each `model:` and `relation:` root.

The `blockers` section traces each refused file through the models it references. Conversion stops
at a statement's first refusal, so for blockers the census converts each refused file again,
collecting every refusal: a refused expression becomes a NULL placeholder typed as the node alone
decides (a cast's target, a predicate's BOOLEAN, a function with a fixed result type) and
conversion carries on; a refused FROM source or join becomes a relation whose columns are unknown.
A refusal over an untyped placeholder or unknown column may be a knock-on, so it is dropped and the
file marked possibly incomplete, as is a file whose statement stops at a query-level refusal; a
missing rule depends on no type and is always kept. Refused models are collected upstream first, and
the columns their conversion reached type their readers. This pass never changes conversion output:
it runs only for refused files, on its own, and its SQL is discarded. A file's own
refusals are its roots, except an unresolved relation that is a refused model of the project, which
contributes that model's roots instead; a relation that is neither a model nor in the schema is
the root `relation:<name>`. Relation and model names match in upper case, as Snowflake folds
unquoted identifiers, and a file's other refusals stay roots next to its relations'. Models refused
only because they read each other share the root `cycle`. Each root reports the files it refuses
directly, the files downstream of those, their total and the files it alone blocks; a file blocked
by several roots counts towards each. `sole` counts the files with complete refusals whose only root it is, which fixing it alone
would make convert as far as the census can tell, and `sole_uncertain` the possibly incomplete ones;
roots rank by `sole`. A file is possibly incomplete when it or a model upstream is.
`single_root_files` counts the complete refused files one fix would unblock. `combinations` ranks
pairs and triples of roots by the complete files whose roots they cover, reading no refused model
(`direct`) or reading one (`downstream`). The `refusals` totals still count each file's first
refusal. Without `--names`, files
are numbered in name order, their unknown relations are left out, and `model:` and `relation:`
roots fold into `upstream model` and `unknown relation`.

Each refused file also lists its refusals in source order under `placed`, each with its category,
construct and the line and column it is at in the file as written: positions after a replaced
reference call are mapped back through the replacement. The text report prints them for the first
refused files, at most five per file and then a count of the rest. `sqlquack review` writes each
refusal as `<category> at <line>:<column>: <reason>`.

### Guards in conditions

A guard raises where DuckDB cannot reproduce Snowflake's value. In a WHERE, HAVING, QUALIFY or join
condition, conjuncts holding a raise are written as `free AND CASE WHEN free THEN guarded ELSE FALSE
END`, so a guard is evaluated only for rows the other conjuncts keep, whatever order DuckDB's
optimizer chooses; the free conjuncts stay plain so filters still push down. A guard inside an OR
or a single guarded condition is evaluated for every row reaching it, which may raise for a row
whose value Snowflake would not need: a loud failure, never a different value.

### Comparing converted models with recorded results

`sqlquack compare --manifest <file>` replays converted SQL on DuckDB and judges each result against a
recording with the strict comparator. The manifest is a JSON list of entries
`{id, recorded, database, sql, ordered}`: a recording's sidecar, beside its `.arrow` file; a DuckDB
database attached read-only as `inputs`, holding the recorded inputs the SQL reads; the converted
SQL file; and whether rows compare in order or as a bag. The sidecar must record `rows`, and the
Arrow file must match its `arrow_sha256`, column count and `row_count`. Each replay runs in the
policy's session profile (`--session-profile`, `utc-v1` by default) with its null ordering. Each
entry prints `{id, outcome, detail}`. The outcome is one of:

- `matches`;
- `matches_at_microseconds`: the recording kept declared nanosecond columns in microseconds without
  the recorder's sub-microsecond check, and the replay matches once its own sub-microsecond digits
  are dropped. This never counts as a full match;
- `differs`, with the first difference;
- `duckdb_error`;
- `unreadable`: the recording or the SQL file cannot be read.

### Guard measurement

`scripts/measure_guards.py` estimates how often known-divergence guards would raise on real rows.
Its sources file lists probes: a guarded Snowflake expression rewritten over one relation's columns
`x1`..`xn`, with its divergence, construct, whether the result is semi-structured, and whether it
aggregates. `--samples` holds `<relation>.parquet` per relation, VARIANT values as JSON text, and
optionally `<relation>.types.json` with Snowflake types for columns the sources file marks UNKNOWN.

Each probe becomes the body of a table function `SELECT (<expression>) IS NULL AS r FROM
guard_sample` (a semi-structured result is wrapped in `TO_JSON`, as output writes it) and is
converted by `sqlquack function` with the policy's divergences, so the measured SQL is exactly what
SQLQuack emits. It runs over the sample's distinct value tuples in one pass: every raise site
(`CAST(CAST('<message>' AS INTEGER) AS "NULL")`) is replaced by a call to a Python hook that
records the tuple and message and returns NULL, so each raising tuple and the first raise it reaches
are known exactly, with common-subexpression elimination off so guards stay inside their CASE. When
DuckDB raises an error of its own, which no hook sees, the count falls back to one pass of
`TRY(<expression>)`, still exact, and reasons come from at most `--examples` raising tuples run
alone, their shares scaled to the exact count and marked estimated. A body of an unexpected shape is
bisected within `--query-limit` queries, a lower bound when the budget runs out. A progress line per
relation gives its worst raising share and top reason. A guard's raise counts against its
divergence, a deliberate Snowflake error against `snowflake_error`, and DuckDB's own error against
the probed divergence. The JSON report and text summary give, per probe, divergence and source
column, the rows checked, rows raising, share and top reasons, with at most three redacted shapes
(digits as `9`, letters as `a`), never a sampled value. Filters in the model are not applied, so
shares are over the relation's rows, not the model's.

## Evidence

Recorded warehouse results are the only arbiter of correctness. Recording runs outside CI, through
the maintainer; results are append-only and named by the hash of their case, and everything that
could be weakened to make a failing case pass lives in one reviewed policy file.

### Running the suites

Each suite has one pattern for every dialect, and `make help` lists the targets. The check- targets
replay committed evidence offline; the record- targets need the dialect's engine or account and
refuse when `RECORDER=0`. Every target needs `DIALECT`; no dialect is the default.

| Target | What it does |
|---|---|
| `make pending DIALECT=<d>` | Lists what is not recorded yet: the syntax suite's unmet obligations (`syntax-coverage missing`) and, for Snowflake, writes the translation suite's pending cases. |
| `make check-syntax DIALECT=<d>` | Reparses every recorded syntax case of `coverage/syntax-<d>` (`coverage/syntax` for Snowflake) and compares with its receipts: `syntax-coverage diff` against the baseline where there is one, `summary` otherwise. `make check-ci` runs it for Snowflake, SQLite and DuckDB. |
| `make record-syntax DIALECT=<d>` | Runs the dialect's syntax referee, `scripts/run_<d>_receipts.py run`, with its arguments in `RECORD_ARGS`. Snowflake's capture tool and the Postgres referee are not in the repository yet, so it refuses those. |
| `make check-convert DIALECT=snowflake [CASES=<folder>]` | Replays recorded translations on DuckDB (`make verify-recorded`, or only `CASES`). |
| `make record-convert DIALECT=snowflake` | Writes the pending cases and records them on Snowflake (Flow, below). |

Every conversion targets DuckDB, so only the source dialect is named; Snowflake is the only one today.

### Cases

Case files live under `cases/<category>/<construct>.toml`. Each file names an edge-case category
from the policy, the construct it exercises and a session profile, then lists cases. A case has
typed input columns, input rows written as source literals, and either an `expression` over the
columns or a `query` over the `input` relation with a declared row order (`ordered` or `bag`).
Every case carries tags, and between them a file's cases must cover every input its category
requires.

Cases imported from a third-party test suite live under `cases/imported/` and name their source in
`origin`. Only the source-dialect SQL becomes the case; the answer is still recorded on Snowflake.
A reference translation, such as sqlglot's expected DuckDB SQL in `sqlglot_duckdb`, is kept as side
data: it is never asserted and is not hashed. See [THIRD_PARTY_NOTICES](../THIRD_PARTY_NOTICES).

### Generated cases

Combinatorial grids are written by deterministic generators in `scripts/case_generators/`
(`make generate-cases`) into `cases/generated/<generator>/`. A generated case is an ordinary case:
it is hashed, recorded and verified like any other, and its `origin` names the generator and its
version, such as `generated: casts@1` (not hashed). The same version always writes the same files,
and `make generated-cases`, part of `make check-ci`, fails when a generated file differs from its
generator's output, so generated files are never edited by hand. Changing a generator's output
bumps its version; cases whose SQL changes get new hashes and need recording again.

The grids cover NUMBER and FLOAT operand pairs for arithmetic and comparison, casts between every
pair of scalar types, CONVERT_TIMEZONE across zone pairs and DST transitions, NULL in every
argument position of n-ary functions, and paths and JSON text over a fixed set of documents. Rows
that may raise, such as results beyond the documented result type or invalid text, go into their
own cases, so one raising row never hides the others. Cases that need judgement, such as JSON
null handling, ordering and unusual formats, stay hand-written.

### Typing-only cases

Some constructs cannot convert yet, but analysis still needs their exact result types. A typing
case is evidence for those types only. It is a normal case with `evidence = "typing"`, set on the
file or on the case, so it has the same Snowflake SQL over typed input rows. Its recording keeps
Snowflake's result metadata: each column's type with its precision, scale or length, and its
nullability. With `values = false` the recorder stores only that metadata and the row count, as
outcome `types`, with no Arrow file. `values = false` is hashed, and is omitted from the hash when
absent, so existing hashes still hold. Without it, the values are kept, but nothing compares them.

A typing case is never converted or replayed on DuckDB. Verify reports it as `typed`, the index
counts no rule for it, and an audit compares re-recorded column metadata and row counts. A typing
case cannot hold an equivalence or an expected divergence. Only a typing case may set
`values = false`; a conversion case recorded without values fails.

Each construct's evidence status is therefore `converted` (a passing conversion case),
`typed` (only typing cases) or `unknown` (neither). The census and the analysis API report it
once the typing cases exist (P1 Part B).

A case may declare `expected_divergence = "<id>"` (not hashed). Verify then accepts it only when the
conversion relies on that divergence, Snowflake returned rows and DuckDB raised that divergence's
error: its guard's tagged raise, or a DuckDB error matching one of the divergence's `native_errors`
patterns (`numeric_range` and `timestamp_range` overflows). Another DuckDB error, a match, a
different value or a Snowflake error fails it, and the verdict names what matched. Such cases do
not count toward verifying a claim.

A raise on one row hides the values of the others, so verify then replays each input row alone
where result rows follow input rows one for one: an expression, or a query reading `input` in
order with no window, grouping, aggregate, join, subquery or set operation. A row that alone
returns a value other than its recorded row fails the case; one that raises again shows nothing.

A case expands to one source statement: the input rows as a `WITH input AS (...)` of casts, then
the expression (`SELECT <expression> AS value FROM input ORDER BY ordinal`) or the query. In an
equivalence file each case expands twice, once per spelling; the canonical one is referenced as
`<file>#<id>~canonical`.

### Project function cases

A case may define SQL project functions its expression or query calls, as `[[case.function]]`
tables: a lower-case `name`, `arguments` (name and Snowflake type, like input columns), exactly
one of `returns` (a scalar type) and `returns_table` (columns), and the `body`, a scalar function's
expression or a table function's query. Later functions may call earlier ones.

- Recording: the pending file carries `setup_sql`, one `CREATE FUNCTION ... AS $$ body $$` per
  function. The recorder runs such a case on its own in the session's schema (set
  `SNOWFLAKE_SCHEMA` to a scratch schema; the recorder never creates one): it renames each
  function `<name>__sqlquack_<first 16 hash digits>_<run nonce>` in the setup and case SQL,
  creates them, runs the case and drops them again, also when the case fails. A failing `CREATE FUNCTION` stops the
  run; it is a case-file error, not an outcome.
- Replay: verify types the functions into the conversion's schema, converts each body with
  `analyze_function`, and creates its macro, `CREATE MACRO "NAME"(p1, ...) AS ...`, before the
  converted query. A body that does not convert refuses the case, and the rules each body used
  count toward the case's provenance.
- Python functions are not case functions yet.

### Setup cases

A case may run Snowflake statements before its SQL, as `setup`, an array of single statements
(no `;`). Each starts with one of `CREATE [OR REPLACE] TABLE`, `VIEW`, `TYPE` or `SEMANTIC VIEW`,
`INSERT INTO`, `SET`, `UNSET`, `ALTER SESSION SET` or `ALTER SESSION UNSET`; loading rejects
anything else, and an `ALTER SESSION` naming a parameter the case's session profile sets or
pins, since receipts report the profile's values. A case with `setup` defines no functions. Setup and the case SQL may name
`{{run_database}}` and `{{run_schema}}`, the recording run's own database and the case's fresh
schema; any other `{{...}}`, or a placeholder in a case without setup, is rejected.

- Recording: the pending file carries `setup`, placeholders unsubstituted, beside `setup_sql`. The
  recorder's guard substitutes the placeholders, checks each statement fail-closed (statement kind,
  every named object inside the run database), and runs setup and query in a fresh session and
  case schema, dropping the run database afterwards. A failing setup statement is a case-file
  error, not an outcome.
- Replay: verify does not replay setup on DuckDB yet, so every setup case refuses.

### Hashes

The case hash is the SHA-256 of the canonical JSON of the evidence format version, the source
dialect, the session profile's Snowflake parameters and the expanded source SQL, plus any
`setup_sql`: the function statements, then the `setup` statements with their placeholders
unsubstituted (omitted when both are empty, so existing hashes hold). The `ALTER SESSION` a
setup runs is hashed this way too, not through the session profile. Descriptions,
tags and file locations are not hashed. Any edit that can change the result changes the hash, so
it becomes a new case with a new result.

### Session time zone

Converted SQL is proven only in UTC sessions: every session profile sets Snowflake's `TIMEZONE`
and DuckDB's `TimeZone` to `'UTC'`, and policy loading rejects any other. DuckDB sessions running
converted SQL must set `TimeZone = 'UTC'` too; `REQUIRED_DUCKDB_SETTINGS` lists it for callers.
A statement whose result depends on the session time zone relies on the named assumption
`session_timezone_utc`: reading a DATE, TIMESTAMP_NTZ or offset-less timestamp text as
TIMESTAMP_TZ or TIMESTAMP_LTZ (including CONVERT_TIMEZONE with two arguments of a naive value),
reading a TIMESTAMP_LTZ as a wall time, date part, VARIANT text or TIMESTAMP_TZ, and CURRENT_DATE
and CURRENT_TIMESTAMP. Such statements convert with a warning by default and are refused in
strict mode unless the caller confirms both sessions run in UTC (`confirm_utc_session`, or
`--utc-session` on the command line). The statement that reads a value in the session zone
carries the assumption; a UTC-offset value it passes on is a fact downstream. CONVERT_TIMEZONE
marks its result as holding the UTC offset only for the target spelled `'UTC'`, the spelling the
recordings prove.

### Session profiles

Each case records under a session profile. `utc-v1` pins Snowflake's documented defaults with the
session in UTC; `utc-qiic-v1` is the same with `QUOTED_IDENTIFIERS_IGNORE_CASE = true`, under which
quoted identifiers resolve to upper case like unquoted ones (recorded: values through quoted
references to unquoted names, `SELECT *`, joins and CTE names) while VARIANT path keys stay
case-sensitive. The comparator matches column names without regard to case, so recordings do not
prove the case of output names; that quoted aliases come back upper case under `utc-qiic-v1` is
Snowflake's documented resolution, checked by a CLI test on the converted SQL rather than by a
recording. Conversion takes the caller's session parameters and converts under the profile they
match: unset parameters take the defaults `utc-v1` pins; a parameter no profile covers is refused
by name unless it is one of `RESULT_NEUTRAL_SESSION_PARAMETERS`, which cannot change a result; a
`TIMEZONE` other than `UTC` refuses every statement relying on `session_timezone_utc`, overriding
`--utc-session`; and parameters matching no profile refuse the statement with a named reason.
`census` and `index` convert under `utc-v1` until they take session parameters. Under `utc-qiic-v1` conversion upper-cases every quoted identifier before resolving
names; one holding a non-ASCII character is refused, its folding unrecorded.

A recording that is a compile-class Snowflake error (SQLSTATE class 0A or 42) must come from a case
declaring `compile_error = true`, or one creating functions or running setup; verify fails it otherwise, so the
recorder's compile pre-check never blocks re-recording it.

### Pinned session parameters

A profile's `pinned` parameters (date, time, timestamp and binary input and output formats,
`TWO_DIGIT_CENTURY_START`, JSON output settings) are set in every recording session alongside its
hashed parameters, but are not hashed. The recorder stores each set and pinned parameter's value as
`SHOW PARAMETERS IN SESSION` reports it (`effective_parameters`) and the SHA-256 of every reported
parameter (`parameters_sha256`). Verify fails a result whose recorded value differs from the
profile's. Results recorded before pinning have no effective values; they are vouched for by
`evidence/session_defaults.json`, which `sqlquack-record --session-defaults` writes from a session
that sets no parameters; it keeps only the parameters pending cases set or pin, so no other account
setting reaches the repository. A pinned parameter whose account default differs fails those results, and
`verify --require-verified` fails while no defaults are recorded.

### Flow

1. `make pending DIALECT=snowflake` (`sqlquack pending`) writes `evidence/pending/<hash>.json` for
   every case without a result.
2. The maintainer runs `make record-convert DIALECT=snowflake` (`sqlquack pending`, then
   `sqlquack-record` with any options in `RECORD_ARGS`) from a machine with a Snowflake connection;
   CI never records, and the target refuses when `RECORDER=0`. Before running anything it compiles every pending
   case's SQL with `EXPLAIN`, which runs nothing and creates nothing, and when any fails with a
   SQLSTATE class 0A or 42 error it stops and lists them, so a mistake such as a reserved word used
   as an alias costs no recording. A case that records Snowflake refusing to compile its SQL
   declares `compile_error = true` (not hashed) to pass the check; cases creating functions are
   not compiled first, since their functions do not exist yet. It then runs the pending SQL in
   batches, one Snowflake
   session per set of session parameters, and writes `evidence/results/<hash>.arrow` (the
   connector's Arrow result, unconverted) and `<hash>.json` (the SHA-256 of the query ID,
   warehouse, connector and DuckDB versions, recording time, column descriptions, row count and
   the Arrow file's SHA-256). A SQL error in SQLSTATE class 0A, 21, 22 or 42 is recorded as the
   outcome instead, as its error number and SQLSTATE, plus the message of a data exception (class
   22) without its query ID; session, connection and privilege
   (42501) errors stop the run without writing anything. When any statement of a batch fails, Snowflake reports the whole
   request as error 100132 (SQLSTATE P0000), so the batch's cases are rerun one at a time and each
   real error is classified on its own. The connector returns a whole TIME or TIMESTAMP(9) column
   in microseconds when any row lies outside the nanosecond range. Such a result is kept only when
   one read-only `SELECT COUNT_IF(DATE_PART(nanosecond, $n) % 1000 <> 0)` over `RESULT_SCAN` of
   the same query finds no value with sub-microsecond digits; the column is then marked
   `sub_microsecond_checked` and compares exactly. Otherwise, or if the check fails, the case is
   refused: a rounded value is never recorded. Existing results are never overwritten.
3. `sqlquack verify` converts each recorded case, replays it on DuckDB in a fresh in-memory
   database under the profile's time zone and the policy's NULL order, and compares. A refusal is
   always acceptable; a difference fails. Equivalence pairs are compared with each other instead.
   Every proven equivalence and every rule is a claim; a claim is verified once at least one
   passed case relies on it (for rules, through the conversion's provenance). The input fixture
   (the outermost `input` CTE and an `ORDER BY ordinal` that only keeps rows in input order) is
   converted with provenance muted, so its casts, literals and set operation count neither toward
   claims nor toward the error check's declared failures; a query's own CTEs follow the fixture's
   in the same `WITH` and do count. `make check-convert DIALECT=snowflake` runs `make
   verify-recorded`, which runs `verify --require-verified --strict-errors` and fails while any
   claim is unverified; CI and `make gate` both run it, so both use those flags. `CASES=<folder>`
   replays only that folder, with `--strict-errors` alone. `make check-merge` does not replay
   recorded cases. A new case's results are committed with the case once `make check-convert
   DIALECT=snowflake` passes.
   Where Snowflake and DuckDB both raised, verify also checks the errors (see Error checks).
4. `sqlquack review` writes every case as a sqllogictest-style record (`source snowflake`,
   `converted duckdb` or `refused`, then `query snowflake nosort|rowsort` with the column names and
   Snowflake types as comments and one tab-separated line per row). Values are exact: decimals at
   the declared scale, timestamps to the nanosecond, text with tabs and line breaks escaped. The
   format is for review only, because sqllogictest's own typing is too lossy to be evidence.
5. `sqlquack audit --errors` writes pending files for every case recorded as a warehouse error,
   to re-record it with its message into `evidence/audit/results`.
   Verify reads those messages (`--audit-results`) for stored errors kept without one, when the
   re-recording raised the same errno and SQLSTATE, so stored results stay append-only.
   `sqlquack audit --sample 10%` writes pending files for a reproducible sample of recorded cases
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

Results are public, so they hold nothing specific to the recording account. The raw query ID is
replaced by its SHA-256, which the maintainer can match against query history by hashing.
Compilation and other error messages can name databases, schemas, roles and other objects, so
only data exceptions (SQLSTATE class 22) keep their message, whose values come from the case. Column names, session parameters and values all come from the case itself, and the
warehouse version is Snowflake's release number.

CI runs `make evidence-guard`: any modified or deleted file under `evidence/results` fails, and an
added result must be named by a case hash, have exactly the recorder's metadata keys, name
`sqlquack-record` as its recorder and carry an Arrow file matching its hash. Its query ID must be
a SHA-256 digest, and an error may hold only `errno`, `sqlstate` and a text `message`.

### Error checks

A raised error passes only as the same failure. `sqlquack-diagnostics`' `failures` module maps each
recorded Snowflake error (errno and SQLSTATE) to a kind (numeric overflow, number text, date
text, division by zero, JSON parse, VARIANT cast, string too long, …) and DuckDB messages to
kinds. Building blocks raise tagged errors, `sqlquack:<errno>: <message>`, naming the exact
Snowflake error they stand for; a text conversion raises text's error for a cast of text and
Snowflake's VARIANT-cast error (100071) for a cast of a VARIANT string. Each rule declares its
failure modes: a kind and the Snowflake errors meaning it.

Every raise carries an ID naming the expression it guards: `sqlquack:<errno>#<id>: <message>` for
a Snowflake error and `<divergence>#<id>: <message>` for a guard. A raise is emitted waiting for
its ID (`#?`); when the expression, query or FROM source holding it is converted, the ID becomes
twelve hexadecimal digits of a hash of that converted SQL, without any raise ID, and the raise's
tag. The ID depends only on content, so the same expression prints the same SQL wherever it
appears (DuckDB matches a select expression to a GROUP BY key by its text, constants included),
and reformatting the source never changes the converted SQL. Positions stay out of the SQL: the
conversion result's `raises` maps each ID to every source location of its expression, and
`Converted::raise_locations` resolves an error message through it. Error matching compares only
what it compared before: the errno before the ID, and a guard's divergence name followed by `:` or
`#`. A replay that raises where Snowflake returned rows names the locations in its failure, and
`sqlquack review` lists each raise ID with its locations.

The located parse keeps the positioned tree beside the stripped one conversion reads, so a
statement's tree is held twice while it converts; this is a known cost.

Converted SQL raises with `duckdb.raised`, `CAST(CAST('<message>' AS INTEGER) AS "NULL")`, not
DuckDB's `error()`: `error()` is volatile, which window PARTITION BY and ORDER BY keys refuse, while
a failing cast of a constant is left in place by constant folding and raises only when evaluated.
DuckDB reports it as `Conversion Error: Could not convert string '<message>' to INT32`; the tag
or divergence name inside the message is what verify reads.

For a passed case where both raised, verify reports `agrees` (same kind, and a rule the
conversion used declares the recorded error), `divergence_raise` (DuckDB raised a known
divergence's guard), or a finding: `errno_mismatch` (a block tagged another errno),
`kind_mismatch`, `undeclared` or `unknown_snowflake_error`. Findings are reported and counted;
`verify --strict-errors` fails on them; `make verify-recorded`, and so CI and `make gate`, pass it.
