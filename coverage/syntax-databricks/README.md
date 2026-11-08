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

The fetch downloads, into an empty directory, every reference page the sitemap lists, the
function pages its indexes link that the sitemap omits, and the pinned Spark parser files. With
`--root`, it reports how the fetch differs from that root's pinned sources (pages added or gone,
and pages whose article or Syntax blocks changed) and exits 1 if it does. A page's script bundle
names change on every deploy, so the article hash, not the page hash, decides whether a page
changed; for the same reason the page set that `indexes.toml` hashes lists each page's article
hash, which a redeploy of the docs site leaves alone.

The referee's commands are `run --cases FILE... --out DIR --private DIR [--shuffle N] [--select
IDS] [--sequential]`, `compare`, `recover --journal FILE --private DIR`, and `resume --journal
FILE` and `refinish --journal FILE`, each with the sequential run's own `run` arguments.
`--sequential` runs the cases one at a time, held cases included, restoring the account state
after each; `recover`, `resume` and `refinish` are described under the referee's tables below.

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

The pages write keywords in upper case and user-supplied parts as lower-case or camelCase words
(`table_name`, `elementType`). The catalogue writes keywords as they are, user parts as
`<snake_case>` placeholders and function names in upper case. A block is cut into productions at
blank lines where every bracket is closed; a lone lower-case word after a blank line names a
definition whose indented body follows it (`column_list` then `( ... )`). A statement's top-level
alternatives become one production each; a definition keeps its body whole as `<name> ::= body`.
An operator or predicate page also writes lower-case keywords: the code words of its title and
the connective words of `OPERATOR_WORDS`; no other page does. A form read under a later heading
than the page's first block (Literals) keeps that heading.

## Omission audit (`audit.toml`)

`audit.toml` is an independent check of the catalogue against the pinned Spark grammar. Every
keyword sequence a grammar rule spells must be contained in a catalogued form, also inside the
form's case context, or carry a reviewed `[[gap]]` reason; every grammar keyword and every
option name the parser helpers dispatch on must be spelled by some form or carry a reviewed
reason. Keywords match strictly, token for token; `==` reads as `=` and `!` as `NOT` by the
reviewed `[[synonym]]` rows. The audit fails on an unreviewed or stale row and on a `[[gap]]`
row that lists a sequence which is not a gap.

Containment is strict: a grammar sequence is covered only as a contiguous run of a catalogued
line. Each catalogued grammar line becomes an automaton over its upper-case words and the
punctuation the audit compares. Placeholders, quoted text, numbers and escaped notation
characters spell nothing, exactly as identifiers, literals and the other tokens spell nothing in
a grammar sequence. A placeholder that a definition of the line's page (or, failing that, of any
page) names stands for that definition or for nothing, as a rule too large to spell stands for
nothing. A `<..._clause>` placeholder after keywords stands for a clause page's form that opens
with those keywords, its shared lead cut. `[ ]` is optional, `{ a | b }` a choice and `...`
repeats the item it follows (`x [ , ... ]` reads x, then the separator and x again any number of
times). A sequence is covered when the automaton reads it without a gap from some state; a
statement's from the line's start.

A form's case context is the text around it in the statement its cases run in, as the
obligations write it. The parentheses of the expression context only keep a word left at the end
of a negative from being read as an alias, and are no part of the form: the context's closing
parenthesis would otherwise close a sequence the form leaves open, such as `FIRST ( … IGNORE
NULLS )`.

A `[[gap]]` row names a rule and alternative. A row with `sequences` covers exactly those
sequences; a row with `contains` covers every sequence of its alternative in which those words
appear together. A notation block is named `<page> § <heading> § <sha256 prefix of its text>`,
so a changed block is a new name and its old `[[block]]` row goes stale.

An `[[unseen]]` row records a lexer token the gap walk cannot compare (it yields no keyword
sequence), with its reason. `UNSEEN_TOKENS` in `scripts/syntax_catalogue_databricks/constants.py`
lists them: the audit fails when a listed token has no row, when a row names a token not
listed, and when the lexer has a token that is neither compared nor listed, so a deleted, stale
or new token is caught. `tests/unit/scripts/syntax_catalogue_databricks/test_review.py` checks
this guard.

At the pin: 279 rules (3 unwalked), 10,135 keyword sequences, 3,259 gaps under 146 reviewed
`[[gap]]` rows, 21 reviewed `[[unseen]]` tokens, 427 keywords (47 not catalogued) and 51
helper option words (6 not catalogued), 0 problems.

### The audit's tables

The tables in `scripts/syntax_catalogue_databricks/constants.py` hold these decisions:

- `OPERATOR_WORDS` are the lower-case connective words that an operator or predicate page writes
  as part of its syntax.
- A `hand.toml` reading's basis `section` is a syntax block under a heading the build does not
  read. The notation-block check of the omission audit finds such blocks.
- `IDENTIFIER_RULES` are the rules that list keywords usable as identifiers. They spell an
  identifier, as `IDENTIFIER` does.
- `PUNCTUATION_TOKENS` are the punctuation and operator tokens that the audit compares, each with
  every spelling the lexer gives it.
- A notation block (`EXAMPLE_HEADING`, `EXAMPLE_PROMPT`, `QUOTED_TEXT`) is a code block under a
  heading the build does not read that writes syntax notation outside its string literals: an
  optional or repeated part, a choice in braces, or an ellipsis. Blocks under an Examples heading
  and example runs (a line that opens with the `>` prompt) do not count. Each notation block needs
  a `[[block]]` row saying where its syntax is catalogued or why it is not syntax.
- `UNSEEN_TOKENS` are the lexer tokens that the walked grammar uses but whose spelling the gap
  walk does not compare: literals, quoted names and strings, code bodies, hints, subscripts,
  markers and separators. Walked rules that use them have syntax that no `[[gap]]` row can
  report. Each token needs a reviewed `[[unseen]]` row that names where the pages document it. A
  missing row, or a listed token that the walked grammar no longer uses, is a problem.
- `NAME_TOKENS` are the lexer tokens that are names or the end of input. They have no spelling of
  their own to review.
- `NAME_RULES` are the rules that spell a name. Inside another rule they spell nothing, as a
  catalogue placeholder does. Their own alternatives are still walked, so `IDENTIFIER ( ... )` is
  checked there.
- `STRUCTURE_SYMBOLS` only frame other parts. A sequence of these alone checks nothing of its own,
  so the gap walk skips it. Every other symbol, keyword or operator, is checked.
- `SYNONYM_TOKENS` are the tokens that the audit reads as another token. Each needs a reviewed
  `[[synonym]]` row in `audit.toml`. `SPELLING_SYNONYMS` are the spellings of one lexer token that
  the audit reads as the token's first spelling.

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

### Fills and contexts

`scripts/syntax_obligations_databricks/constants.py` holds the builder's fills, contexts and
class rules. The reasons for each choice follow.

- `SETUP` runs in a fresh schema of its own for every case. The referee creates the schema and
  drops it.
- Repeated fills (`DB_REPEAT_FILLS`): EXECUTE IMMEDIATE refuses two USING arguments with one
  alias, so a repeated `alias` or `var_name` takes `v2`. A list of keys names distinct keys: the
  parser refuses a repeated option or property key (`DUPLICATE_KEY`), and a constraint's column
  names must not be repeated (the constraint pages' `key_column`).
- Form fills (`FORM_FILLS`):
  - The create-view page's `column_list` is `( { column_alias [ COMMENT column_comment ] } [,
    ...] )`. A metric view's body is a YAML specification. JSON is YAML, so the fill fits on one
    line.
  - ROW FORMAT DELIMITED is compatible with a text file only (the parser refuses it with
    PARQUET), so the Hive-format table is stored as TEXTFILE. A CTAS partitions by the query's
    columns, so the query names them, and one more, since a table may not be partitioned by all
    its columns.
  - A data-source temporary view's columns are typed (SqlBaseParser `#createTempViewUsing`:
    `( colTypeList )`), though the page names them `column_list`.
  - The create-table-like page's `table_clauses` are USING, LOCATION, TBLPROPERTIES, ROW FORMAT
    and STORED AS. COMMENT is not one of them.
  - COLLATE applies to a string type (collation page), so the added column is a STRING.
  - INPUTFORMAT and OUTPUTFORMAT take class names as string literals (SqlBaseParser
    `fileFormat`).
  - DROP FEATURE takes a Delta table feature by name (the alter-table page's example: `DROP
    FEATURE deletionVectors`). SET SERDE takes a SerDe class name as a string literal (the page's
    SET SERDE examples).
  - A clone source is read at a version or timestamp of its history (sql-ref-names
    `temporal_spec`). The setup's INSERT makes version 1 of `t1`.
  - COPY INTO's option values are string literals (the page's examples: `'mergeSchema' =
    'true'`).
  - CUME_DIST's `over_clause` includes OVER. The other window forms write OVER before it.
  - ESCAPE takes a single-character string literal (the like and ilike pages).
  - A `read_*` function's option is a named argument, so its name is an identifier, not the
    string literal that an OPTIONS clause key may be (the read_files page: `format => 'file'`).
  - A control statement's operands name the label, cursor and variable that its context
    declares.
  - EXECUTE IMMEDIATE runs its string, so the string is a statement that parses.
  - A privilege and a securable object are spelled as sql-ref-privileges spells them, and
    principals take the shapes that sql-ref-principal writes.
  - LIST takes a path of a volume or an external location (the list page's examples).
  - A JSON path after the colon is a field path, as in `raw:store.bicycle` (the colonsign page).
- Runtime-only scope: `RUNTIME_SCOPE_FAMILIES` are the families whose Runtime-only forms are
  observed rather than expected. They are the core set, whose pages are reviewed for their
  applies-to banners; the other families are not reviewed yet. `RUNTIME_ONLY_CLAUSES` lists the
  forms that are one clause of a page that also applies to Databricks SQL, under the clause's
  own applies-to banner, which leaves Databricks SQL out (sql-ref-syntax-ddl-alter-table,
  Parameters). A page-level banner is read from `sources.toml` instead.
- Form contexts (`FORM_CONTEXTS`):
  - A table-valued function (its page's title, or its page's "is a table-valued function") is a
    relation. A generator function stands alone in the select list, because a generator nested
    in an expression is refused (`UNSUPPORTED_GENERATOR.NESTED_IN_EXPRESSIONS`).
  - The vector-search page's Syntax is a whole query, so it has no context.
  - A clause page's form runs in a statement that the page names as its host.
  - LIKE in a partition spec is allowed only in ALTER SHARE ADD TABLE (sql-ref-partition).
  - A securable object and a principal are the operands of GRANT (sql-ref-privileges,
    sql-ref-principal).
  - A control statement runs in a compound statement (control-flow pages): a cursor statement
    after the cursor's declaration, FETCH and CLOSE after OPEN, LEAVE and ITERATE inside the loop
    they name, and GET DIAGNOSTICS and RESIGNAL inside a condition handler.
- Family contexts (`FAMILY_CONTEXTS`): a function or expression is written as a call in
  parentheses, so that a word left at the end of a negative is not read as a column alias.
- Class rules (`DB_CLASS_RULES`): a clause, a column list or ALTER VIEW's rename is an operand
  that the linked page defines, not a name, though its placeholder contains `col` or `name`. A
  Hive-format column's type placeholders (`col_type1`, `col_type2`) name a data type, not the
  column, though they contain `col`.
- Reserved words (`TABLE_ALIAS_PLACEHOLDERS`, `ALIAS_RESERVED_WORD`): the reserved-words page
  (sql-ref-reserved-words) says Databricks formally disallows no literal as an identifier, but
  ANTI and fifteen other words must be back-ticked to serve as a table alias. So a table-alias
  operand takes ANTI as its reserved word (refused unquoted, accepted in backticks), and every
  other identifier operand takes the keyword SELECT, accepted both ways.

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
expects the same text. Such a text stays expected: a Hive-format line with neither STORED AS nor
ROW FORMAT is CREATE TABLE [USING] (the hiveformat page's note). Positives are observed because
the pinned engine is a Databricks SQL warehouse. A Runtime-only clause inside a form that
applies to Databricks SQL (GLOBAL TEMPORARY views, ROW FORMAT in CREATE TABLE LIKE) leaves the
form's reading and is kept as observed `[[limit]]` rows that quote the banner.

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

### The referee's tables

`scripts/databricks_referee/constants.py` holds the referee's reviewed tables. The reasons for
their entries follow.

Connection and timing:

- `DATABRICKS_REFEREE_CASE_SECONDS` (`ENV_CASE_SECONDS`) is optional. It sets a case deadline
  other than `CASE_SECONDS`, for a retry of cases that ran to the deadline.
- `WAREHOUSES_PATH` is the warehouse, which a check before each case reads to prove that the
  token still works. `HISTORY_PATH` is the warehouse's query history, which names the statements
  that ran in a time window.
- `TOKEN_REJECTED_CODES` and `TOKEN_REJECTED` are the HTTP codes and body that the workspace
  rejects an expired or revoked token with. A rejected token is replaced once and the request
  sent again, since the workspace did nothing with it. The token's age is measured on the wall
  clock, because a monotonic clock stands still while the machine is paused.
- A request is retried after a 429 or 5xx reply (`HTTP_RETRY_STATUS`, `HTTP_SERVER_ERROR`) and
  after a transport failure. A statement is not idempotent, so one whose reply never arrived may
  have run and is never submitted again (`REPLY_LOST`). An empty reply body, as a SCIM DELETE
  sends, reads as an empty object.
- `current_version()` is asked at most once a minute (`VERSION_SECONDS`) and once more after the
  last case. A run whose answers differ writes no receipt, so each receipt's release is the one
  that answered it.
- A case schema's `DROP ... IF EXISTS CASCADE` is asked up to `DROP_ATTEMPTS` times. The drop is
  idempotent, so a retry changes nothing but a transient failure's outcome. The schema is dropped
  even when the reply to its creation was lost.

Reading a refusal:

- `PARSER_MARK` is the excerpt that Spark's ParseException appends, in its older and newer
  header. Only an error that the parser or the parse-tree builder raised carries it.
- `SERVICE_ERROR_CLASS` reads a Unity Catalog service error, whose class follows a per-request
  id, which receipts mask.
- `CASE_SCHEMA` is the per-case schema that a message quotes. Its run stem and case number
  differ between runs, so it is masked.
- `ENVIRONMENT_MASKS` are the names of the account and its workspace that a message may quote:
  the account id, the metastore, the referee's principal and the workspace's storage bucket.
  Receipts are public, so each is masked. None is part of the statement that the case sent. A
  receipt's message is the first line of the engine's message, with URLs, addresses, request
  ids, these names and the per-case schema masked.
- `PARSER_CLASSES` lists each class the parser raises, by its full class and subclass. Any other
  parser class stops the run, so a new one is reviewed before it is counted. Beside the parser's
  own classes, it lists:
  - classes by which the parse-tree builder refuses a statement that the grammar admits;
  - `INVALID_TABLESAMPLE_FRACTION` and `TEMP_VIEW_IF_NOT_EXISTS_NOT_ALLOWED`: Databricks' parser
    refuses a sample fraction outside [0, 100] and IF NOT EXISTS on a temporary view. Both
    classes are the warehouse's own (Spark 4.2.0 has neither), and their receipts carry the
    parser's excerpt;
  - classes by which the parse-tree builder checks a utility statement's operands and refuses
    them with the excerpt: SET PATH's schema names, SET QUERY_TAGS' form, SYNC's matching table
    names, and DESC ... AS JSON on one column;
  - classes by which the parser refuses a statement's form or a literal's text while building the
    plan: a SET, an empty IN list, a REFRESH path, a SHOW FUNCTIONS scope, a variable with
    neither type nor default, ANALYZE ... DELTA STATISTICS FOR, DROP INDEX, a staging path, a
    typed literal's value or type, and an AUTO CDC sequence clause without its storage clause.
- `EXCERPT_UNSUPPORTED_CLASSES` are classes that the parse-tree builder raises, with the excerpt,
  for a feature the workspace has not enabled. The text parsed, so the refusal is
  `unsupported_feature`, never `syntax_error`.
- `PARSER_MESSAGE_CODE` is the engine code that a reviewed parser message without a SQLSTATE is
  recorded with, so that the receipt's `syntax_error` carries a code that `dialect.toml`'s
  `syntax_codes` lists.
- `LATER_PHASES` gives the reviewed phase of each class raised after parsing, by its full class
  and subclass. None is `syntax_error`: only the parser decides syntax. Its rows fall into these
  groups:
  - analysis or the catalog refuses a name, a missing object, the state of the setup's objects,
    a type, or a use of a function in a non-core case (`semantic_error`);
  - Unity Catalog refuses a value, principal or object after the statement parsed, including a
    storage location given without a URI scheme (`semantic_error`);
  - in account- and catalog-level cases, the object a statement names is missing, already exists,
    is not empty, or has a name or value that Unity Catalog refuses (`semantic_error`);
  - utility, session and scripting statements are refused after parsing, for the objects,
    columns, names or values they reference (`semantic_error`);
  - the warehouse refuses what the statement asks for, not how it is written; this covers a
    function the workspace refuses for the object it reads or for Unity Catalog, a function whose
    type the workspace has not enabled, RESOLVE DBFS LOCATION (which only a federated Hive
    metastore catalog supports), and an operation on the object the statement names
    (`unsupported_feature`);
  - a value is refused when the parsed statement runs: a function's input, a setting's value, a
    transaction statement with no transaction open, a path refused because the public DBFS root
    is disabled, a result struct with repeated field names (refused when the result is converted
    to Arrow), and ANALYZE ... COMPUTE STORAGE METRICS, which reads the storage inventory when it
    runs (`runtime_error`).
- `LATER_WITHOUT_SQLSTATE` lists the later classes that the warehouse sends without a SQLSTATE.
  Each is reviewed, and its receipt carries no code. Any other class without a SQLSTATE, and
  every parser class, still stops the run.
- `UNCLASSED_PARSER` and `UNCLASSED_LATER` review a message that names no error class by the
  message itself. A parser message (one that carries the excerpt) is a `syntax_error` by the rule
  above; a later one has its reviewed phase. Among them:
  - `Expected keyword 'FOREIGN'.` is the parser's own message, with the excerpt, for REFRESH
    CATALOG without FOREIGN;
  - a message that is only `org.example.In` or `org.example.Out` is a Hive table's INPUTFORMAT or
    OUTPUTFORMAT class, which the cases supply and the warehouse cannot load;
  - `java.net.URISyntaxException` comes from a LOCAL directory's path, which the warehouse
    resolves when it writes (`file:` with no path);
  - in non-core cases, analysis refuses an AUTO CDC flow's clauses, a securable or a share name;
  - the referee's principal may not read the metastore's policies, and is not a metastore admin,
    which changing a metastore policy needs; both refusals are authorization after parsing;
  - a recipient for open sharing is refused because this metastore has not enabled it;
  - a group or user named as a member that does not exist, a grantee that does not exist, a
    sharing identifier that names no metastore, and a securable of a type the workspace does not
    have, named in a grant or a policy, are refused once the statement parsed;
  - `None.get` is a statement with an optional name left out, which fails inside the warehouse
    after parsing.
- A run may end with these referee failures, each leaving its case undecided with no receipt: a
  case statement cancelled at the deadline (`CANCELLED_AT_DEADLINE`), a statement whose reply was
  lost, and a failed setup. The run summary lists why each case is undecided; a setup failure
  adds the error class its message carries. One of the case's own setup statements failed, so
  the case's statement never ran, and a setup that fails on every run is seen rather than left
  undecided unnoticed. Any other referee failure stops the run.

Scope:

- The referee isolates a case only inside its own schema. A case whose text changes an account-
  or catalog-level object (a catalog, a schema of the catalog, a share, a connection, a group, a
  governed tag, a metastore policy, a grant on one of them), or moves files outside a table, is
  held (`ACCOUNT_SCOPE`): the run stops before any statement runs. Each pattern is searched in
  the whole text, string literals included, so a script block or EXECUTE IMMEDIATE cannot hide
  one.
- `CASE_SCOPE_HEADS` are the statement words a case may begin with. Each runs inside the case
  schema unless a pattern above holds it. A case beginning with another word is held, since the
  referee cannot place it. A case beginning with a label, a literal, a parenthesis or an
  identifier the parser refuses is kept, because the patterns above are searched in its whole
  text. `CASE_HEAD` reads a case's first word, or a label (`l1 :`) before it; `STATEMENT_WORDS`
  are the words that start a statement, and a case starting with any other word starts with a
  name.

The sequential phase:

- Each kind in `SNAPSHOT_KINDS` is listed page by page (`paging`). Each object of the reply's
  `items` list is keyed by its `name` field and fingerprinted by `fields`, where `a.b` is the
  sorted `b` of each item of list `a`, and any other list is sorted. Volatile fields are left
  out. An object whose `skip` field has the given value, or whose name starts with
  `skip_prefix`, belongs to another kind or to the referee; the referee's own case schemas are
  not part of the state. `variant` names the field that selects the object's reviewed drop. The
  credentials list has no purpose filter, so a storage credential is skipped from the service
  credentials and read as its own kind.
- A Unity Catalog or tag-policy list returns `next_page_token` until its last page. A SCIM list
  is read `SCIM_PAGE` objects at a time from `startIndex` until `totalResults`.
- `FIXTURE_NAMES` are the names the cases give the objects they create, lowercased as Unity
  Catalog stores them: a fixture name, its backquoted `<name> Q` spelling, and the reserved word
  `SELECT`. Only an object a case added under one of these names is dropped; any other added
  object stops the run. A run also stops before any case when the account already holds objects
  with fixture names.
- `DROPS` gives the reviewed drop of each kind (and, for `workspace.default`'s tables, each
  `table_type`), in the order cleanup runs them, dependents first; `{name}` is the backquoted
  name. A kind or table type with no row has no reviewed drop, and an object of it that a case
  added stops the run.
- Metastore policies are the one kind the snapshot cannot read. After every case that names a
  policy on the metastore (`METASTORE_POLICY_CASE`), and once more after the last case, the
  sequential phase drops each fixture-named metastore policy (`METASTORE_POLICY_NAMES`; the
  grammar has no IF EXISTS here). An answer that no such policy exists, or that the referee is
  not a metastore admin (so it could not have created one), leaves the state as it was
  (`METASTORE_POLICY_ABSENT`). No ABAC policy can be attached to the metastore, so an answer
  saying so also means none could have been created. A drop that succeeds, or any other answer,
  stops the run.
- A sequential run's journal holds the baseline, then each case as it starts (`pending`, with its
  schema and the fixture names its statement writes, as backquoted identifiers or bare words)
  and once it is settled. Each step is journaled, and flushed to disk, before it runs. A settled
  line holds what the case left behind (the account state equal to the baseline again) and the
  case's answer: an observation, or a failure no reviewed row maps.
- When a run is stopped or freezes part way, `recover` reads the journal and drops only what the
  unsettled case may have created: its schema, and objects it added that its statement names
  under a fixture name with a reviewed drop. Anything else that differs from the baseline is
  reported and left alone.
- `resume` continues a stopped run from its journal. It refuses when the referee source (the
  digest of the referee's own Python files), the cases, their order or the seed differ, when the
  run finished, or when the account state differs from the baseline once the unsettled case is
  recovered. A check of the warehouse before each case that fails stops the run before that case
  starts, so `resume` continues from it.
- `refinish` writes the receipts of a finished sequential run whose account state ended as its
  baseline, with its failures judged again by the current reviewed rows. A failure that a row now
  decides becomes an observation of its recorded statement, found in the warehouse's history: the
  one failed statement between the previous case's settling and this case's whose error message
  is the recorded one. Only one case runs at a time, so its window holds its own statements
  alone. `refinish` refuses when the cases, their order or the seed differ, or any case is
  unsettled. The run's answers are reused as recorded; only their phase decision is the current
  source's. A failure in a journal written before failures kept their statement has an empty
  statement id.

Comparing runs:

- `PHASE_ONLY_CASES` lists the cases a repeat compares by phase alone, each with the reason: the
  warehouse gives two answers. Every other case is compared by phase, code, parse-only code and
  capture.

## Checking

`dialect.toml` names `local_parser = "databricks"`, so the checker gives each case the strict
verdict of `crates/sqlquack-syntax-databricks`'s `parse_strict`:

```sh
cargo build -p sqlquack-syntax-coverage --bin syntax-coverage
target/debug/syntax-coverage validate --root coverage/syntax-databricks
target/debug/syntax-coverage summary --root coverage/syntax-databricks
```

`validate` prints `valid, 1230 forms, 9622 receipts (0 superseded or withdrawn)`. A form
verifies only when every required obligation is met and a current family review covers its
fingerprint. The auditor's family reviews in `reviews/auditor-<family>.toml` cover 909 forms, so
`summary` prints `checked 1230 forms, 0 problems` and `909 of 1219 forms verified (74.57%)`, with
obligations 11666/14572. The 11 forms without obligations are not counted.

- 966 forms meet every obligation. The reviews approve 909 of them and withhold 57 until their
  cases are fixed; the scripting family has no form ready for review.
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
