# SQLite 3.54.1 syntax coverage root

The documented syntax of SQLite 3.54.1 (`SELECT sqlite_version()` = `3.54.1`, source id
`2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76`), the
current release on sqlite.org when this root was pinned.

## Sources

Every source is a release archive from `https://sqlite.org/2026/`, checked against its SHA-256
and the SHA3-256 sqlite.org publishes on its download page. `indexes.toml` lists them with the
grammar files the omission audit reads and the engine the referee loads.

| Archive | SHA-256 |
| --- | --- |
| `sqlite-doc-3540100.zip` | `e9f634ac28a4c7b48f5c389e4b08c039654ec1487512092b330ccfcfdc0816e0` |
| `sqlite-src-3540100.zip` | `387e7f6cad4797b17848c07d453dc6606917bb356a6cfe5a3abafd38c240c71b` |
| `sqlite-amalgamation-3540100.zip` | `0a9c8e34352dad46006aa82836d5383bff9dc6bc9ce5a60553c05aa30e6ad1f9` |

From the source archive: `src/parse.y` (`ba52f3a1…a449a`) and `tool/mkkeywordhash.c`
(`099f834d…c35cae`).

Fetch them into a private cache, each archive alone in its own empty directory, and build the
engine from the amalgamation:

```sh
python -m scripts.fetch_syntax_docs_sqlite <cache>
```

`sources.toml` lists every page of the page set with its hashes and its decision; the pages
themselves are not committed. The page set is `PAGE_GLOBS` in
`scripts/syntax_catalogue_sqlite/constants.py`: 117 of the 847 pages of the documentation
archive (81 with forms, 32 restating a form catalogued from another page, 2 with no syntax
block, 2 out of scope). The others belong to the reviewed excluded groups of
`PAGE_EXCLUSIONS` beside it (the C and Tcl interfaces, release notes, extension and
operations documentation, and so on), each with the reason its pages are not catalogued. The
build fails when a page is neither in the page set nor in an excluded group, or when a group
excludes nothing.

## Build and product scope

The pinned engine is the release amalgamation, whose parser (`parse.c`) is generated from
parse.y with no option defined. Two kinds of documented compile-time option follow from that,
and one rule settles each:

- An option that only registers functions is enabled, so that the functions the in-set pages
  document exist: `SQLITE_ENABLE_MATH_FUNCTIONS` (lang_mathfunc.html) and
  `SQLITE_ENABLE_PERCENTILE` (percentile.html, which says the extension "is disabled by default
  and must be activated at compile-time"). Neither changes the grammar. Two functions of
  lang_corefunc.html are documented as needing an option the pinned build does not define:
  `soundex()` (`SQLITE_SOUNDEX`) and `sqlite_offset()` (`SQLITE_ENABLE_OFFSET_SQL_FUNC`). The
  build follows the release's default options apart from the two above, so these two stay
  catalogued, and their calls record the engine's `no such function` as `unsupported_feature`
  (`NOFTR`): the syntax of a call is the same whichever functions are registered.
- An option that changes the grammar cannot be enabled in the amalgamation: compile.html says
  `SQLITE_ENABLE_UPDATE_DELETE_LIMIT` "must also be defined when using the Lemon parser
  generator", so it "may only be used when the library is built from source, not from the
  amalgamation". Syntax that the documentation scopes to such an option is out of scope; the
  documentation and the engine agree that the pinned build has none of it.
  - `delete-stmt-limited` and `update-stmt-limited` (ORDER BY and LIMIT on DELETE and UPDATE):
    lang_delete.html and lang_update.html document them only for builds with
    `SQLITE_ENABLE_UPDATE_DELETE_LIMIT`; parse.y has them only under that option, and the
    pinned engine refuses `LIMIT` there while parsing. Both pages are `out_of_scope` in
    `hand.toml` with that reason, their two forms are `absent_from_source`, and their four
    obligation ids are tombstoned in `retired.toml`. DELETE and UPDATE without them are
    catalogued.
  - `WITHIN GROUP` (`SQLITE_ENABLE_ORDERED_SET_AGGREGATES`, a parse.y `%ifdef`): percentile.html
    writes `percentile_cont(P) WITHIN GROUP (ORDER BY Y)` only for builds with that option and
    says that without it "only the simpler `percentile_cont(Y,P)` form is supported", which is
    catalogued. The `WITHIN` keyword row of `audit.toml` records the exclusion.
- A PRAGMA that pragma.html documents only for another build is out of scope, by the same rule:
  the pinned build does not have it. `PRAGMA_OUT_OF_SCOPE` in
  `scripts/syntax_catalogue_sqlite/constants.py` lists eight, each with its reason, and the
  build fails if one is no longer documented. `parser_trace`, `stats`, `vdbe_addoptrace`,
  `vdbe_debug`, `vdbe_listing` and `vdbe_trace` exist only with `SQLITE_DEBUG` (pragma.html
  says so for all but `stats`, which is for "testing and validation of SQLite" only;
  `tool/mkpragmatab.tcl` defines all six under `defined(SQLITE_DEBUG)`), `data_store_directory`
  only in Windows builds (`defined(_WIN32)`), and `legacy_file_format` "no longer functions" and
  is no longer defined at all. The pinned engine lists none of the eight in
  `pragma_pragma_list`; an unknown PRAGMA is silently ignored, so their accept cases said
  nothing. Their forms are `absent_from_source` and their 30 obligation ids are tombstoned in
  `retired.toml`.

Documented syntax that the pinned build should accept but refuses would be an observed-limit
form, as in the SQL Server and DuckDB roots (D337, D266): kept in the denominator, with
hand-written `[[limit]]` rows whose receipts record the refusal. At the pin one such case is
documented: create-trigger-stmt's diagram allows TEMP with a schema-qualified trigger name, which
the parser refuses from the text alone (`sqlite3BeginTrigger`). The form stays in the
denominator; its grammar is read as the engine parses it and the refused combination is a
hand-written `reject` row (see Obligations and cases), so `limits.toml` has no
`expect = "observe"` row.

## Regenerating

```sh
python -m scripts.fetch_syntax_docs_sqlite <cache>
python -m scripts.build_syntax_catalogue_sqlite --cache <cache>
python -m scripts.audit_syntax_catalogue_sqlite --cache <cache>
python -m scripts.build_syntax_obligations_sqlite --root coverage/syntax-sqlite
python -m scripts.run_sqlite_referee run --cache <cache> \
  --cases coverage/syntax-sqlite/cases/*.jsonl --out <run>.jsonl --private <dir>
```

A rebuild of an unchanged pin rewrites only the `retrieved_utc` of `sources.toml`. The
catalogue builder reuses `scripts/syntax_catalogue` and the obligation builder reuses
`scripts/syntax_obligations`, by import.

## Catalogue

351 forms, of which 341 are catalogued and 10 are `absent_from_source` (the limited DELETE and
UPDATE and the eight PRAGMAs of other builds above): query 24, dml 10 (8 catalogued), ddl 38,
transaction_session 5, security_admin 0, utility 83 (75 catalogued), functions 145,
expressions 46. Statement forms come from the syntax diagrams, read
into the catalogue notation by the `[[diagram]]` rows of `hand.toml`, each bound to the SHA-256
of its SVG; one grammar line is one form. Functions come from the documented signatures, one
form per function name; PRAGMAs from pragma.html, one form per PRAGMA. `hand.toml` also holds
the forms read by hand from tables, lists and prose, bound to the page's SHA-256, and the
reviewed status of every in-set page that yields no form.

## Omission audit (`audit.toml`)

`audit.toml` is an independent check of the catalogue against the pinned `parse.y`, the keyword
table of `mkkeywordhash.c` and the pinned engine's own lists. Every keyword sequence a walked
production spells must be covered by a catalogued form or carry a reviewed `[[gap]]` reason;
every keyword, engine function, PRAGMA, STRICT type and collation must be spelled by some form
or carry a reviewed reason. The audit fails on an unreviewed or stale row. Its limits are
written at the top of the file.

Containment is strict. Each catalogued grammar line becomes an automaton
(`scripts/syntax_catalogue_sqlite/_helpers/strict.py`) over keywords and the punctuation
parse.y names as terminals. Placeholders, identifiers, literals and quoted text spell nothing,
as identifier and literal tokens spell nothing in a parse.y sequence. A placeholder that names
a syntax diagram stands for that diagram's grammar or for nothing, as a parse.y rule too large
to spell stands for nothing. A sequence is covered only when the automaton reads it as one
contiguous run from some state, and a sequence of `cmd` only from the line's start. A longer
documented form whose keywords are not adjacent no longer covers a sequence.

An `[[unseen]]` row records syntax of a walked rule that no gap row can report, kept
uncatalogued with its reason. `UNSEEN_RULES` in `constants.py` lists the rules that need one,
and the audit fails when a listed rule has no row, when a row names an unlisted rule, or when a
listed rule is no longer walked. Seven of the ten are parse.y alternatives the engine takes
that no documented form spells: an empty CAST type (`typetoken`), a table-valued function with
no argument (`exprlist`), an alias on a parenthesised FROM list (`seltablist`), table
constraints with no comma between them (`tconscomma`), ON CONFLICT after a CHECK table
constraint (`tcons`), unquoted fallback keywords as names (`nm`), and expressions as ATTACH and
DETACH schema names (`cmd`). `tests/unit/scripts/syntax_catalogue_sqlite` checks these
guards against the committed rows.

At the pin: 134 rules (0 unwalked), 412 productions (YYNRULE of the generated parser), 1,135
keyword sequences, 80 reviewed gaps (5 derived), 10 `[[unseen]]` rows, 148 keywords (0 not
catalogued), 144 functions (2 not catalogued), 66 PRAGMAs (0 not catalogued), 9 STRICT types
and collations (0 not catalogued), 0 problems.

## Obligations and cases

`obligations/` and `cases/` are generated from the catalogue grammars by the shared builder.
`limits.toml` holds the hand-written inputs: `[[reading]]` rows that replace a grammar for
derivation only, bound to the grammar's sha256, with the reason; `[[suppress]]` rows that
withdraw one derived obligation with the reason it is not the negative or positive it looks
like (251 of the 254 cite parse.y or tokenize.c; the other 3, on SELECT forms with the select
core omitted, cite `sqlite3_prepare_v2` accepting an empty text as no statement); and `[[limit]]` rows that add a hand-written obligation
with its SQL. The 217 suppressions of a string in place of a name also cite lang_keywords.html
(:169), which documents that quirk. Where only the minimal rendering of an obligation is not a
real negative, the suppression withdraws that rendering and a `[[limit]]` row with the same
form, kind, target and note renders the obligation again by hand: `RENAME COLUMN TO c2` keeps
the optional COLUMN, `t1 LEFT t2` keeps LEFT, the reserved function name keeps its quoted accept
half and uses WHERE for the reject half, and a STRICT column without a type is an accept case.

An obligation keeps its id across rebuilds. `retired.toml` lists every id that has left the
root, grouped by the full SHA of the commit that retired it, with the reason; the builder
reserves those numbers and fails if a retired id reappears or if an id leaves `obligations/`
without a row there.

4,094 obligations and 2,780 cases (obligations/cases): query 480/254, dml 320/196, ddl 819/609,
transaction_session 38/26, security_admin 0/0, utility 884/594, functions 909/654,
expressions 644/447. At the pin every case's expectation matches the engine's verdict.

Beyond the derived obligations, `limits.toml` adds by hand:

- Boundaries, both sides of each: `?NNN` at 1 and 32,766 against `?0` and `?32767`
  (lang_expr.html, Parameters), the greatest integer literal 9223372036854775807 against the
  next, which lang_expr.html says is read as a floating point value, 500 against 501 compound
  SELECT terms, and 2,000 against 2,001 table and result columns (limits.html). The parser
  itself refuses the far side of the parameter and compound-term limits, in parse.y's actions
  (`sqlite3ExprAssignVarNumber`, `parserDoubleLinkSelect`), so those cases are `reject` with
  `GRAMR`. Both limits are the connection's `SQLITE_LIMIT_VARIABLE_NUMBER` and
  `SQLITE_LIMIT_COMPOUND_SELECT`, which default to `SQLITE_MAX_VARIABLE_NUMBER` (32766) and
  `SQLITE_MAX_COMPOUND_SELECT` (500) in the pinned amalgamation's `sqliteLimit.h`; the pinned
  build defines neither, so the boundaries hold for default builds only, as each obligation's
  note says. The parser also refuses a table of 2,001 columns, in `sqlite3AddColumn`, which
  parse.y's column action calls, so that case is `reject` with `GRAMR` too; the limit is
  `SQLITE_LIMIT_COLUMN`, whose default `SQLITE_MAX_COLUMN` (2000) is in the same `sqliteLimit.h`.
  A result set of 2,001 columns parses, and prepare refuses it later (`selectExpander`, while
  names are resolved), so that case is `accept` with the receipt's later stage.
- A frame whose ending boundary comes higher in windowfunctions.html's list than its starting
  one (Frame Boundaries): `1 FOLLOWING AND 1 PRECEDING`, `1 FOLLOWING AND CURRENT ROW` and
  `CURRENT ROW AND 1 PRECEDING` as negatives, refused in parse.y's frame action
  (`sqlite3WindowAlloc`), with `1 FOLLOWING AND 2 FOLLOWING` as the positive for the starting
  operand. The derived operand case wrote the refused order, so it is suppressed.
- Two common table expressions with one name, an observed limit: lang_with.html is silent, and
  parse.y's `wqlist` action (`sqlite3WithAdd`) refuses it. A repeated common table expression in
  a derived case takes the next name (`w2`).
- The join words in any order (lang_select.html 9.1): the hand form
  `sl.query.select.join-keywords-any-order`, with one to three words in any order, four words
  and the disallowed INNER OUTER and bare OUTER as negatives. parse.y reads the seven words as
  one token class (JOIN_KW), so the omission audit cannot see this.
- Several ON CONFLICT clauses (lang_upsert.html): the diagram's loop is read as a repetition,
  with an untargeted clause before a targeted one as the negative.
- Requirements the parser checks from the statement text alone, in build.c helpers that parse.y's
  actions call, as `reject` cases with `GRAMR`: a STRICT column with no type ("missing datatype",
  `sqlite3EndTable`) and AUTOINCREMENT on a column that is not an INTEGER PRIMARY KEY, as a
  column and as a table constraint (`sqlite3AddPrimaryKey`). The derived AUTOINCREMENT cases
  declare the column `INTEGER`, so their positives are genuine. Further text-only refusals, each
  note naming the function that raises it: a table whose only column is generated and WITHOUT
  ROWID with no primary key or with AUTOINCREMENT (`sqlite3EndTable`); TEMP with a schema other
  than `temp` on a table or view (`sqlite3StartTable`); two columns with one name
  (`sqlite3AddColumn`); a DEFAULT that is not constant (`sqlite3AddDefaultValue`); a second
  primary key (`sqlite3AddPrimaryKey`) and a generated column in it
  (`makeColumnPartOfPrimaryKey`); an expression in a PRIMARY KEY or UNIQUE constraint
  (`sqlite3CreateIndex`); a foreign key on a column the table lacks, more than one referenced
  column in a column constraint, and referenced lists of a different length
  (`sqlite3CreateForeignKey`); a column list assigned a row of another width
  (`sqlite3ExprListAppendVector`); and, through `sqlite3ResolveSelfReference` while CREATE TABLE
  is parsed, a CHECK, generated column or key that names a column the new table does not define,
  and a subquery in a CHECK or generated column.
- Two observed limits read into the grammar. A TEMP trigger takes no qualified name, `temp .`
  included, though the diagram allows one (trigger.c's `sqlite3BeginTrigger`), so CREATE
  TRIGGER is read as two branches, the optional qualified name without TEMP and TEMP or
  TEMPORARY with a plain name, with TEMP and a qualified name as the hand negative. The foreign-key clause, derived in a
  column constraint, references one column, and its continued list is a hand positive in a table
  constraint. The derived contexts write texts that parse: a TEMP table or view is qualified
  `temp .`, a generated column has a plain column beside it, a table constraint's table defines
  every column it names, and a column-name list is assigned a row of its own width.
- Lexical spellings: digit separators (`1_000`, `0xFF_FF`, `.5_5` against `1__0`, `1_`,
  `0x_1`) and the TCL parameter syntax (`$a::b(c)` against `$a(x y)`).

## Receipts

`receipts/` holds one run of the local referee against the pinned engine (receipt format
`sqlquack-syntax-receipt/2`), the run the checker reads. `runs.toml` records it and its repeat
run, kept in `repeats/`, which the checker does not read, so that each case has one current
receipt. `python -m scripts.run_sqlite_referee compare <receipts> <repeat>` compares the two,
and `check --cases ... --receipts <receipts>` checks one run against the cases; at the pin the
runs do not differ and the check finds nothing missing or stale.

The referee (`scripts/sqlite_referee`) loads the pinned `libsqlite3.so` through ctypes and
judges each case in its own child process, in a fresh in-memory database with the case's setup
applied in autocommit. A parse-only pass prepares each statement of the text in turn, unrun
(`parser_check = "whole_text"`); then every statement is prepared and stepped in turn, in the
case's capture. A text that starts with a statement a surrounding transaction changes (BEGIN,
COMMIT, END, ROLLBACK, SAVEPOINT, RELEASE, VACUUM, PRAGMA) is captured
`execute_autocommit_in_throwaway_database`: it runs in autocommit, and the database, in memory
and the case's own, is dropped with the child process. Every other case is captured
`execute_in_rolled_back_transaction`, inside a transaction that is rolled back. One budget of 30 s covers setup, the parse-only pass and
execution. A case that outlives it after every statement prepared is recorded as `TMOUT`, a
runtime error; one that outlives it earlier, or whose setup fails, stops the run.

SQLite reports nearly every prepare error as `SQLITE_ERROR` with a message, so the phase of a
prepare error is decided from the message and the primary code, by reviewed classes only, and fails closed: a message in no
reviewed class stops the run, naming the case. A prepare error is a syntax error only when the
parser itself raised it (the tokenizer, a parse.y syntax error, or a parse.y action):

| Prepare error | `engine_code` | Phase |
| --- | --- | --- |
| `near "…": syntax error` | `SYNTX` | syntax_error |
| `incomplete input` | `INCMP` | syntax_error |
| `unrecognized token: "…"` | `UNTOK` | syntax_error |
| a parse.y action's refusal (`SYNTAX_MESSAGES`: unknown table option, unknown join type, misplaced ORDER BY or LIMIT, duplicate WITH table name, unsupported frame specification, a variable number out of range, too many compound terms, too many columns on a table, a missing STRICT datatype, misplaced AUTOINCREMENT, …) | `GRAMR` | syntax_error |
| `no such function: soundex`, `sqlite_offset` or `regexp` (not in the pinned build) | `NOFTR` | unsupported_feature |
| a primary code other than `SQLITE_ERROR`, or a `RUNTIME_MESSAGES` class (limits, safety level, …) | `E` + the extended code | runtime_error |
| a `SEMANTIC_MESSAGES` class (no such table or column, arity, misused row value, …) | `E` + the extended code | semantic_error |
| any other message | none | the run stops |

An error from `sqlite3_step` is a runtime error, `E` with the extended code. The classes are in
`scripts/sqlite_referee/constants.py`, and `tests/unit/scripts/sqlite_referee` checks the
mapping, including that an unreviewed message stops the run. The receipt keeps the result
code's name, the extended code and `sqlite3_errmsg()` verbatim, so the mapping can be checked.

A refusal is a syntax error when SQLite raises it while parsing, from the statement text alone,
whether in a parse.y action or in a build.c helper the action calls; one that depends on existing
objects, such as a missing table, stays semantic. The classes in `SYNTAX_MESSAGES` are matched
on the message, so one message has one phase: `too many columns on <table>` is also what ALTER
TABLE ADD COLUMN raises on a table that already has 2,000 columns, which depends on that table;
no case creates one. Messages a build.c or expr.c helper raises both from the text and against
existing objects (`TEXT_ONLY_MESSAGES`: `N columns assigned M values`, `duplicate column name`,
`no such column`, …) are syntax errors only when the statement, prepared again alone in an empty
database, raises the identical message; otherwise they are semantic. `no such column` also
needs the statement to be CREATE TABLE with a column list, where only
`sqlite3ResolveSelfReference` resolves names, so a name resolved in a query stays semantic.

`syntax_codes` in `dialect.toml` is `["GRAMR", "INCMP", "SYNTX", "UNTOK"]`, so every syntax
receipt carries a listed code. `parser_path` names `crates/sqlquack-syntax-sqlite`, and
`local_parser = "sqlite"` gives each case the strict verdict of that crate's parser.

## Checking

The root is checked with the coverage checker, which reads `dialect.toml`:

```sh
cargo build -p sqlquack-coverage --bin syntax-coverage
target/debug/syntax-coverage validate --root coverage/syntax-sqlite
target/debug/syntax-coverage summary --root coverage/syntax-sqlite
```

`validate` prints `valid, 351 forms, 2780 receipts (0 superseded or withdrawn)`. The engine
side is complete: every case has one current receipt that agrees with its expectation.

`summary` checks every case against the SQLite parser. A form verifies only when every
required obligation is met and a current family review covers its fingerprint. It prints
`checked 351 forms, 0 problems` and `207 of 341 forms verified (60.7%)`, with obligations
2036/4094. The 10 forms without obligations are not counted.

- 210 forms meet every obligation. 207 are approved by the independent syntax auditor in
  `reviews/auditor-<family>.toml`, copied verbatim from its records on
  `wip/syntax-omission-audit` (`coverage/syntax/audits/sl-family-batch-b24e4b34.toml` and its
  addendum `sl-family-batch-b24e4b34-addendum-db419eb8.toml`), read against the strict parser
  at tree `1cc1e867`. An approval holds while the form's definition hash and fingerprint are
  unchanged. The other 3 wait for review.
- 131 forms have an opaque case. The parser turns every statement that is not a query into a
  gapped `Command` node, and gaps `INDEXED BY` and `IN table`, as its README lists. In 126 of
  them the gapped statement also accepts a negative case that SQLite refuses, which the checker
  reports as a disagreement; no other case disagrees.

`baseline.json` is the `snapshot` of these claims, bound to the commit that added the catalogue.
`make syntax-coverage`, part of `check-ci`, runs `diff --root coverage/syntax-sqlite` against it,
so a verified form or met obligation the parser stops supporting, or a catalogue change the
baseline was not re-recorded for, fails the gate. Re-record it with
`syntax-coverage snapshot --root coverage/syntax-sqlite --catalogue-commit <commit> --json`.
