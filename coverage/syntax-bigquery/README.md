# BigQuery syntax catalogue

The documented syntax of GoogleSQL for BigQuery as stable form IDs `bq.<family>.<slug>`, with
the same layout and checker as the DuckDB root in `coverage/syntax-duckdb` and the PostgreSQL
root in `coverage/syntax-postgres`. `dialect.toml` names the dialect `bigquery` for
`syntax-coverage`.

## Sources

- Documentation: the GoogleSQL for BigQuery reference under
  `https://docs.cloud.google.com/bigquery/docs/reference/standard-sql/`, with the system
  variables and system procedures pages beside it. The reference is not versioned, so it is
  pinned by fetch date (2026-11-06): each page's Markdown export (`<url>.md.txt`), with its
  retrieval time and the sha256 of the fetched bytes, is a `[[source]]` row of `sources.toml`.
  A rebuild keeps a page's written retrieval time while its sha256 is unchanged, so a
  rebuild from a later fetch of the same pages rewrites no `retrieved_utc` line.
  The page set is every page the reference's navigation links (fetched from the query syntax
  page), closed under the links between reference pages: 191 pages. Eight linked addresses
  have no Markdown export (moved pages); the fetch records them as not found.
- Grammar: `google/googlesql` (formerly `google/zetasql`, Apache-2.0), tag `2026.11.1`,
  commit `d48c569aebd63870ff6acecf08b6611015ef1d9d`: the Textmapper parser grammar
  `googlesql/parser/googlesql.tm` with its semantic actions (the helper code that checks
  identifier spellings and language features inside the grammar), the keyword table
  `googlesql/parser/keywords.cc`, the lookahead transformer that turns keywords into their
  contextual tokens, and `googlesql/public/options.proto` for the language feature names.
  Each file is an `[[index]]` row of `indexes.toml`, named by repository, tag, commit and
  path, with its sha256.
- Pre-filter parser: Google's prebuilt GoogleSQL parser for the same tag, release asset
  `execute_query_linux` of `google/googlesql` `2026.11.1`, sha256
  `5e654aeec9c4bc41231991ab5b6a003ae51e08148ac4e64b30d438c6ae4218d9`. It runs as
  `execute_query_linux --mode=parse --sql_mode=script --enabled_language_features=MAXIMUM -`
  with the case text on standard input, from an empty temporary directory. It is offline and
  free; the obligation build refuses a binary whose sha256 differs (`PARSER_*` in
  `scripts/syntax_obligations_bigquery/constants.py`). The binary is not committed.
- The pages and the checkout stay in a private cache; nothing fetched is committed.

## Regenerating

```sh
python -m scripts.fetch_syntax_docs_bigquery /tmp/bq-cache
python -m scripts.fetch_syntax_docs_bigquery /tmp/bq-cache --verify-only
python -m scripts.build_syntax_catalogue_bigquery --cache /tmp/bq-cache
python -m scripts.audit_syntax_catalogue_bigquery --cache /tmp/bq-cache
python -m scripts.build_syntax_obligations_bigquery --root coverage/syntax-bigquery \
  --parser <dir>/execute_query_linux
```

The scripts read the cache only as data. Run them from the repository root, never from
inside the cache; to run them isolated, use `python -I -c "import runpy, sys;
sys.path.insert(0, '.'); sys.argv[0] = 'x'; runpy.run_module('scripts.<name>',
run_name='__main__')" <arguments>`, since `-I` leaves the current directory off the path.

The fetch refuses a cache directory that is not empty. It fetches the navigation, crawls the
reference, makes a sparse depth-1 checkout of the pinned GoogleSQL commit (verified against
the tag), and then compares every page and grammar file with the pins in `sources.toml`:
any page whose sha256 differs, any pinned page no longer reachable, any new page and any
grammar file whose sha256 differs from `indexes.toml` is listed, and the fetch exits 1. A clean fetch on the pin date reproduced all 191 page hashes.
When the documentation changes, the catalogue is rebuilt in place against a new fetch date.

## Catalogue

902 forms: query 77, dml 7, ddl 96, transaction_session 6, security_admin 17, utility 33,
functions 510, expressions 156. Statement forms come from each page's syntax and synopsis code
blocks. Blocks that follow prose (as the INSERT, DELETE, UPDATE and MERGE synopses do) are
read only when they look like notation, and blocks the build cannot read safely are
refused until reviewed. Function forms come from the signature blocks, one form per function
name with its documented signatures as its grammar lines. Operator forms come from the
operator precedence table and the operators page's Syntax columns, and type forms from the
data types summary table.

`hand.toml` holds the reviewed inputs, each bound to a sha256:
- `[[block]]` reviews (437): notation, corrected, restated, example, not_syntax or
  out_of_scope, each bound to the block's sha256. A restated block's words must appear in the
  page's forms. Corrected blocks repair the notation the page gets wrong: the MERGE clause
  repetition, the autoencoder `ACTIVATION_FN` braces, PCA, the graph-pattern property
  `name : value`, `STRUCT<T>`, and `ML.BAG_OF_WORDS` and `ML.TF_IDF` with their `OVER ( )`.
- `[[form]]` rows (31): forms read by hand from prose and tables, such as lexical literals,
  comments, `LEAVE`/`ITERATE`, system variables, dashed table names, `FOR SYSTEM_TIME AS OF`,
  procedural `RETURN`, chained function calls, and `ALTER COLUMN` with `SET GENERATED` and
  `SET OPTIONS ( … += … )`.

The build rewrites three notations before deriving: an ML `STRUCT` option written
`NAME AS name` becomes `<NAME> AS NAME` (the value first, as the examples write it), a
`STRUCT` of optional fields becomes one comma-separated list of the fields, and a named array
argument `=> [ <X> ]` keeps its brackets as literal tokens.
- `[[page]]` rows (9): the status of each page that yields no form, bound to the page's
  sha256.

A block, form or page review that names nothing current is a problem, and so is a changed
page. The pages for migrating from legacy SQL are out of scope: legacy SQL is a different
dialect.

## Omission audit (`audit.toml`)

`audit.toml` is an independent check of the catalogue against the pinned Textmapper grammar
`googlesql.tm` and the keyword table `keywords.cc`. The audit reads the grammar into rule trees
with its own reader. It walks every rule reachable from the grammar's `%input` entry rules and
the statement dispatchers, and spells each alternative as its keyword sequences: keyword-only
rules are inlined and every other nonterminal is a wildcard.

A sequence is covered only when one catalogued grammar line, with its page's definitions
inlined, spells each keyword run contiguously and in order. Placeholders and quoted strings
never count as keywords. Each uncovered sequence is a `[[gap]]` with a reviewed reason, unless
it is derived from another gap.

The audit also checks:
- every `keywords.cc` keyword;
- every word the grammar's actions compare identifiers with (`CaseEqual`, in the semantic
  actions rather than the grammar rules);
- every function in the reference's all-functions index;
- every type in the data types table.

**Scope: GoogleSQL versus BigQuery.** GoogleSQL is wider than BigQuery. Every
`FEATURE_*` language option that the actions test with `LanguageFeatureEnabled` (directly or
through a helper of the parser template) is a `[[feature]]` row, reviewed against the
reference as `enabled` or `not_enabled`:
- An `enabled` row must cite a catalogued form as evidence.
- A gap gated by a `not_enabled` feature must name that feature in its reason.
- A `googlesql_only` row's reason must start "GoogleSQL grammar, not a BigQuery feature". The
  audit fails if any code line or code span of the pinned pages spells one of its sequences.
- A `spelled_otherwise` row must be spelled by some page.
- An `outside_pinned` row's reason must start "BigQuery syntax outside the pinned reference":
  syntax a BigQuery page names that the pinned grammar has no rule for (`EXPORT TABLE
  METADATA`, `CREATE TEMP TABLE FUNCTION`).
- A row may name, in `mentioned_on`, pages that spell its words in passing (a reserved-word
  table, a "not supported" note). Every page it names must spell one of them, and no other
  page may.

Only code lines and code spans count as spelling: fenced blocks are read as code lines, and
escaped backticks never open a code span.

**Fail-closed guards.** The audit fails on any unreviewed or stale row in any table. It also
fails when a rule of `UNSEEN_RULES` (in `scripts/syntax_catalogue_bigquery/constants.py`) has
no `[[unseen]]` row, when an `[[unseen]]` row names a rule not in that list, and when a listed
rule is no longer walked. The test `tests/unit/scripts/syntax_catalogue_bigquery/test_review.py`
shows that deleting any reviewed row (a gap, a feature, a dispatched word or an unseen rule) is
a problem. The limits of the walk are listed at the top of `audit.toml`.

At the pin:
- 840 rules walked (5 unwalked, listed) and 33 rejected alternatives skipped;
- 5,421 keyword sequences, with 280 reviewed gaps in 119 rows (114 googlesql_only,
  3 spelled_otherwise, 2 outside_pinned) and 2,824 derived;
- 24 language features: 9 not enabled, 15 enabled;
- 342 keywords (87 not catalogued, reviewed), and 2 dispatched words not catalogued;
- 403 functions and 29 types, none uncatalogued;
- 0 problems.

## Obligations and cases

`obligations/` and `cases/` are generated from the catalogue grammars by the shared builder in
`scripts/syntax_obligations`. `scripts/syntax_obligations_bigquery` imports it and registers:
- the `bq.` styles: a statement as written, a function call after `SELECT`, an expression
  inside `SELECT ( … )`;
- contexts for clause forms: pipe operators after `FROM <table>`, clauses after
  `SELECT 1 FROM <table>`, and graph clauses inside `GRAPH <graph> MATCH ( n ) … RETURN n`;
- BigQuery fills for the catalogue's placeholders;
- a tokeniser that reads `|>` as one operator.

The BigQuery style also places each form where it can stand:
- table-valued functions (62) after `SELECT * FROM`, types inside `CAST ( NULL AS … )`;
- aggregate-only and named-argument forms in their clauses, graph predicates and subqueries
  inside a graph query, comments and dashed names in a statement;
- `OPTIONS ( name = value )` lists with a real option (`description = 'a'`), each placeholder
  with a fill of its own kind, and window functions with an `ORDER BY` where they need one.

`limits.toml` holds the hand-written inputs:
- `[[reading]]` rows (68) replace a grammar for derivation only, bound to the grammar's
  sha256. They record how the notation reads where it is ambiguous or wrong: CREATE
  FUNCTION's doubled parameter list, the CREATE TABLE choice inside parentheses, MERGE's
  repeated clauses, `LIKE`/`COPY`/`CLONE`, the graph subquery braces, the ML option-list
  commas, `PRUNING_METHOD`, `MAKE_INTERVAL`, the pipe `SELECT DISTINCT`/`AS STRUCT` and set
  operation modifiers, the label `%` and `|` operators. An empty grammar means a form's
  obligations are all `[[limit]]` rows: the `FORMAT` specifiers and the two pipe comparison
  examples read that way.
- `[[suppress]]` rows (584) each carry a `disposition` and a reason for their group. Being
  taken by GoogleSQL's parser does not make a text another BigQuery form, so only the 16
  `documented` rows withdraw a derived obligation, each because the text left is another
  documented BigQuery form: the graph path mode and quantifier, `GROUP BY ( c1 )`, CREATE
  FUNCTION and CREATE SCHEMA, DROP VIEW, `DROP TABLE f1`, implicit aliases and empty comments.
  The other 568 are `observe`: the obligation stays and its reject texts become `observe`
  cases, which the engine's receipts settle. Their groups: a table-valued function documented
  only with its MODEL or TABLE argument (143), or with that argument unnamed (139); an operand
  that leaves a unary operator, a shorter call or a literal (72); a dropped keyword or clause
  (55); an empty `OPTIONS ( )` (50); a column, field or parameter list missing its name or
  type, such as `CREATE TABLE t1 ( c1 )` or `RETURNS TABLE < c1 >` (42); a string literal
  where the parser takes an expression or path (36); an empty statement list (18);
  identifier_class (10); and `ENFORCED` without `NOT`, which the docs only write as
  `NOT ENFORCED` (3). So CREATE/DROP INDEX without SEARCH or VECTOR, ROW POLICY without
  ACCESS, `ENFORCED` without `NOT` and the name-or-type lists are observe cases. `DROP TABLE
  f1` from DROP TABLE FUNCTION stays withdrawn: it is the documented DROP TABLE statement on a
  table named f1, and what it does to a function of that name is for analysis, not syntax.
  The build parses every withdrawn text and stops if the parser refuses one, since a refused
  text is a valid negative and must not be withdrawn. An observe text the parser refuses is
  made a reject case, so the `ML . ( v1 ) OVER ( )` negatives of ML.BAG_OF_WORDS and
  ML.TF_IDF stay scored. For identifier_class the accept half (the backquoted name, such as
  `SELECT ( @`SELECT` )`) is kept as an accept case.
- `[[limit]]` rows (46) add a hand-written obligation with its SQL. Five are the forms an empty
  reading leaves to hand-written rows (the `FORMAT` specifiers inside a format string and the
  two pipe comparison examples). The other 32 are the documented bounds, each on both
  sides: NUMERIC and BIGNUMERIC precision and scale, JSON nesting at 500, arrays of arrays,
  and the INT64 literal range (decimal and hexadecimal). The pinned parser takes every
  out-of-range text except a negative scale, so those cases expect `accept` with
  `stage = "semantic"`: analysis, not the parser, refuses them. The receipts settle the stage.
  Nine are `observe` rows for the documented spellings that readings set aside to follow the
  parser: CREATE EXTERNAL SCHEMA without OPTIONS, DROP EXTERNAL SCHEMA with CASCADE or
  RESTRICT, consecutive joins with fewer conditions than joins, and STRICT CORRESPONDING with
  a mode (two set-operator rows and one per pipe operator). The pinned parser refuses each, and
  the engine decides. The MEASURES alias without AS and DECLARE without type or DEFAULT need no
  row, because their readings already make those exact texts reject cases.

**Parser gate.** The build runs every case, and every fixture's DDL, through the pinned
parser (see Sources) before it writes anything, with each `SQLQUACK_*` token replaced by a
plain name (`project1`, `dataset1`, `dataset2`). An accept case, and a reject case with a later
`stage`, must parse; any other reject case must not. The one reviewed exception is the open
build's object types: GoogleSQL's `GenericEntityTypeSupported` and
`GenericSubEntityTypeSupported` (googlesql.tm:3610, :3639) refuse `RESERVATION`, `CAPACITY`,
`ASSIGNMENT`, `BI_CAPACITY`, `REPLICA`, `ORGANIZATION` and `PROJECT`, which BigQuery registers.
A case refused only with `<type> is not a supported object type` passes (28 cases at the pin).
Any other disagreement stops the build: it must be fixed at its cause or carried by a
reviewed reading, suppression or limit. The build also stops on a parser refusal that no
syntax class of the referee maps (see Receipts).

| Cases against the pinned parser | before (89756c22) | after |
| --- | ---: | ---: |
| accept cases the parser refuses (less the object-type exception) | 702 (730 with it) | 0 |
| reject cases the parser takes | 327 | 0 |
| forms whose minimal positive has no parsing case | 150 | 0 |
| positive obligations with no parsing case | 2,266 | 0 |

The build prints these figures as `{"parser": …}` on every run. Before the receipts: 8,565
cases, 8,040 accept or reject, 2,780 that must parse, 28 under the exception, 0 disagreements.
With the receipts, the current figures are in Receipts.

**Ids.** An obligation keeps its id across rebuilds. `retired.toml` lists every id that has
left the root, grouped by the change that retired it, with the reason. The builder plans every
family first and checks the plan against the written root before it writes any file. It
refuses to write if a retired id would be issued again or an issued id would leave without a
`retired.toml` entry, so a refused build leaves the root unchanged and a second build
refuses again. `tests/unit/scripts/syntax_obligations_bigquery/test_tombstones.py` checks this
through the build's entry point. Retired numbers are not reused: the retired ids stay among
the ids the numbering skips. A rebuild from the same inputs is byte-identical. The catalogue
review of 89756c22 retired 2,118 ids: 1,513 in forms with suppressions (withdrawn, or no longer derived by a corrected
grammar), 571 in forms now derived through a reading, and 34 from corrected grammars.

13,225 obligations and 8,627 cases (obligations/cases): query 1,318/876, dml 218/164,
ddl 3,216/2,241, transaction_session 42/32, security_admin 168/144, utility 363/282,
functions 6,733/4,171, expressions 1,167/717 (before 12,747 and 8,051 in 898 forms;
at 153c7b86, when every suppression withdrew its obligation: 12,571 and 8,080).
Every case with a receipt has been run against BigQuery (see Receipts); the 19 held cases have no
receipt: they need paid capacity, a connection, or a resource outside the sandbox.

### Names in case SQL (placeholder convention)

Case SQL never names a real project or dataset. It uses three tokens, which the referee
replaces before submitting a case:

| Token | Stands for |
| --- | --- |
| `SQLQUACK_PROJECT` | the project the run's jobs run in |
| `SQLQUACK_DATASET` | the dataset the run creates for its cases, dropped at the end of the run |
| `SQLQUACK_DATASET2` | a second run dataset name, used where a statement creates, alters or drops a dataset (schema) |

A dataset-level name is written `SQLQUACK_DATASET.t1`. A name written with its project, as the
reference's `project_id.dataset.table` placeholders are, is backquoted as one path:
`` `SQLQUACK_PROJECT.SQLQUACK_DATASET.t1` ``, `` `SQLQUACK_PROJECT.SQLQUACK_DATASET.m1` ``, and
connections as `` `SQLQUACK_PROJECT.us.c1` ``. The referee must:
- replace whole tokens only, inside backquotes as well as outside;
- use run names that need no quoting, such as `sqlquack_run_<run id>`;
- hash `sql_sha256` over the text with the tokens, as committed, so receipts stay comparable
  across runs.

The shared builder's identifier obligations describe quoted identifiers as double-quoted, in
notes the shared builder writes. BigQuery delimits strings with double quotes and identifiers
with backquotes, so the BigQuery cases write those identifiers in backquotes: `` `a1 Q` `` and
`` `SELECT` ``; read "double-quoted identifier" in those notes as "backquoted identifier".

## Receipts

Recorded in the free-trial sandbox, dry runs except where execution is the evidence:
`receipts/bigquery-20261107T033339.716606Z.jsonl`, with a run in reverse order in `repeats/`
that `compare` reads as `8543 and 8543 receipts; 0 differ`. 8,543 cases have a receipt and 19
are held (no receipt; reasons in the run report): reservations, capacity and BI Engine (6),
project, IAM and data-policy statements (5), connections (4), a dataset or regional resource
outside the case's datasets (2), `BQ.ABORT_SESSION` (1) and one dry run that gave
`internalError` on every retry (1). Both runs billed 0 bytes. Phases: 5,279 `syntax_error`,
2,452 `semantic_error`, 24 `runtime_error`, 788 `accepted`. Every reject case got
`syntax_error` and no accept case did. Two runs at once exceed the
per-user job-insert rate limit, so the runs are made one after the other.

Two execution modes beyond the run datasets settle the cases the first runs held, under one
spend cap:

- DDL, `TRUNCATE TABLE`, `EXPORT DATA` and the row access policy drops execute in two datasets
  made for that one case (the run dataset's name with `_c<n>`), holding only the case's own
  fixtures, and dropped after it. Their receipts' basis says so. Such a case is held instead
  when its text names a `region-` resource, a schema other than the two dataset tokens, or a
  statement target (after `TABLE`, `VIEW`, `INTO`, `INSERT`, `DELETE`, `CLONE` and the like,
  or the table after an index's or row access policy's `ON`, never a join condition) qualified
  by any other dataset, or when its dry run reports a table, routine, DDL target or destination outside the
  run datasets. `EXPORT DATA` is held unless every uri is in the placeholder bucket `gs://b/`,
  which BigQuery refuses before it writes (`Bucket name is invalid`, `INVLD`).
- `LOOP` and `WHILE TRUE` scripts run with a 10 s job timeout and are cancelled if still running
  at 30 s; the stop is the receipt (`TIMEO` for BigQuery's timeout, `CANCL` for a cancel), phase
  `runtime_error`. Every one at the pin ended on the timeout.

The spend cap: every job keeps `maximumBytesBilled`, and the run stops if executions pass 1 GiB
billed. The run report lists `case_bytes_billed` for each case run in its own datasets or to a
stop: all 73 billed 0 bytes. The guards on dataset targets and the bucket were added after the
recorded runs; replayed over both runs' recorded dry runs, they change no case's mode.

What the execution receipts prove, when cited: an `EXIST` or `INVLD` receipt proves only that
the text parsed, since BigQuery refused the fixture or the value after the parse. A `TIMEO`
receipt proves that the script parsed and started. An accepted case run in its own datasets ran
against that one case's fixtures only.

With the receipts in place the build settles every observe case that has one, and the parser
gate yields to the engine: a case whose own receipt gives its expectation is not refused because
the pinned parser disagrees. The build prints these as `engine_over_parser` (4 at the pin): three
string literals in table position and `CREATE PROPERTY GRAPH g1` with no tables, which the
pinned parser takes and BigQuery refuses as syntax errors. The figures are now 8,627 cases, 8,624
accept or reject, 3,320 that must parse, 39 under the object-type exception, 0 disagreements.
8,608 cases have a receipt for their current text and the same 19 are held. The latest run,
`receipts/bigquery-20261108T081312.150491Z.jsonl` with its reverse in `repeats/`, covers the one
case the second `TIMESTAMP_DIFF` limit row added: `WEEK(MONDAY)` on DATE arguments, which the
page says behaves as `DATE_DIFF`, is accepted, and both runs billed no bytes. The run before it,
`receipts/bigquery-20261108T022009.099039Z.jsonl` with its reverse, covers the 4
cases the datetime date-part limit rows added (`ISOWEEK` for `DATETIME_TRUNC` and `EXTRACT`, and
`WEEK(<weekday>)` and `ISOWEEK` for `LAST_DAY`): all 4 are accepted, and both runs billed no
bytes. The run before those, `receipts/bigquery-20261107T203342.206143Z.jsonl`, covers the 46 cases
the CREATE MODEL corrections and the first date-part limit rows added or changed: the 18
date-part cases are accepted and `TIMESTAMP_DIFF` with `WEEK(MONDAY)` is refused at analysis
only.

An accept receipt shows that BigQuery parsed the text, not always that it analysed it. Many
settled accept cases get an analysis refusal from the fixture, such as `Column already exists`
for `ADD COLUMN c1`. Those receipts are parse evidence only; don't cite them as analysis
evidence. Every case whose receipt falls in the `SHAPE` class must parse in the pinned parser,
and the build refuses a `SHAPE` row whose text the parser refuses, so a syntax message cannot
settle as analysis.

Project numbers in engine messages (`<number>:dataset`) are masked to `SQLQUACK_PROJECT` like the
project id, and the referee counts any it finds as a leak. The sandbox project id comes from the
`GCP_BQ_SANDBOX_PROJECT` environment variable.

The referee is `scripts/run_bigquery_receipts.py` (package
`scripts/bigquery_receipts`), modelled on the SQL Server referee. It writes receipt format
`sqlquack-syntax-receipt/2` with the capture `execute_autocommit_in_throwaway_database`.

```sh
uv run -q --no-project --with google-auth --with requests python -m scripts.run_bigquery_receipts \
  run --cases coverage/syntax-bigquery/cases/*.jsonl \
  --out coverage/syntax-bigquery/receipts/<run>.jsonl --private <private dir> [--reverse] [--workers 8]
python -m scripts.run_bigquery_receipts compare receipts/<run>.jsonl repeats/<run>.jsonl
uv run -q --no-project --with google-auth --with requests python -m scripts.run_bigquery_receipts \
  probe "SELECT 1 FROM SQLQUACK_DATASET.t1" [--family query]
python -m scripts.run_bigquery_receipts refusals <cache>/googlesql/googlesql/parser/googlesql.tm
```

`GCP_BQ_SANDBOX_SA_KEY` holds the service-account key JSON, or the path of its file, and
`GCP_BQ_SANDBOX_PROJECT` the sandbox project id; the referee stops if either is missing. No
committed file names the project, and the key is never printed or written. `compare` prints `N and N receipts; N differ`, as the SQL Server
referee does, and exits 1 when any receipt differs.

**Run datasets.** The project comes from `GCP_BQ_SANDBOX_PROJECT`. Each run creates `sqlquack_run_<run id>` (for
`SQLQUACK_DATASET`) and `sqlquack_run_<run id>_2` (for `SQLQUACK_DATASET2`). They are created
with a one-hour `defaultTableExpirationMs` and dropped with `deleteContents=true` when the run
ends, including when it fails. The referee also checks that both are gone. Every dataset call
is refused unless the name is one of this run's two, matches `^sqlquack_run_[a-z0-9_]+$` and
does not start `sqlload_`. A dataset that already exists is never adopted.

**Tokens.** `SQLQUACK_PROJECT`, `SQLQUACK_DATASET` and `SQLQUACK_DATASET2` are replaced as whole
tokens, inside backquotes too. `sql_sha256` stays the committed text's hash. Before an error
message is published:
- the run's project and dataset names are turned back into the tokens;
- job ids, URLs and emails are masked;
- each `[line:column]` is moved to its position in the committed text.

The referee refuses to write receipts or a report whose text still names the project, a run
dataset, a job, a URL, an email or an address. The submitted text, the raw replies and the
audit (which names the project and datasets) go to `--private` only.

**Fixtures and setup.** The fixture set is declared once (`FIXTURES` in
`scripts/syntax_obligations_bigquery/constants.py`), and each case records the fixtures it
needs as its `setup`, bound by `setup_sha256`. The builder scans the case text for the names
it uses in `SQLQUACK_DATASET`/`SQLQUACK_DATASET2`; each one must be a declared fixture or have
a reason in `ABSENT_OBJECTS` (`m1`: models need paid training, a connection or files in a
bucket; a dataset token where the object name goes), otherwise the build stops. A fixture's
setup includes what it needs (`g1` needs `t1` and `t2`). The fixtures are empty and free,
created with DDL in the run dataset only: `t1` and `t2` (`c1, c2, c3, v1 INT64`), the SQL
function `f1(x INT64)`, the procedure `p1(x INT64)` and the property graph `g1` over `t1` and
`t2`. The build checks that every fixture's DDL parses. The referee creates the distinct setup
statements of the cases it runs. It refuses a case whose `setup_sha256` does not bind its
setup, a setup statement that is not a declared fixture, and a fixture that names more than
the run dataset. Because every fixture is fixed DDL and no case that changes the datasets is
executed (DDL is only dry-run, see Execution), a case resolves against the same objects
whatever else is in the run, so receipts do not depend on which cases ran.

**Parse-only pass.** Every case is first a dry-run query job (`dryRun: true`,
`useLegacySql: false`), run on 8 workers. The receipts are kept in case order, so the run order
and `--reverse` do not change them. Every job carries `maximumBytesBilled` = 10 MB, dry runs
included; jobs that are not dry runs also carry a 60 s `jobTimeoutMs`. The report records
bytes processed and billed, summed from the jobs' `totalBytesProcessed` and
`totalBytesBilled`, with dry-run estimates kept apart from executed jobs. It also records the
start and end state of both run datasets: their options, tables (schema and row count),
routines and models.

**Error classes.** Each error is matched on its `reason` and on a pattern over the normalised
message, against a fixed table (`scripts/bigquery_receipts/constants.py` and
`parser_refusals.json`). The table gives `engine_code`, which is kept beside the verbatim
message:
- **Parser refusals:** `PR001`–`PR151` are `syntax_error`. Each is one distinct message of a
  `MakeSyntaxError` call in the pinned `googlesql.tm` (sha256 bound), whose dynamic parts read
  as `.+?`. Not every parser refusal starts `Syntax error:` (for example `FOR UPDATE is not
  supported`, `PR095`), so the phase does not come from the prefix alone. `refusals`
  regenerates the table from the pinned grammar. `PI001` (`EXCEPT must be followed by ALL,
  DISTINCT, or "("`) and `PI002` (`Reserved keyword '…' may not be used as a label name
  without backticks`) are the two refusals the parser raises from its own code
  (`parser_internal.cc`) rather than the grammar. Every refusal the pinned parser gives on the
  committed cases maps to one of these classes; the obligation build checks this on every
  run and stops on an unmapped one.
- **Generic syntax:** `SYNTX` is `syntax_error`, for any other message starting `Syntax error:`.
- **Analysis (from the first live run):** `SHAPE` takes the analyzer's statement-form
  restrictions (for example `CREATE INDEX is not supported`, `... cannot have both OR REPLACE and IF
  NOT EXISTS`, `columns must be named`), kept apart so how they count can be reviewed by code without
  a rerun. `OPTVT` (an option value of the wrong type), `TVFAR` (a table-valued function argument
  kind) and the catch-alls `ANLYS` (`invalidQuery`) and `INVLD` (`invalid`) are `semantic_error`;
  `QUALD` also takes the `invalid` reason the live engine gives. A catch-all never takes a message
  that names syntax or starts `Expected` or `Unexpected`: such a message stays unmapped and stops
  the run. On execution after a clean dry run, `SYSVR` (`System variable not found`) is
  `semantic_error` and `RUNTM` any other `invalidQuery`, `runtime_error`.
- **Run-dependent text:** a table snapshot decorator `@<ms>` and `Cannot read before <ms>` are
  masked as `@<snapshot>` and `<time>`, so time-travel cases give the same receipt in every run.
- **Semantic errors:** `UNREC`, `FUNNF`, `TVFNF`, `TYPNF`, `NOSIG`, `CASTL`, `QUALD` (all
  `invalidQuery`), `NOTFD` (`notFound`) and `EXIST` (`duplicate`) are `semantic_error`.
- **Stops:** permission, quota, billing, rate-limit, backend and HTTP 401/403/5xx errors stop the
  run. Rate limits and backend errors are first retried up to 6 times with exponential backoff.
  One exception: a case whose dry run gives HTTP 500 `internalError` on every retry is held
  (no receipt, listed in the private held file) rather than stopping the run, since that is the
  engine failing on one text; more than 5 such cases still stop the run as an outage.
- **Unmapped:** an error no row matches stops the run. It is listed with its count and a case,
  and its phase is never guessed. The raw records are still written to `--private`.

When the dry run raises more than one error, the first syntax error decides, else the first
error.

**Execution.** A clean dry run settles query, functions and expressions cases. Other cases are
judged by the policy rows in `constants.py`, the one place to extend:
- Scripts and procedural statements, sessions and transactions (`utility`,
  `transaction_session`) are executed as written.
- `INSERT`, `UPDATE`, `DELETE` and `MERGE` are executed inside
  `BEGIN TRANSACTION … ROLLBACK TRANSACTION`.
- Executions run one at a time. The run datasets' state is read after each, and the run stops
  if a case changed it.
- A case is never executed when the dry run reports a table, routine, DDL target or
  destination outside the run datasets, or when its text has DDL, DCL, `EXPORT`, `LOAD`, `INFORMATION_SCHEMA`,
  `EXTERNAL_QUERY` or a `region-` qualifier.

Held cases get no receipt. Each is listed with its reason in `<private>/<run>.held.jsonl`, and
the reasons are counted in the report. A case is held for:
- `EXPORT DATA`, `LOAD DATA`, `EXPORT MODEL`, or `CREATE … MODEL` (training or remote models);
- `REMOTE`, `CONNECTION`, `EXTERNAL`, `BQ.JOBS` or `BQ.ABORT_SESSION`;
- an endless `LOOP` or `WHILE TRUE`;
- reservations, capacity, assignments or BI Engine;
- otherwise by family: all other DDL (it would change the datasets the other cases resolve
  against, so it waits for a per-case dataset), `TRUNCATE TABLE`, and the other
  security_admin statements.

A simulated offline run over the 8,080 cases of 153c7b86, with each case's pinned-parser
verdict as its recorded dry-run reply, held 710 (451 of them DDL) and wrote 7,370 receipts; it
was superseded by the live runs (see Receipts).

**Open, to settle against the engine:**
- whether a dry run accepts multi-statement scripts and which statements it validates;
- whether DDL dry runs resolve names;
- the real reasons and message texts, and the classes they need;
- `syntax_codes`;
- whether `jobs.insert` dry runs need the location;
- whether rolled-back DML works outside a session;
- whether BigQuery takes the 28 cases under the object-type exception, and at which stage it
  refuses the out-of-range boundary cases the parser takes;
- whether the engine refuses, as syntax, anything the pinned parser takes (the semantic and
  runtime stages the receipts record).

`syntax_codes` in `dialect.toml` stays empty (a TODO) until the referee has seen BigQuery's
errors. `parser_path` names the crate `crates/sqlquack-syntax-bigquery`, and
`local_parser = "bigquery"` has the checker run it on every case; a checker that has no such
parser reports every case as `no_parser`, which must fail rather than compare equal.

## Local parser limits

Where the local parser and BigQuery differ by design rather than by a missing form:

- **Nesting.** The parser counts nested levels against a budget of 200 units and refuses deeper
  statements with "the statement nests too deeply", a parser check rather than a syntax error.
  A level can cost more than one unit: the budget allows 98 nested scalar, `EXISTS` and `IN`
  subqueries, parenthesised joins and set operations, lambdas and parenthesised table-valued
  function arguments, 197 parentheses, calls and `CASE` expressions, and fewer levels of mixed
  constructs (39 of `(SELECT f(-( ... )))`). It also refuses a statement whose tree nests more
  than 8,000 levels, which bounds flat chains and chains nested in another chain's operand.
  GoogleSQL's parser accepts all of these at 20,000 levels.
- **Nested keyword-led calls.** A keyword reading of a call such as `TRIM` or `POSITION` is tried
  at most once at each position, so nested calls take about quadratic time in their depth; the
  nesting budget caps it at about 0.4 s in a debug build and 31 ms in a release build at 195
  levels.
- **A string date part.** `EXTRACT('HOUR' FROM ts)` is refused by the local parser as a syntax
  error. GoogleSQL's parser takes it and its analysis refuses it ("A valid date part name is
  required"), so the verdict matches BigQuery's and only the syntax flag differs.
- **Type parameters only analysis refuses.** The local parser takes `MAX` as the length of
  STRING and BYTES and as BIGNUMERIC's precision, and refuses as syntax the parameters that
  GoogleSQL's parser takes and only its analysis refuses: `MAX` elsewhere (`NUMERIC(MAX)`,
  `NUMERIC(MAX, 2)`, `DECIMAL(MAX)`, `INT64(MAX)`, `TIMESTAMP(MAX)`, `BIGNUMERIC(MAX, MAX)`,
  `BIGNUMERIC(2, MAX)`), a second STRING or BYTES parameter (`STRING(MAX, 1)`,
  `BYTES(MAX, 1)`) and a string parameter (`STRING('MAX')`). The verdict matches BigQuery's and
  only the syntax flag differs. The pinned data-types page documents only `STRING(L)`,
  `BYTES(L)`, `NUMERIC(P[, S])` and `BIGNUMERIC(P[, S])`, so these are no catalogue obligations.

## Checker status

The checker built from `da3f754ba03410db07d225f5e8e21c06d2fd7878` (`wip/pg-integration`) reads
`form_prefix` and `families` from `dialect.toml`. It refuses this root only because
`syntax_codes` is empty:
`dialect.toml: syntax_codes are five-character codes, required with receipt format /2 only`.
With any non-empty five-character list, `validate --root coverage/syntax-bigquery` prints
`valid, 902 forms, 0 receipts (0 superseded or withdrawn)`, and `summary` reports every form
as `MissingReceipt`. The list stays empty until the engine has been observed. The audit's checker
finding (receipt format /2 and its capture) is settled by the case capture
`execute_autocommit_in_throwaway_database`, which the cases carry since 6f1005b16; nothing
else was needed for it. Receipts the referee wrote in the offline simulation above (a temporary
copy of the root with `syntax_codes = ["SYNTX"]`) validated: `valid, 902 forms, 7370 receipts`.
