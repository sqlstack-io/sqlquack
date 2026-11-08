# Databricks SQL syntax catalogue

The documented SQL syntax of Databricks SQL 2026.44 as stable form IDs `db.<family>.<slug>`,
with the same layout and checker as the Snowflake root in `coverage/syntax`, the PostgreSQL
root in `coverage/syntax-postgres` and the DuckDB root in `coverage/syntax-duckdb`.
`dialect.toml` names the dialect for `syntax-coverage`.

## Sources

- Documentation: the Databricks SQL language reference
  (`https://docs.databricks.com/aws/en/sql/language-manual/`), retrieved 2026-11-06. The page
  set is every page of the docs sitemap under the language manual, plus three function pages
  that the function indexes link but the sitemap leaves out (`index_linked` in
  `scripts/syntax_catalogue_databricks`): 1,191 pages. `sources.toml` lists each page with the
  sha256 of its HTML and of its Syntax blocks, its status (1,102 with forms, 89 with no Syntax
  block) and the reviewed decisions about it; `indexes.toml` hashes the sitemap, the page list
  and the grammar files. The pages themselves are not committed.
- Grammar: `apache/spark` tag `v4.2.0` (`32f7299601108917fb01920a54e084595b7b3bf8`),
  `SqlBaseLexer.g4` and `SqlBaseParser.g4`, and the parser helpers that read option names
  outside the grammar (`AstBuilder.scala`, `DataTypeAstBuilder.scala`, `SparkSqlParser.scala`).
  Databricks SQL's parser is not published; the Spark grammar it extends is the independent
  check the omission audit reads.
- Engine: a Databricks SQL serverless warehouse whose `current_version()` reports
  `dbsql_version` 2026.44. Every receipt records that value.

## Regenerating

```sh
python -m scripts.fetch_syntax_docs_databricks /tmp/dbx-pin
python -m scripts.build_syntax_catalogue_databricks --cache /tmp/dbx-pin
python -m scripts.audit_syntax_catalogue_databricks --cache /tmp/dbx-pin
python -m scripts.build_syntax_obligations_databricks
python -m scripts.run_databricks_referee run --cases coverage/syntax-databricks/cases/query.jsonl \
    --out /tmp/run --private /tmp/private
```

The referee reads `DATABRICKS_HOST`, `DATABRICKS_WAREHOUSE_ID`, `DATABRICKS_CLIENT_ID` and
`DATABRICKS_CLIENT_SECRET` from the environment (OAuth machine-to-machine). The catalogue
scripts reuse the shared grammar normaliser in `scripts/syntax_catalogue`, and the obligation
builder reuses `scripts/syntax_obligations`, by import.

## Catalogue

1,230 forms: query 42, dml 13, ddl 107, transaction_session 28, security_admin 15, utility 80,
scripting 18, functions 847, expressions 80. Statement forms come from each page's Syntax
blocks, read in the pages' own notation (`[ ]` optional, `{ | }` choice, `[ , ... ]` a
separated repetition); a page's Literals blocks give the literal forms. Functions are one form
per documented function page. `hand.toml` holds the reviewed readings of Syntax lines the
notation cannot read as written (a stray bracket, an examples prompt inside the block, braces
the page leaves out), each bound to its page's sha256 with its reason.

## Omission audit (`audit.toml`)

`audit.toml` is an independent check of the catalogue against the pinned Spark grammar. Every
keyword sequence a grammar rule spells must be contained in a catalogued form, also inside the
form's case context, or carry a reviewed `[[gap]]` reason; every grammar keyword and every
option name the parser helpers dispatch on must be spelled by some form or carry a reviewed
reason. Keywords match strictly, token for token; `==` reads as `=` and `!` as `NOT` by the
reviewed `[[synonym]]` rows. The audit fails on an unreviewed or stale row and on a `[[gap]]`
row that lists a sequence which is not a gap.

An `[[unseen]]` row records a lexer token the gap walk cannot compare (it yields no keyword
sequence), with its reason. `UNSEEN_TOKENS` in `scripts/syntax_catalogue_databricks/constants.py`
lists them: the audit fails when a listed token has no row, when a row names a token not
listed, and when the lexer has a token that is neither compared nor listed, so a deleted, stale
or new token is caught. `tests/unit/scripts/syntax_catalogue_databricks/test_review.py` checks
this guard.

At the pin: 279 rules (3 unwalked), 10,135 keyword sequences, 3,259 gaps under 146 reviewed
`[[gap]]` rows, 21 reviewed `[[unseen]]` tokens, 427 keywords (47 not catalogued) and 51
helper option words (6 not catalogued), 0 problems.

## Obligations and cases

`obligations/` and `cases/` are generated from the catalogue grammars by the shared builder.
`limits.toml` holds the hand-written inputs: `[[reading]]` rows that replace a grammar for
derivation only (bound to the grammar's sha256, with the reason the documented grammar is not
what the warehouse accepts), `[[suppress]]` rows that withdraw one derived obligation with its
reason, and `[[limit]]` rows that add a hand-written obligation with its SQL. Where the
warehouse refuses a spelling the page documents, the reading leaves it out of derivation and a
`[[limit]]` row keeps the documented spelling as an observed case, which its receipt settles.

Identifiers follow the reserved-words page: Databricks formally disallows no literal as an
identifier, so an identifier operand's keyword case writes `SELECT` unquoted and in backticks,
both expected to be accepted; a table alias also gets a reserved case on `ANTI`, which the page
reserves as a table alias, refused unquoted and accepted in backticks. Quoted identifiers are
written in backticks.

An obligation keeps its id across rebuilds. `retired.toml` lists every id that has left the
root, grouped by the commit that retired it, with the reason; the builder reserves those
numbers, so they are never issued again, and fails if a retired id reappears in `obligations/`
or if a rebuild drops an id that `retired.toml` does not list, so a changed meaning always takes
a new id. `tests/unit/scripts/syntax_obligations_databricks/test_tombstones.py` checks both.

14,568 obligations and 9,612 cases: query 859/449, dml 560/341, ddl 3,820/2,759,
transaction_session 199/133, security_admin 291/205, utility 1,089/786, scripting 359/237,
functions 6,673/4,388, expressions 718/314.

Syntax documented for Databricks Runtime only is observed, not expected, on the warehouse. In the
query, dml and ddl families every positive case of a form whose page's applies-to banner leaves
out Databricks SQL (its `sources.toml` decision) is `observe`, and so is each form that is one
Runtime-only clause of a page that applies to Databricks SQL, under the clause's own banner
(`RUNTIME_ONLY_CLAUSES` in `scripts/syntax_obligations_databricks/constants.py`); such a case
stays observed whatever its receipt says, unless a form that applies to Databricks SQL also
expects the same text. A Runtime-only clause inside a form that applies to Databricks SQL
(GLOBAL TEMPORARY views, ROW FORMAT in CREATE TABLE LIKE) leaves the form's reading and is kept
as observed `[[limit]]` rows that quote the banner.

## Receipts

`receipts/` holds the referee runs the checker reads (receipt format
`sqlquack-syntax-receipt/2`); `runs.toml` records each run, the cases it covered and its repeat
run, a second run of the same cases in a shuffled order kept in `repeats/`, which the checker
does not read, so that each case has one current receipt. Compare a run with its repeat with
`python -m scripts.run_databricks_referee compare receipts/<run>.jsonl repeats/<run>.jsonl`.

Every family is recorded, in the runs `runs.toml` lists. Each run is repeated in a shuffled
order; every repeat compares with 0 differences except the sequential phase's, which `runs.toml`
explains. The core run replaced three earlier core runs, whose receipts were all either
re-recorded or of SQL texts no case now has.

The referee runs each case through the SQL Statement Execution API in a schema of its own in
catalog `workspace`: it creates the schema, runs the setup (`SETUP` in
`scripts/syntax_obligations_databricks/constants.py`), runs the case with autocommit, and drops
the schema with `CASCADE`, so no case sees another's objects; the shuffled repeat checks that.
A run fails if any of its schemas is left behind. Sixteen cases run at a time, each under one
300 s deadline.

A case whose text changes an account- or catalog-level object (a catalog, a schema of
`workspace`, a connection, a share, a recipient, a provider, an external location, a credential,
a governed tag, a group, a policy, or a grant on one of them) is held: a run stops before any
statement runs. Held cases are recorded by `run --sequential`, one case at a time. Before the
first case and after each one, the referee reads the account state over REST (`SNAPSHOT_KINDS`
in `scripts/databricks_referee/constants.py`): catalogs, the schemas of `workspace`, connections,
shares, recipients, providers, external locations, storage and service credentials, tag
policies, groups and their members, the metastore's and `workspace`'s grants, and the tables,
functions and volumes of `workspace.default`. Each object is recorded by the fields a change
alters. After a case, an object it added under a reviewed fixture name (`FIXTURE_NAMES`) is
dropped by its kind's reviewed statement (`DROPS`), and the state must then equal the baseline.
Any other change stops the run with a private report, writes no receipt and leaves the state for
review. That covers a removed or changed object, any other added object, and a failed drop. A
completed run writes its receipts as a parallel run does, plus a private log of what each case
dropped. The phase cannot see what its REST reads do not cover: the referee's principal may not
read metastore policies, and changes to users, policies on objects other than the metastore,
or objects outside these kinds go unseen.

The phase follows one rule: a refusal whose message carries the `== SQL ==` excerpt is a
`syntax_error` when its class is reviewed as one the parser or the parse-tree builder raises,
whatever its SQLSTATE says; a failure without the excerpt is never a `syntax_error`. Spark also
writes the excerpt (`== SQL (line N, position M) ==`) under ANSI runtime errors such as
`DIVIDE_BY_ZERO`; those classes are reviewed in `EXCERPT_RUNTIME_CLASSES` as `runtime_error`,
never `syntax_error`, and any other excerpt-bearing class stops the run. The class is read in
full, with its subclass, from the start of the message (`[CLASS.SUBCLASS]`, or
`[RequestId=… ErrorClass=CLASS]` for a Unity Catalog service error, whose request id receipts
mask), and only a reviewed class is counted: `PARSER_CLASSES` lists the classes the parser
raises and `LATER_PHASES` gives the phase of each class raised after parsing, in
`scripts/databricks_referee/constants.py`. A message with no error class is read only by an
anchored pattern, in `UNCLASSED_PARSER` when it carries the excerpt and in `UNCLASSED_LATER`
with its phase when it does not. Any other failure, including a parser class or a later class
that is not reviewed, stops the run before a receipt is written and lists every such case in a
private report. So a case the parser refuses can never pass as accepted syntax: an accept case
whose text the parse-tree builder refuses is a wrong expectation, and is fixed in the case.

`syntax_codes` in `dialect.toml` lists the codes a `syntax_error` receipt carries: 42601
(`PARSE_SYNTAX_ERROR` and the builder's syntax classes), 42617 (`PARSE_EMPTY_STATEMENT`), 0A000
and 23505 (builder refusals of unsupported combinations and repeated keys), 22023 (the builder's
`INVALID_TABLESAMPLE_FRACTION`), and `PARSE`, the
referee's code for a reviewed parser message that carries no SQLSTATE (an `Operation not
allowed` message, for example); such a receipt's `sqlstate` and `parse_only_sqlstate` stay
empty, as the engine sent them.
`tests/unit/scripts/databricks_referee/test_phases.py` checks these rules.

## Checking

`dialect.toml` names `local_parser = "databricks"`, so the checker gives each case the strict
verdict of `crates/sqlquack-syntax-databricks`'s `parse_strict`:

```sh
cargo build -p sqlquack-syntax-coverage --bin syntax-coverage
target/debug/syntax-coverage validate --root coverage/syntax-databricks
target/debug/syntax-coverage summary --root coverage/syntax-databricks
```

`validate` prints `valid, 1230 forms, 9611 receipts (0 superseded or withdrawn)`. A form
verifies only when every required obligation is met and a current family review covers its
fingerprint. No family review exists yet, so `summary` prints `checked 1230 forms, 0 problems`
and `0 of 1219 forms verified (0%)`, with obligations 11666/14572. The 11 forms without
obligations are not counted.

- 966 forms meet every obligation and wait only for review.
- 150 forms have a receipt that disagrees with the case's expectation (`UnexpectedOutcome`).
- 146 forms have an opaque case: the parser gaps the statement into a `Command` node.
- 12 forms await an observation, and 12 have a case the parser and the engine disagree on.
- 2 forms lack a receipt. One is `db.ddl.create-recipient`: its cases
  `db.ddl.create-recipient.k6af5ffd8d13d` and `k86915abb1b06` (`CREATE RECIPIENT v1 PROPERTIES
  ( v1 )` and `( 'v' )`) ran to the 300 s deadline in both sequential runs. Retried with a 900 s
  deadline, both answered after about 610 s with an internal engine error (`Cannot invoke
  "java.lang.CharSequence.length()" because "sequence" is null`, SQLSTATE 42000, with no parser
  excerpt). That error proves no phase, so no reviewed row maps it, and the form stays
  unverified.
  Another is `db.scripting.iterate-stmt` (o001, o003): its case `k1d3a5a4a6874`, a LOOP ITERATE
  with no LEAVE, runs to the 300 s deadline, so no phase can be observed, and it is left out of
  its run (`runs.toml`).

A form counts once even when several reasons apply, so the reasons overlap.
