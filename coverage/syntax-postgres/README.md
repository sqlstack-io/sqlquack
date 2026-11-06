# PostgreSQL syntax catalogue

The denominator of SQLQuack's PostgreSQL syntax coverage: every documented PostgreSQL 18.6 syntax
form, one record per form, with a stable id `pg.<family>.<slug>[.<variant>]`. The layout and the
record formats are those of `coverage/syntax` (see `docs/syntax-coverage.md`); this file records
what differs.

## Source

The pinned source is the release tarball `postgresql-18.6.tar.bz2` (sha256 in `indexes.toml`),
not the rendered website. Its DocBook SGML is what postgresql.org/docs/18 is built from, so a
source id `doc.<docbook-id>` names the page `https://www.postgresql.org/docs/18/<docbook-id>.html`.
`indexes.toml` also binds the files the page list is read from (`reference.sgml`,
`ref/allfiles.sgml`, `filelist.sgml`, `postgres.sgml`) and the grammar the omission audit reads
(`src/backend/parser/gram.y`, `src/include/parser/kwlist.h`).

A page is either

- a SQL command reference page (a `refentry` of the SQL Commands part, 183 pages), or
- a `sect1` (or a chapter's introduction) of the SQL-language chapters: syntax, ddl, dml,
  queries, datatype, json, array, rowtypes, rangetypes, func, typeconv, indices, textsearch, mvcc,
  perform and parallel.

`page_sha256` is the sha256 of that SGML text; `syntax_sha256` of its synopsis blocks and
signatures.

## Regenerating

```sh
python -m scripts.fetch_syntax_docs_pg CACHE      # private cache, never committed
python -m scripts.build_syntax_catalogue_pg --cache CACHE --out coverage/syntax-postgres
```

The build writes nothing while any input is unresolved: a synopsis block with no disposition, a
disposition naming no block, a restatement of an uncatalogued form, or a hand form whose page
changed. PostgreSQL 19 is an in-place rebuild against its tarball; ids follow the same keeping
rules as the Snowflake catalogue (`scripts/syntax_catalogue/_helpers/forms.py`, reused by import).

## Omission audit (`audit.toml`)

```sh
python -m scripts.audit_syntax_catalogue_pg --cache CACHE
```

An independent check against the server's own grammar, `gram.y` and `kwlist.h` of the pinned
release. It reads every rule of gram.y and spells each alternative of every rule reachable from
`toplevel_stmt`, `stmt` and the expression rules (720 of 733 rules, with no depth bound) as keyword
sequences,
expanding nonterminals that are keyword sets (`DROP object_type_any_name` checks DROP TABLE,
DROP SEQUENCE and so on). A sequence is covered when some catalogued form, with its page's
definitions inlined, contains its keywords in order; for a statement the form's leading required
keywords must also appear in the sequence, so DROP FOREIGN TABLE does not cover DROP TABLE.
Every kwlist.h keyword that a production uses must appear in some grammar.

`audit.toml` binds the sha256 of gram.y and kwlist.h and gives a reviewed reason for every
production that no form covers (`[[production]]`, keyed by rule and production text) and every
keyword that no grammar names (`[[keyword]]`). The audit fails on an unreviewed gap, a review
naming no current gap, or a changed grammar file. The reasons are of three kinds: syntax gram.y
accepts that the 18.6 documentation does not document (the spelling ANALYSE, GLOBAL and LOCAL
before TEMPORARY outside CREATE TABLE, `BETWEEN ASYMMETRIC`, `TREAT`, `NCHAR`); productions that
only raise an error (CREATE ASSERTION, `ALTER TYPE ... DROP VALUE`, UNENCRYPTED PASSWORD); and
documented values of a placeholder (`NFC`, `OUT`, `SETOF`). Single-keyword alternatives below a
statement are checked by the keyword rule only.

`[[unwalked]]` lists each of the 13 rules the walk does not reach with its reason: the start
symbol and top-level wrappers, the PL/pgSQL-only parse modes and the five keyword category lists.
The audit fails on an unwalked rule that is not listed. `limits` states what the method does not
check, such as identifiers, punctuation and adjacency of keywords.

## Form rules

1. **Command pages.** The `<refsynopsisdiv>` synopsis is split into statements at each unindented
   line that starts with the command's first word (SELECT also `WITH` and `TABLE`). Each
   statement is a form. `<phrase>where X is:</phrase>` (and `and X is`, `X in ... is`) starts a
   sub-grammar definition, `[[definition]]` `doc.<page>::<X>`; `can be one of` / `is one of`
   lists its alternatives one per line.
2. **Actions.** A statement whose grammar takes `<action> [, ... ]`, where the page lists
   `action` as alternatives (ALTER TABLE and six other ALTER pages), becomes one form per action, with the
   action substituted into the statement.
3. **Other synopsis blocks** (on command pages outside the Synopsis, and in the chapters) each
   have a reviewed disposition in `hand.toml` `[[block]]`, keyed by page and block sha256:
   `form` (each production a form; on a command page, older syntax still accepted, named
   `<page>.<label>`), `calls` (signatures grouped as functions), `definition`, `restated` (the
   page joins the listed forms' `also_sources`), or `fragment` / `hand` with a reason.
4. **Functions and operators.** Every `func_signature` of func.sgml is read. Plain call
   signatures of one function name on one page form one `call` form, `pg.functions.<page>.<name>`;
   a signature with its own keyword grammar (`substring(... FROM ... FOR ...)`, `WITHIN GROUP`,
   `ORDER BY` inside the call, the SQL/JSON constructors, keyword functions written without
   parentheses) is a `special` form per name and keyword skeleton. Operator signatures form one
   `special` form per operator per page in `expressions`, `pg.expressions.<page>.op-<chars>` or
   the keywords of a keyword operator. Table sections that catalogue no form are listed in
   `hand.toml` `[[signatures]]` with the reason.
5. **Data types.** Each row of the Data Types, Special Character Types, Object Identifier Types
   and Pseudo-Types tables is a form in `expressions`; aliases are alternatives.
6. **Hand forms** (`hand.toml` `[[form]]`): lexical syntax and expressions written in prose or
   examples (identifiers, string and bit-string constants, comments, constructors, named
   notation, array types, interval fields, range constructors, ILIKE), the comparison operators
   whose table uses the generic `datatype`, SET SCHEMA and SET NAMES, the SQL function bodies
   (`RETURN`, `BEGIN ATOMIC`) of CREATE FUNCTION and CREATE PROCEDURE, and the deprecated CREATE
   ROLE clauses.

Families: `query`, `dml`, `ddl`, `transaction_session`, `security_admin`, `utility`, `functions`,
`expressions`. There is no `classes` or `scripting` family: PL/pgSQL bodies are string constants
to the server grammar, and DO and CREATE FUNCTION are catalogued as statements.

## Obligations and cases

```sh
python -m scripts.build_syntax_obligations_pg [--root ROOT] [--focus focus/postgres.toml]
```

Obligations are derived for every catalogued form by the shared builder
(`scripts/syntax_obligations`, reused by import). `scripts/syntax_obligations_pg` adds what is
Postgres's own: the submission context of each family (statements as written, expressions inside
`SELECT ( … )` so a trailing keyword is never read as a bare column label, data types inside
`CAST ( NULL AS … )`), placeholder fills by synopsis name, class rules for operator and function
placeholders that name a value of a type, and a grammar read that writes a form's bare words in
upper case (Postgres synopses spell keywords and type names in lower case; every placeholder is
in angle brackets), with a few list spellings rewritten so the renderer can copy them
(`GRAMMAR_REWRITES` in its constants: ONLY and `*` as alternatives, as gram.y relation_expr takes
them; LIMIT and FETCH as one clause; DISTINCT with a required select list; a with_query as
name AS ( query ); a list's second item written out; a window definition optional inside
OVER). The grammar is tokenized with Postgres's operators: a run of operator characters
(`&&`, `<->`, `?-|`, …) is one token, and `\[ \]` in a reading is a literal bracket (array
constructors, subscripts and array types) rather than an optional part. `limits.toml` holds the
readings, suppressions and hand-written limits, each with its reason, in the format of
`coverage/syntax/limits.toml`. Most expressions suppressions are one gram.y fact: every
operator token is also a prefix operator (`qual_Op a_expr`), so an operator form with its left
operand left out is valid syntax; with its right operand left out it stays a negative.

Cases are written for the forms in `focus/postgres.toml` and for every form of the families in
`CASED_FAMILIES`; the other families' case files hold only their focus forms until taken up.
`pending.toml` lists cases recorded but held out of the receipts because the referee's method
misjudges them, each with the referee change it waits on. Each case has the referee's shape: the session
`{"TimeZone": "UTC", "statement_timeout": "30s"}`, capture `execute_in_rolled_back_transaction`,
or `execute_autocommit_in_throwaway_database` for statements Postgres refuses inside a
transaction block (the referee's `engine.toml` `autocommit` list), no setup, and one statement
without a terminating semicolon. `elisions/` is not written: the Snowflake elision rules do not
apply, and the few Postgres `...` fills are style entries.

Receipts (`receipts/*.jsonl`, format `sqlquack-syntax-receipt/2`) are recorded by the Postgres
referee (`engines/postgres`) against the pinned 18.6 image; the raw records and run audits stay
private. The case build reads them as the shared builder does: an accepted case takes its
receipt's later stage, and an observed case its receipt's outcome.
