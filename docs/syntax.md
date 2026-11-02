# Syntax foundation: tokenizer, parser and tree

Status: P1 Part A. Steps 1 to 5 are done: the tree keeps every Snowflake call as written, the
Snowflake print-and-reparse fidelity check is retired, and polyglot is a dependency of the
development-only check crate alone.

SQLQuack owns its whole stack. This note fixes how the tokenizer, parser and syntax tree get into
SQLQuack, how we prove they behave exactly like the code they come from, and the order in which
they then become ours. Snowflake and DuckDB are the first analysis dialects.

## Requirements

1. **Lossless tokens.** Every byte of the input belongs to exactly one token, including whitespace
   and comments. Each token has a byte range, so the tokens joined back equal the input.
   sqlbuild's formatter and lint rules build on this.
2. **Two parse modes.**
   - *Strict*, used for conversion: any error, or any significant token the tree does not cover,
     refuses the statement.
   - *Tolerant*, used for formatting and linting unfinished SQL: always returns the tokens, parses
     what it can statement by statement, keeps unparsed ranges as such, and never panics.
3. **A faithful tree.** Functions stay as written, a name plus arguments, with no cross-dialect
   canonicalisation at parse time. Meaning comes from SQLQuack's typing and rules.
4. **Speed and memory** at least as good as polyglot's, within sqlbuild's CI budgets.

## Crate layout

| Crate | Contents | Depends on polyglot |
|---|---|---|
| `sqlquack-syntax` | the tokenizer (significant tokens and trivia), the parser (strict and tolerant), the tree, the Snowflake and DuckDB dialect settings | never |
| `sqlquack-syntax-derive` | the proc macro that generates child traversal for tree nodes, copied from `polyglot-sql-ast-derive` | never |
| `sqlquack-polyglot-check` | dev-only tool and tests: the exact comparison with polyglot, corpus runs and parse timings; `publish = false`, a fensu tooling crate | yes, through the workspace pin |
| `sqlquack-core` | parsing entry points re-pointed at `sqlquack-syntax`, plus classification, refusals and provenance as today | removed in step 5 |
| `sqlquack-duckdb` | building blocks, the DuckDB printer and DuckDB's own check of the printed SQL (step 4) | never |

Inside `sqlquack-syntax` the copied files are split into fensu's domain layout, by
`scripts/fensu_split_modules.py` following the placements in `scripts/fensu/constants.py`: `tokens`
(the tokenizer, its errors and the `Dialect` and `CallMode` settings), `lossless`, `modes`, `guard`,
`parser` (the parser, split by statement family under `_helpers/`, and the dialect settings it
parses with), `functions` (polyglot's function registry), `traversal` and `expressions` (the tree,
see below). The split moves items without editing them and re-points every path;
`scripts/fensu_prune_imports.py` then drops the imports no build needs.

The tree's node types are about 12,000 lines, more than fensu's 2,000-line limit for one file, so
`scripts/fensu_split_models.py` splits them into one `expressions` subdomain per area, following
`MODEL_PLACEMENTS`: `tree` (the `Expression` node), `names`, `queries`, `operators`, `functions`,
`typed_functions`, `types`, `tables`, `columns`, `schema_objects` and `statements`. Each area's
`main/` holds a function the parser calls that builds or reshapes that area's nodes.

No runtime crate depends on `sqlquack-polyglot-check`. `make polyglot-guard`, part of `check-ci`,
reads Cargo's resolved graph and fails if a workspace crate outside an allowed set reaches polyglot
through normal (not dev or build) dependencies. Since step 5 the allowed set
(`scripts/polyglot_guard/constants.py`) is the check crate alone, so the `sqlquack` binary contains
no polyglot at all.

## Where the code comes from

The tokenizer, parser and tree are copied from **sqlstack-io/polyglot `7d5fe2c`**, the commit
SQLQuack's polyglot pin moved to first, so the exact check compares like with like. It includes the
fork's typed-marker lowering (`__polyglot_typed`, inert when the names are absent) and upstream
0.13.4's parser fixes.

The copy is exactly the closure polyglot compiles with only `dialect-snowflake`, `dialect-duckdb`
and `stacker` enabled: `tokens.rs`, `parser.rs`, `expressions.rs`, `error.rs`, `guard.rs`,
`function_registry.rs`, `traversal.rs`, `ast_children.rs`, and `dialects/` with `mod.rs`,
`snowflake.rs`, `duckdb.rs` and `generic.rs`; plus the `AstNode` derive. The generator,
transpiler, optimizer, lineage, validation, bindings and other dialects' modules stay behind.
`lib.rs` is SQLQuack's own and exposes only Snowflake and DuckDB.

The copied files are byte-for-byte polyglot's except for these edits:

- the derive crate's name;
- `mod` declarations of dialect modules that were not copied (they are never compiled, and
  rustfmt resolves every declaration);
- two inner `cfg_attr` attributes on features this crate never enables, resolved to the
  `allow(dead_code)` they always meant here;
- in-file tests that need the generator or transpiler, or name another dialect, removed.

Polyglot's other feature gates stay as written and are never enabled. `parser.rs` still holds
branches for other dialects; they stay, unreachable, until the strip step deletes them under the
exact check. Until the strip and split steps the copied modules allowed polyglot's lint level and
dead code on their declarations in `lib.rs`, and fensu skipped both copied crates; since the split
fensu checks them like the rest of the workspace.

**Licensing.** `THIRD_PARTY_NOTICES` holds polyglot's MIT text and copyright line next to
sqlglot's, and says that parts of the tokenizer and parser are derived from polyglot. There are no
per-file or per-test headers. The import commit message names the copied files and the polyglot
commit.

## Tree: copy nearly as-is, then make it ours (decided)

There are two options.

- **A. Copy the tree nearly as-is first, then change it in checked steps (recommended).** The
  copied `Expression` keeps polyglot's variants, fields and serde shape, so the exact check is a
  plain equality of the two serialised trees. Today's rules already work on the serialised tree
  (`serde_json::Value` dispatched by node kind), so step 3 swaps the parser underneath them without
  rewriting them.
  - *Cost:* for a while we carry polyglot's warts: canonicalised function nodes, comment wrappers,
    uneven field names and about 600 variants. Each later step removes some of them under the
    check.
- **B. Define our own tree now and convert polyglot's into it in one place.** The tree is clean
  from day one. But the exact check then also tests a converter of about 600 variants, written
  before we know which shapes the rules need, and all the rules must be rewritten in step 3.

With A, every change after the copy is a small, checked diff. With B, the first diff is large and
the check is weaker. A is also the route the plan already lays out: copy, strip, split, make
faithful.

**After the copy, the tree diverges on purpose in this order:**

1. Spans move from Unicode character offsets (polyglot's) to byte offsets. The check compares
   spans after converting units.
2. Functions become faithful calls (step 5, below). The check compares through an explicit,
   reviewed list of expected differences, one entry per spelling.
3. Further cleanup (field names, comment wrappers) follows the same pattern.

**Two mechanisms retired with the faithful tree (step 5):**
- the six proven parse-fidelity equivalences (RLIKE, `:` paths, POSITION, MOD, TEXT, FROM VALUES),
  which existed only because polyglot canonicalises. Their recorded pairs stay as evidence
  (`SPELLING_PAIRS`), with no rewrite: a rule that converts both spellings cites the pair;
- the Snowflake print-and-reparse fidelity check, which needed polyglot's generator.

Their job passes to strict mode's token-coverage check and the rules.

## Tokens

The copied tokenizer keeps producing the significant tokens the parser consumes, unchanged. A thin
layer around it produces the lossless stream: it walks the input and emits trivia tokens
(whitespace, line comments, block comments) for every gap between significant tokens.

- Each token carries its kind and its byte range; its text is a slice of the input.
- Polyglot's string-literal text is processed (escapes resolved); the slice keeps the original.
- The invariant "tokens joined back equal the input, byte for byte" is checked on every corpus
  and by fuzzing (`fuzz/`, a cargo-fuzz target outside the workspace). CI's `fuzz` job runs the
  tolerant-mode target on nightly for 90 seconds, seeded with the exact check's statements, and
  keeps any failing input as an artifact.
- Gap text that is neither whitespace nor a comment, or overlapping tokens, is an error.

**Parse modes.** `parse_strict` refuses on any parse error or any byte the lossless tokens do not
cover. Once the tree carries byte spans, it will also check that they cover every significant
token exactly once. `parse_tolerant` always returns the tokens, splits statements at top-level
semicolons, parses each on its own, and keeps the ranges it cannot parse; it never panics.

## The exact check against polyglot

`sqlquack-polyglot-check` parses each statement with both polyglot `7d5fe2c` and
`sqlquack-syntax`, in the same dialect, and compares:

1. whether each succeeded;
2. for errors: the kind, the message and the position (the error's full debug form);
3. for trees: the serde serialisations, which must be equal.

It also asserts the lossless-token invariant, and times both parsers on the same inputs
(`--timing-rounds`: medians over whole-corpus passes, alternating which parser goes first). It
runs as an integration test on the committed corpora, as a CI job on DuckShop, and as a command
for local corpora (`--counts-only` prints no SQL, trees or file names).

During steps 2 to 4 the bar is zero differences. From step 5 the check runs both of SQLQuack's
call modes: the lowered mode (`CallMode::Lowered`, polyglot's lowering, kept for the check only)
must still give zero differences from polyglot, and the faithful mode must differ from the lowered
one only as `corpora/polyglot-check/expected_differences.toml` allows:

- the faithful parse records each kept call's source span (`CallMode::FaithfulWithSpans`);
- top-down, each kept call that the lowered mode parses differently from its own source text is
  replaced by that lowered parse, and the pair (written name, polyglot's node) is recorded;
  calls the lowered mode also keeps are descended into;
- the result must equal the lowered tree exactly, and every recorded pair must be listed;
- the only other accepted difference is comment placement: a kept call holds its trailing comment
  itself, where polyglot's caller places a lowered node's comment, sometimes twice. The trees must
  be equal without comments, and the faithful tree must keep every distinct comment (the lossless
  tokens hold each comment exactly once).

Inside a lowered call the check compares the whole call through polyglot's own parse of its text,
not argument by argument; arguments are parsed by the same code in both modes.

SQLQuack's pipeline parses in a third mode, `CallMode::FaithfulLocated`: the faithful parse that
also records each node's source range, so refusals and runtime raises can say where they come
from. Ranges live in `span` fields of type `SourceSpan` on the node payloads that carry one
(operators, casts, CASE, predicates, aliases, subqueries, queries, joins and CTEs, besides the
names and calls that had spans already); the parser fills them at the expression levels, select
items, table expressions and statements. A `SourceSpan` is left out of serialised trees when
absent and is ignored by equality, and a located parse never changes the spans the parser keeps
on names, so the located tree equals the faithful one. The check proves it on every corpus: each
entry's located parse, without `span` fields, must equal its faithful parse.

SQLQuack also fixes valid Snowflake that polyglot rejects. Each fix is a `[[fix]]` entry in the
same file: an id, a marker the statement contains (whitespace removed) and polyglot's errors on it.
The check counts a statement as fixed, not different, only when SQLQuack parses it, polyglot's
error is one listed, and the marker appears; `crates/sqlquack-syntax/tests/fixes.rs` pins the tree
each fix produces. The first fix: `v[3]:a`, a key path on an array element.

The opposite case, a statement SQLQuack rejects and polyglot parses, is a `[[rejection]]` entry:
an id, the exact statement, its dialects and text of SQLQuack's error. The check counts it as
rejected, not different, only for that statement in a listed dialect with that error. The only
rejection: `__polyglot_typed(amount, 'NOT A TYPE (')`, a generated typed-expression marker whose
type text is malformed, errors instead of becoming an ordinary call.

The faithful parse also keeps a quoted variant path key that holds `.`, `[`, `]`, `"` or `:` in
brackets (`v:a."x.y"` is the path `a["x.y"]`), where polyglot joins it with dots (`a.x.y`), which
reads as two keys. The check compares variant paths written flat, so this bracketing is the only
difference it allows there; `crates/sqlquack-syntax/tests/faithful.rs` pins the paths.

## Corpora

| Corpus | Dialect | Where it runs |
|---|---|---|
| SQLQuack's cases (source SQL and converted DuckDB) | Snowflake, DuckDB | CI, `cargo test` |
| sqlglot v30.14.0's identity, parser and dialect fixtures: every Snowflake and DuckDB statement, generic ones in both (`corpora/polyglot-check/sqlglot.json`) | Snowflake, DuckDB | CI, `cargo test` |
| The SQL inputs of polyglot's own tests, whatever they were written for, in both dialects (`corpora/polyglot-check/polyglot_tests.json`) | Snowflake, DuckDB | CI, `cargo test` |
| sqlglot v30.14.0's PostgreSQL dialect tests (`corpora/polyglot-check/sqlglot_postgres.json`) | PostgreSQL | CI, `cargo test` |
| sqlglot v30.14.0's SQLite dialect tests (`corpora/polyglot-check/sqlglot_sqlite.json`) | SQLite | CI, `cargo test` |
| sqllogictest's evidence statements and a seeded sample of the rest (`corpora/polyglot-check/sqllogictest.json`, MIT) | SQLite | CI, `cargo test` |
| All 5,719,877 distinct sqllogictest statements SQLite runs | SQLite | locally only; counts in the change notes |
| PostgreSQL 17.2's regression SQL (`src/test/regress/sql`), split into statements (`corpora/polyglot-check/postgres_regress.json`, PostgreSQL License) | PostgreSQL | CI, `cargo test` |
| DuckShop (`benchmarks/duckshop` in sqlbuild), compiled with `sqb compile`: models, audits, tests and functions | Snowflake | CI job `polyglot-check`, at a pinned sqlbuild commit and release |
| DuckShop 10K (`benchmarks/duckshop-10k`, generated, then compiled) | Snowflake | the same job |
| The production project, compiled on a local copy | Snowflake | locally only; aggregate counts only, never committed |

## Tests ported from polyglot

The rule: port tests that pin syntax, never tests that pin meaning. Snowflake, DuckDB and generic
SQL only.

**Ported as tests** (in-crate, so they reach the copied internals; `src/ported/` for polyglot's
integration tests, through a small `compat` module standing in for polyglot's root API):
- the unit tests in the copied files, Snowflake, DuckDB and generic only;
- `error_handling`, `deep_nesting_regression`, `snowflake_regression_test`, `data_type_api`,
  `typed_expression`, `numeric_exponent_boundaries`, `duckdb_json_arrow_context` and
  `duckdb_unreserved_relation_aliases`, the tests among them that need no generator;
- the fuzz target `fuzz_parser`, which became the tolerant-mode fuzzer.

**Ported as corpus input.** Round-trip tests assert that `generate(parse(x))` equals `y`, and
SQLQuack prints no Snowflake. Every SQL string in the tests that need the generator, in
`identity_roundtrip`, `prefix_not_regression`, `set_operation_subquery_operand_regression`,
`create_view_security_comment_regression` and `is_predicate_precedence`, and in the sqlglot
fixtures `sqlglot_identity`, `sqlglot_dialect_identity` and `sqlglot_parser` read, is instead an
exact-check corpus entry: SQLQuack must parse it exactly as polyglot does.

**Leave behind:**
- every transpile test (`sqlglot_transpile*`, `sqlglot_compat` transpile cases,
  `tpch_transpile_stack`, `transform_regression`, `dialect_matrix`), since they pin polyglot's
  guesses;
- pretty-printing (`sqlglot_pretty`), which is sqlbuild's;
- analysis expectations (`query_analysis*`, `qualify_*`, `analyze_failures`,
  `engine_type_truth`), though their SQL can be reused as corpus input;
- every other dialect's tests;
- `fuzz_transpile` and `fuzz_roundtrip`.

**Our fork's typed-marker tests** that need only the parser came with the copy
(`typed_expression`); those that need polyglot's analysis are rewritten against SQLQuack's tree
when the analysis API (CHI-852) needs them.

## Steps and their checks

1. **This note.** Done.
2. **Copy.** Check: zero differences from polyglot on every corpus; lossless tokens on every
   corpus. Done.
3. **Port the rules and building blocks** to `sqlquack-syntax`. Check: identical conversions;
   every recorded case passes. Done: every parse goes through `sqlquack-syntax`; the rules already
   read the serialised tree, which is unchanged. Printing still goes through polyglot's generator
   in one place, `sqlquack-core`'s `printing` domain, which hands polyglot the identically
   serialised tree: the Snowflake print-and-reparse fidelity check, cast type names in
   classification, and the unused `check_duckdb_print`. The review export of every case, census
   and verify are identical before and after.
4. **DuckDB printer.** Rules build DuckDB output through the printer instead of polyglot. DuckDB
   checks every output with `json_serialize_sql` (parse) and `PREPARE`/`EXPLAIN` (bind),
   replacing the polyglot re-parse. Polyglot then leaves the runtime dependency tree.
   - Done: the rules' building blocks are SQLQuack's DuckDB printer. `sqlquack-duckdb`'s
     `check_printed` has DuckDB parse and bind every converted statement (`prepare`) over empty
     tables for the relations it reads, created from the schema; semi-structured columns are held
     as `JSON` (decision 3). A lexical pass, skipping strings, quoted names and comments, requires
     exactly one statement and finds the names that may be relations or project functions; DuckDB's
     `json_serialize_sql` did this before and cost more than the bind on large statements. Each
     thread reuses one in-memory engine, dropping the objects the previous check created before
     defining its own, so a check sees only its relations; opening an engine per check dominated
     the census once its completion pass bound thousands of statements. The engine is never
     closed, since a thread-local engine closing during process exit killed processes
     intermittently. The re-parse of emitted SQL and the unused `check_duckdb_print` are gone.
   - Done: cast type names in classification are SQLQuack's own (`cast_type_name`); the exact
     check proves them equal to polyglot's printed base names on every cast in every corpus,
     and on every Snowflake type spelling in each cast form (`snowflake_types.json`).
   - The Snowflake print-and-reparse fidelity check, the last runtime use of polyglot, stayed
     until the tree was faithful, so no rule was added on a tree that still canonicalised; it
     retired in step 5.

5. **Faithful tree**, before Part B.
   - Done: every Snowflake call keeps its written name and arguments. polyglot's typed path runs
     first; when it fails, or keeps the call under its own name (unit arguments of `DATEADD` and
     `DATE_PART` still become `var`), its result stands. Only when it lowers the call into another
     node, and the call parsed as written covers exactly the same tokens, does the written call
     replace it. Calls written in keyword syntax (`EXTRACT(x FROM y)`, `POSITION(a IN b)`,
     `TRIM(BOTH ...)`) stay typed. Results are cached per call, so nested lowered calls parse a
     bounded number of times. DuckDB is unchanged. 144 spellings are listed, all spelling-only.
   - Done: the pipeline parses faithfully and the rules convert the parsed tree, not the
     canonical one. A call reaches a rule only through its spelling's entry in
     `CALL_SPELLINGS` (`crates/sqlquack-rules/src/conversion/constants.rs`), and every entry is
     a spelling recorded cases use. Spellings polyglot merged into a shared node without cases of
     their own (`LEN`, `CHAR_LENGTH`, `CHARACTER_LENGTH`, `UCASE`, `LCASE`, `IF`, `IIF`,
     `STARTS_WITH`, `ENDS_WITH`, `DATE_DIFF`, `TIMEDIFF`, `TIMESTAMPDIFF`, `YEAR`, `QUARTER`,
     `MONTH`, `DAY`, `HOUR`, `MINUTE`, `SECOND`) were refused until they had cases. Of these,
     the ten Snowflake documents (`LEN`, `TIMEDIFF`, `TIMESTAMPDIFF` and the seven part functions)
     convert again through recorded spelling pairs with `LENGTH`, `DATEDIFF` and `DATE_PART`; the
     other nine are refused as not in Snowflake's function reference (`NOT_SNOWFLAKE_FUNCTIONS`). The `RLIKE`
     operator converts through the `REGEXP_LIKE` rule, proven by the recorded pairs in
     `cases/fidelity/rlike.toml`. No rule reads a node only an equivalence produces, so the
     equivalences no longer shape conversion.
   - Done: the fidelity check retired with `sqlquack-core`'s `printing`, `canonical`, `reparse`
     and `comparison` domains, and `sqlquack-core` no longer depends on polyglot. The six
     equivalences remain as recorded spelling pairs (`SPELLING_PAIRS` in
     `crates/sqlquack-rules/src/conversion/constants.rs`): `sqlquack verify` still requires both
     spellings of every pair to record one result, and a rule that converts both spellings of a
     pair records it in its provenance (`RLIKE`, and `TEXT` and `STRING` cast targets). The census
     loses its print-back section; files the check refused are refused by other rules.
     `make polyglot-guard` allows the check crate only.

After step 5, under the exact check: strip other dialects and split into fensu-clean modules.
