# Snowflake syntax catalogue

The denominator of SQLQuack's syntax coverage: every documented Snowflake syntax form, one
record per form, with a stable id. Syntax coverage, semantic analysis and translation equivalence
are separate claims; hazards and domains (`coverage/hazards.toml`, `coverage/domains/`) stay the
translation-equivalence measure.

## Files

- `indexes.toml`: the documentation indexes (`llms.txt` files and the docs navigation tree
  `reference.json`) the page list was read from, by sha256.
- `sources.toml`: one record per pinned documentation page: id, URL, retrieval time (UTC), sha256
  of the fetched page and of its syntax blocks, family, status and the extraction decisions taken
  on that page. The pages themselves are not tracked.
- `catalogue/<family>.toml`: `[[form]]` records (id, family, source page, other pages restating
  it (`also_sources`), anchor heading, kind,
  normalized grammar, grammar sha256, sub-grammars used, status) and the page's `[[definition]]`
  sub-grammars (`name ::= ...`).
- `hand.toml`: forms catalogued by hand from pages whose syntax is not in a syntax block (data
  types, operator tables, literals, identifiers, variables, and pages whose syntax is only in the
  HTML). Each is bound to the page's hash; the build refuses a hand form whose page has changed.
- `aliases.toml`: old form ids that were split, merged or folded, with the forms that now carry
  their grammar. The old id stays catalogued as `absent_from_source` and is never reused. An id
  removed as a fragment (not a form) has no alias.
- `reviews/exclusions.toml`: the reviewed reason for every page that catalogues no form
  (`out_of_scope` and `no_syntax_block`), bound to the sha256 of `sources.toml`.
- `census/<family>.jsonl`: for each counted case that `sqlquack convert` refuses only
  with `unresolved name`, the smallest schema that resolves its names, one line per case sorted
  by case id: `{"case_id", "relations": [{"name", "columns": [{"name", "type"}]}]}`. Names are
  spelled as the case SQL spells them (unquoted, or with their double or single quotes); types
  are Snowflake type names. Cases that no schema resolves (a column with no relation in scope,
  `CONNECT BY PRIOR`) have no line. The checker does not read this directory.

Families: `query`, `dml`, `ddl`, `transaction_session`, `security_admin`, `utility`, `scripting`,
`functions`, `expressions`, `classes`.

## Regenerating

```sh
python -m scripts.fetch_syntax_docs_snowflake CACHE CACHE/idx/*      # private cache, never committed
python -m scripts.build_syntax_catalogue_snowflake --cache CACHE --out coverage/syntax
```

Pages with no Markdown export (the per-category command hubs) are fetched as HTML and catalogue
no form. A rebuild keeps every existing id whose (page, grammar) is unchanged. A form whose
grammar changed keeps its id when its natural id is unchanged or when all its earlier signatures
are still among its signatures. A form whose grammar no longer appears keeps its id with
`status = "absent_from_source"`; ids are never reused, and splits and renames go through
`aliases.toml`.

## Form rules

A form is one documented syntax block or variant. Options and alternatives inside it are its
obligations, not forms.

1. Fenced `sqlsyntax` blocks, and `sql` blocks titled "Syntax", are read. A block is split into
   productions where every bracket is closed: at blank lines, before each `name ::=`, and before
   a line that repeats the current statement's first two keywords.
2. `name ::= ...` is a sub-grammar definition. It runs to the next definition in its block. A
   definition under its own heading inside the Syntax section (DECLARE's variable and cursor
   declarations) is also a form.
3. On command pages a statement starts with a statement keyword (the first words of the command
   page titles, plus DESC/LS/RM). A chunk that does not start one continues the previous
   production. Blocks under Parameters, Usage notes, Examples, Access control or
   `... actions (`name`)` headings are fragments, not forms. On construct pages only blocks under
   a Syntax heading (or under the title) are forms. List items, lone labels such as `Aggregate:`
   and keyword-order illustrations (`SELECT * EXCLUDE ... RENAME ...`) are fragments.
4. Each statement production is a form, including each of several statements in one block or in
   several blocks under the same heading (JOIN, GROUP BY ALL, INSERT ALL/FIRST). Each variant
   heading under Syntax/Variant syntax is a form.
5. A statement whose tail is `{ aAction | bAction | ... }` of `*Action` sub-grammars the page
   defines (ALTER TABLE) becomes one form per action.
6. Identical grammar documented under several headings (per-language ALTER FUNCTION blocks) is one
   form.
7. Functions and class methods: plain call signatures of one function name form one `call` form
   whose signatures become obligations; a block listing several functions, or `{ NAME1 | NAME2 }`,
   gives one form per name. A syntax with its own grammar (`CAST ... AS`, `::`, OVER, WITHIN GROUP,
   named arguments `=>`, keyword arguments) is a separate `special` form per signature; `DISTINCT`
   and `ALL` inside the parentheses are options of the plain call.
8. A variant heading with no syntax block (the DCM `DEFINE <object>` variants, some CREATE OR
   ALTER variants) is a `prose` form with an empty grammar. Its grammar is a `[[reading]]` in
   `limits.toml` derived from the page's substitution rule: CREATE OR ALTER X takes
   the CREATE X grammar with OR ALTER, DEFINE X takes the CREATE OR ALTER X grammar with DEFINE in
   its place, and each reading quotes the page. DEFINE cases are captured from a DCM project
   definition file (`capture = "dcm_definition_file"`), never by executing them. DCM refuses
   unqualified schema-level names before it reports parse errors (errno 1540), so DEFINE cases
   name the defined object, and the schema-level objects it references, as `d1.s1.<name>`
   (`d1.<name>` for schemas and database roles); account-level names stay unqualified. References
   after AFTER, FINALIZE, ROOTS, FINALIZER, CORTEX SEARCH SERVICE, REFRESH USING ( DELETE FROM and
   the secret objects in SECRETS = ( ... ) are qualified the same way. Other ddl, security_admin
   and classes cases are executed in a run database (`capture = "execute_ddl_in_run_database"`), and each carries the
   exact setup its receipts were recorded under, so the checker binds receipts strictly:
   the setup of the case's latest receipt from an imported or reconciled run, skipping
   measure-only receipts and shared-schema setups (whose outcome depends on run order). A case
   whose receipts disagree on syntax_error across setups is listed for review. A case without
   such a receipt carries its family's sweep setup with a per-case schema (the ddl sweep for
   ddl, the run-database sweep for security_admin and classes), which future captures use. Cases of
   other families on a held list (`--held`, the import's held-case file) run later under the
   run-database runner and carry its setup too, unless they already have such a receipt, whose setup
   they then carry like a ddl case.
9. Pages whose syntax is written outside syntax blocks are catalogued from `hand.toml`; the
   page's `decisions` records the basis (HTML parsed-literal, table, heading, prose, inline code or
   examples, the last only for syntax shown by examples alone, such as `<expr>['<key>']`).
10. Syntax restated on another page counts once. Grammar repeated byte for byte on
   several pages is one form, on the page with the shortest id (the command's own page before its
   variant pages, `create-iceberg-table` before the per-catalog pages); grammars that differ at all
   are separate forms. General-reference pages that restate forms catalogued elsewhere
   (constraints, collation, virtual columns, geospatial, translators, format models, interval
   formats) are folded by the `[[fold]]` entries in `hand.toml`. Each restating page is listed in
   the canonical form's `also_sources`; a page whose every form was folded has status `restated`.
   A folded id stays `absent_from_source` with an alias to the canonical form.

Variant ids come from the words that distinguish the form's required skeleton (the grammar without
`[ ]` parts and placeholders) from the page's other forms, or from a hand form's label. On a
collision the heading is appended, then a number.

## Recorded ambiguity and known gaps

- Every page where a rule above changed the form count lists that rule in `decisions`.
- `prose` forms (30, after CREATE OR ALTER WAREHOUSE was catalogued from its syntax block):
  prose-only variants, each read from its page's substitution rule (rule 8). DEFINE STAGE needs
  obligation numbers above 999, which the checker accepts as `o1000` onwards, unpadded. The DCM
  `DEFINE` variants are valid only inside DCM project files, not as standalone SQL.
- An unquoted identifier in place of a string literal is an observed neighbour, not a scored
  rejection: Snowflake parses an identifier there and fails it later (OAUTH_ALLOWED_SCOPES,
  IMPORTS, FROM LISTING), so a receipt decides it. An observed neighbour still settles as a
  rejection when its receipt is a syntax error, unless Snowflake refuses every documented positive
  of its form: then the form's receipts say nothing about its negatives, which all stay observed.
- A rejection that only empties a list (`= ( )`) is observed, not scored: Snowflake parses
  `ENCRYPTION = ( )` and accepts `BLOCKED_NETWORK_RULE_LIST = ( )`, and pages document empty
  lists such as `BLOCKED_OBJECT_TYPES_FOR_CREATION = ()`.
- `classes` methods are called as `CALL <instance>!<METHOD>( ... )`, as the SQL classes page
  says, except DESCRIBE and LIST, whose pages call them only with SELECT. Reference, object and
  array arguments are filled from each page's examples (`SYSTEM$REFERENCE( ... )`,
  `OBJECT_CONSTRUCT( ... )`). Method and constructor arguments are expressions, as function
  arguments are: a wrong-class whole argument is withdrawn by class and an omitted one is an arity
  case, as in `functions`.
- Data types: each documented type heading is one form whose synonyms are alternatives
  (`{ DECIMAL | DEC | NUMERIC }`); a heading that names several types with different semantics
  (TIMESTAMP_LTZ/NTZ/TZ) is one form with each spelling an alternative.
- The `ddl`/`security_admin` split is by title words (users, roles, policies, integrations,
  accounts, grants, shares, ...); it is a reporting split, not part of a form's identity.
