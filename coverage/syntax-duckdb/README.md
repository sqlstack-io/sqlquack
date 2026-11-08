# DuckDB syntax catalogue

The documented SQL syntax of DuckDB 2.0.0 as stable form IDs `dd.<family>.<slug>`, with the
same layout and checker as the Snowflake root in `coverage/syntax` and the PostgreSQL root in
`coverage/syntax-postgres`. `dialect.toml` names the dialect for `syntax-coverage`.

## Sources

- Documentation: `duckdb/duckdb-web` at `b68c2955a464bac75ffca9e679e28e36956e14f1`, the
  `docs/2.0` tree, its railroad diagrams and its function and type tables.
- Grammar and engine: `duckdb/duckdb` tag `v2.0.0` (`d54cbbe3df17e8e5ce68312f483b4155836112aa`),
  the PEG grammar in `src/parser/peg/grammar/statements/*.gram`, and the Python package
  `duckdb==2.0.0` for the engine's function, keyword and type lists.
- `sources.toml` lists every page of the page set with its hashes and its decision;
  `indexes.toml` hashes the page list and the grammar files. The pages themselves are not
  committed.
- The page set is `PAGE_GLOBS` in `scripts/syntax_catalogue_duckdb/constants.py`: 148 of the
  464 pages of `docs/2.0`. The other 316 belong to the reviewed excluded page groups of
  `PAGE_EXCLUSIONS` beside it (clients, extension pages, guides, developer and operations
  documentation, and so on), each glob with the reason its pages are not catalogued. The build
  fails when a page is neither in the page set nor in an excluded group, or when a group
  excludes nothing; a `--draft` build and the audit run the same check.

DuckDB 2.1 and later come as an in-place rebuild: re-pin both commits, rebuild and re-audit.

## Release and product scope

Every sentence of the 148 catalogued pages that scopes syntax to a release, a deprecation or
a product was read, not only the headings. None scopes a form or an option to a release after
2.0.0: the release notes name 2.0 or earlier releases, and the syntax they introduce is in
2.0.0. So no form or option leaves the catalogue on release grounds.

- Deprecated syntax that 2.0.0 still parses stays catalogued: `PRAGMA <name> = <value>`, and
  `SHOW { <name> | <query> }`, whose `<query>` alternative (describe.md's Alias section and
  show.md's first paragraph; describe.gram `ShowDeprecatedSelect`) the sweep found missing and
  added. The keyword walk cannot see it, since `SHOW` is the only keyword the rule spells.
- The single-arrow lambda `x -> x + 1` (lambda.md, Deprecated Syntax) is the production of the
  `->` operator (expression.gram `LambdaArrowExpression <- LogicalOrExpression SingleArrowPair*`),
  catalogued as the JSON form `<json> -> <path>`. Whether the arrow is a lambda is decided by the
  binder: 2.0.0 refuses it by default and accepts it after
  `SET lambda_syntax = 'enable_single_arrow'`, so it is semantics and adds no form.
- Options only an extension provides (the `fts` pragmas) are values of the catalogued
  `<pragma-name>` placeholder, not forms; extension pages are an excluded group.

## Documented but unsupported syntax: observed limits

One rule, as in the SQL Server root (D337, D266): a statement, clause or option that an in-set
page names, even if only to say DuckDB 2.0.0 does not support it, is a documented form and stays
in the denominator. Where 2.0.0 refuses it, wherever the refusal happens (while parsing, in the
binder or at execution), it is an observed-limit form: its obligations are hand-written
`[[limit]]` rows in `limits.toml` with `expect = "observe"`, each with SQL that exercises the
refusal (with the tables it needs created in the same text), and a `[[reading]]` keeps the
supported part of a form derived as usual. The receipts record the refusal, but the cases are
never settled or scored: `UNOBSERVABLE_FORMS` (whole forms) and `UNOBSERVABLE_TEXTS` (refused
texts of otherwise supported forms) in `scripts/syntax_obligations_duckdb/constants.py` keep
them `expect = "observe"`, out of D133 settlement, as the SQL Server root's lists do. So an
observed-limit obligation is never satisfied, and an observed-limit form stays in the
denominator without verifying. A restriction on combining syntax the forms already spell (EXPLAIN (SQL) with
ANALYZE, RETURNING with DISTINCT, a CTAS with constraints, RETURNING or ON CONFLICT on a table
with row triggers) is a negative or a semantic rule, not a form.

| Syntax | Page | 2.0.0 refuses it | Catalogued as |
| --- | --- | --- | --- |
| `ALTER DATABASE ... RENAME TO` | alter_database.md | while parsing (syntax error at RENAME) | `dd.ddl.alter-database`, limit rows |
| `SET LOCAL`, `RESET LOCAL` | set.md, Scopes | while parsing (Not implemented) | `LOCAL` limit rows of `dd.transaction_session.set.set-to` and `.reset` |
| `ALTER TABLE ... ADD [ CONSTRAINT ] CHECK` | alter_table.md | at execution (Not implemented) | `dd.ddl.alter-table.add-check` |
| `ALTER TABLE ... ADD [ CONSTRAINT ] FOREIGN KEY` | alter_table.md | at execution (Not implemented) | `dd.ddl.alter-table.add-foreign-key` |
| `ALTER TABLE ... DROP CONSTRAINT` | alter_table.md | while parsing (Not implemented) | `dd.ddl.alter-table.drop-constraint` |
| `VACUUM FULL` | vacuum.md | while parsing (Not implemented) | `dd.utility.vacuum.vacuum-full` |
| `INSTEAD OF` triggers | create_trigger.md, Limitations | at execution (Not implemented) | `dd.ddl.create-trigger.instead-of` |
| Temporary triggers | create_trigger.md, Limitations | while parsing (Parser Error) | `dd.ddl.create-trigger.temporary` |
| `BEFORE ... FOR EACH ROW`, `UPDATE ... FOR EACH ROW` | create_trigger.md, Limitations | at execution (Not implemented) | limit rows of `dd.ddl.create-trigger.create-trigger-before-after`, whose reading takes FOR EACH ROW only after AFTER INSERT and AFTER DELETE |
| `ALTER TABLE ... ADD [ COLUMN ] ...` with a `PRIMARY KEY`, `CHECK` or `REFERENCES` column constraint | alter_table.md, Syntax | while parsing (Parser Error: not supported yet) | limit rows of `dd.ddl.alter-table.add-primary-key` |
| `COMMENT ON SCHEMA`, `COMMENT ON DATABASE` | comment_on.md, Limitations | while parsing (Not implemented) | `dd.ddl.comment-on.schema-or-database` |
| `FOREIGN KEY ... REFERENCES ... ON DELETE CASCADE` | create_table.md | while parsing (Parser Error) | `dd.ddl.create-table.foreign-key-on-delete-cascade` |

Spellings the grammar accepts beyond these documented forms (`DROP CONSTRAINT IF EXISTS`,
`INSTEAD OF` with `FOR EACH STATEMENT`, `ON DELETE SET NULL`, the other PostgreSQL `VACUUM`
options) are undocumented and stay reviewed gaps in `audit.toml`.

## Local parser limits

The DuckDB parser in `crates/sqlquack-syntax-duckdb` refuses some deep or long inputs that
DuckDB 2.0.0 accepts, so that no input exhausts the caller's stack or takes exponential time.
These are declared gaps, named below, not differences for the checker to find. No case reaches
them, since the deepest `nesting_depth` obligation asks for 8 levels, and the catalogue claims
nothing beyond them.

| Limit | Refused locally, with "the statement nests too deeply" | DuckDB 2.0.0 |
| --- | --- | --- |
| `nesting-depth-200` | past 200 nested levels: 196 to 198 nested parentheses, `FROM` subqueries, joins, calls, `CASE`s, `CAST`s, lists, subscripts, `NOT`s or set operations parse, and 98 nested scalar, `IN` or `EXISTS` subqueries, which count two levels each | accepts expressions up to 999 levels deep and does not limit query nesting |
| `type-depth-200` | a type taller than 200 levels, counting its constructors and its array suffixes together (`STRUCT(a INTEGER)[]` is three) | accepts types up to its expression depth |
| `tree-depth-8000` | a statement whose finished tree is more than 8,000 levels deep, counted iteratively after it parses: an `AND`, `OR`, `UNION`, `EXCEPT` or `INTERSECT` chain of about 8,000 links, whose tree is as deep as the chain, or 60 nested parenthesised queries each with a 200-link `UNION ALL` chain, where each level stays within the limits above but they multiply | reads `AND` and `OR` as one conjunction and does not limit set operations, so it accepts both at any length |

A flat chain of other operators or indirections (`::`, `.`, `[]`, `!`) is refused at 1,000 links
with DuckDB's own message ("Max expression depth limit of 1000 exceeded"), which is DuckDB's
boundary within two links. That is DuckDB's limit, not a local one.

## Regenerating

```sh
python -m scripts.fetch_syntax_docs_duckdb /tmp/dd-cache
python -m scripts.build_syntax_catalogue_duckdb --cache /tmp/dd-cache
python -m scripts.audit_syntax_catalogue_duckdb --cache /tmp/dd-cache
python -m scripts.build_syntax_obligations_duckdb --root coverage/syntax-duckdb
python -m scripts.run_duckdb_referee run --cases coverage/syntax-duckdb/cases/*.jsonl \
    --out /tmp/run.jsonl --private /tmp/private
```

The catalogue scripts reuse the shared grammar normaliser in `scripts/syntax_catalogue`, and
the obligation builder reuses `scripts/syntax_obligations`, by import.

## Catalogue

994 forms: query 28, dml 11, ddl 36, transaction_session 10, security_admin 2, utility 36,
functions 673, expressions 198. Statement forms come from each page's railroad diagrams;
functions and operators come from the documented signature headings and tables, one form per
function name with its documented signatures as its grammar lines. `hand.toml` holds the
reviewed forms read by hand from prose, tables, examples or synopsis blocks, bound to the
page's sha256, and the reviewed status of every page that yields no form.

## Omission audit (`audit.toml`)

`audit.toml` is an independent check of the catalogue against the pinned PEG grammar and the
engine's own lists. Every keyword sequence a grammar rule spells must be contained in a
catalogued form or carry a reviewed `[[gap]]` reason; every grammar keyword, engine function
and engine type must be spelled by some form or carry a reviewed reason. The audit fails on an
unreviewed or stale row. A gap that a `[[gap]]` row names under its own rule is a root with
its own reason even when it contains another rule's gap. An `[[unseen]]` row records a documented
or engine-accepted spelling left uncatalogued that the gap walk cannot see (it yields no keyword
sequence), under the walked rule that spells it, with its reason. `UNSEEN_RULES` in
`scripts/syntax_catalogue_duckdb/constants.py` lists the rules that need such a row: the audit
fails when a listed rule has no row, when a row names a rule not listed, and when a listed rule
is no longer walked, so a deleted or stale row is caught. The test
`tests/unit/scripts/syntax_catalogue_duckdb/test_review.py` checks this guard; run it with
`uv run pytest tests/unit/scripts/syntax_catalogue_duckdb` (part of `make test-python`). Its limits, including what the walk
cannot see (punctuation-only syntax and optional clauses that add no keyword sequence), are
written at the top of the file. At the pin: 1,120 rules (2 unwalked, listed, 4 reviewed
`[[unseen]]` rows), 1,585 keyword
sequences, 247 reviewed gaps (789 derived), 508 keywords (48 not catalogued), 1,161 functions
(442 not catalogued), 85 types (7 not catalogued), 0 problems.

## Obligations and cases

`obligations/` and `cases/` are generated from the catalogue grammars by the shared builder.
`limits.toml` holds the hand-written inputs: `[[reading]]` rows that replace a grammar for
derivation only (bound to the grammar's sha256, with the reason the documented grammar is not
what the parser accepts), `[[suppress]]` rows that withdraw one derived obligation with its
reason, and `[[limit]]` rows that add a hand-written obligation with its SQL.

An obligation keeps its id across rebuilds. `retired.toml` lists every id that has left the
root, grouped by the commit that retired it, with the reason; the builder reserves those numbers,
so they are never issued again, and fails if a retired id reappears in `obligations/` or if a
rebuild drops an id that `retired.toml` does not list, so a changed meaning always takes a new id.

8,818 obligations and 5,702 cases (25 of them observed): query 923/593, dml 623/415, ddl 1,182/901,
transaction_session 56/41, security_admin 61/36, utility 382/280, functions 4,250/2,636, expressions
1,341/800. At the pin every scored case's expectation matches the engine's verdict.

## Receipts

`receipts/` holds one run of the local referee against DuckDB 2.0.0 (receipt format
`sqlquack-syntax-receipt/2`), the run the checker reads. `runs.toml` records it and its repeat
run, a second run of the same cases kept in `repeats/`, which the checker does not read, so
that each case has one current receipt. Compare the two with
`python -m scripts.run_duckdb_referee compare receipts/<run>.jsonl repeats/<run>.jsonl`; at the
pin they do not differ.

The referee judges cases in a child process. Each case runs in a fresh in-memory database:
`extract_statements` alone decides syntax (`parser_check = "whole_text"`), then each extracted
statement is executed in turn and its result materialized inside the engine, so no result value
is converted to Python. One budget of 30 s covers setup, `extract_statements`, execution and
the `EXPLAIN` check; when it runs out, the child is killed and restarted. A case that outlives
the budget after the parse-only pass has decided is recorded as `TMOUT`, a runtime error, and
the run carries on; one that outlives it before then stops the run, naming the case and whether
setup or the parse-only pass ran out, since its syntax is undecided. A failing setup statement
also stops the run with the case, the statement and its error; the child catches that error
and keeps running, and is restarted only if it dies. A `ParserException` raised while
executing, after the parse-only pass has succeeded, comes from the binder or transformer: the
referee replays the statements before the failing one and runs `EXPLAIN` of the failing statement alone, and records a semantic error when
that raises the same message, compared up to any `LINE` context quoting the query, and a runtime
error otherwise. A case with no statements stops the run.

There is no SQLSTATE in DuckDB, so `engine_code` is a fixed five-character code per DuckDB
exception type. The receipt's error text keeps DuckDB's exception name and message verbatim,
so the mapping can be checked. The referee stops on an exception type it does not map; it
never guesses a code.

| DuckDB exception | `engine_code` | Phase |
| --- | --- | --- |
| `ParserException` | `PARSE` | syntax_error |
| `BinderException` | `BINDR` | semantic_error |
| `CatalogException` | `CATLG` | semantic_error |
| `ConversionException` | `CONVR` | runtime_error |
| `NotImplementedException` | `NOIMP` | unsupported_feature |
| `InvalidInputException` | `INVIN` | runtime_error |
| `OutOfRangeException` | `RANGE` | runtime_error |
| `ConstraintException` | `CONST` | runtime_error |
| `PermissionException` | `PERMS` | runtime_error |
| `TransactionException` | `TRANS` | runtime_error |
| No result within the budget, after the parse-only pass decided (the child is killed) | `TMOUT` | runtime_error |
| `Error` whose message starts "An error occurred while trying to automatically install the required extension" and says external access is "disabled by configuration" | `PERMS` | runtime_error |
| Any other `Error` whose message starts "An error occurred while trying to automatically install the required extension" | `EXTLD` | unsupported_feature |

`syntax_codes` in `dialect.toml` is `["PARSE"]`. `parser_path` names the reserved crate
`crates/sqlquack-syntax-duckdb`; until it exists `local_parser = "none"` and no form verifies.

## Checking

The root is checked with the coverage checker on `main` at
`fc15d512c240487b318d3b00802097d70663f7e6`, which reads `dialect.toml`:

```sh
cargo build -p sqlquack-syntax-coverage --bin syntax-coverage
target/debug/syntax-coverage validate --root coverage/syntax-duckdb
target/debug/syntax-coverage summary --root coverage/syntax-duckdb
```

`validate` prints `valid, 994 forms, 5702 receipts (0 superseded or withdrawn)` and `summary`
prints `checked 994 forms, 0 problems` and `0 of 994 forms verified (0%)`: with
`local_parser = "none"` every form is `LocalParseFailure` and no family review exists, so no
form verifies until `crates/sqlquack-syntax-duckdb` exists. The engine side is complete: every
case has one current receipt that agrees with its expectation.
