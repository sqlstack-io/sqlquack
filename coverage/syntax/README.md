# Snowflake syntax catalogue

The denominator of SQLQuack's syntax coverage: every documented Snowflake syntax form, one
record per form, with a stable id. Syntax coverage, semantic analysis and translation equivalence
are separate claims; hazards and domains (`coverage/hazards.toml`, `coverage/domains/`) stay the
translation-equivalence measure.

## Files

- `indexes.toml`: the documentation indexes (`llms.txt`) the page list was read from, by sha256.
- `sources.toml`: one record per pinned documentation page: id, URL, retrieval time (UTC), sha256
  of the fetched Markdown and of its `sqlsyntax` blocks, family, status and the extraction
  decisions taken on that page. The pages themselves are not tracked.
- `catalogue/<family>.toml`: `[[form]]` records (id, family, source page, anchor heading, kind,
  normalized grammar, grammar sha256, sub-grammars used, status) and the page's `[[definition]]`
  sub-grammars (`name ::= ...`).

Families: `query`, `dml`, `ddl`, `transaction_session`, `security_admin`, `utility`, `scripting`,
`functions`, `expressions`, `classes`.

## Regenerating

```sh
python -m scripts.syntax_catalogue.fetch CACHE idx/*.txt   # private cache, never committed
python -m scripts.syntax_catalogue.build --cache CACHE --out coverage/syntax
```

A rebuild keeps every existing id whose (page, grammar) is unchanged. A form whose grammar no
longer appears keeps its id with `status = "absent_from_source"`; ids are never reused, and a
rename goes through `aliases.toml`.

## Form rules (decision D62)

A form is one documented syntax block or variant. Options and alternatives inside it are its
obligations, not forms.

1. Only fenced `sqlsyntax` blocks are read. A block is split into productions at blank lines where
   every bracket is closed, and before each `name ::=`.
2. `name ::= ...` is a sub-grammar definition. It runs to the next definition in its block.
3. On command pages a statement starts with a statement keyword (the first words of the command
   page titles, plus DESC/LS/RM). A chunk that does not start one continues the previous
   production. Blocks under Parameters, Usage notes, Examples or `... actions (`name`)` headings
   are fragments, not forms.
4. Each statement production is a form. Each variant heading under Syntax/Variant syntax is a form.
5. A statement whose tail is `{ aAction | bAction | ... }` of sub-grammars the page defines (ALTER
   TABLE) becomes one form per action.
6. A block that elides with a `...` line, under the same heading as an earlier statement,
   elaborates that statement and is merged into it.
7. Identical grammar documented under several headings (per-language ALTER FUNCTION blocks) is one
   form.
8. Functions and class methods: plain call signatures of one function name form one `call` form
   whose signatures become obligations. A syntax with its own grammar (`CAST ... AS`, `::`, OVER,
   WITHIN GROUP, DISTINCT, named arguments `=>`, keyword arguments) is a separate `special` form.
9. A variant heading with no syntax block (the DCM `DEFINE <object>` variants, some CREATE OR
   ALTER variants) is a `prose` form with an empty grammar. Its grammar is added by hand.

Variant ids come from the words that distinguish the form's required skeleton (the grammar without
`[ ]` parts and placeholders) from the page's other forms. On a collision the heading is appended,
then a number.

## Recorded ambiguity and known gaps (v1 skeleton)

- Every page where a rule above changed the form count lists that rule in `decisions`.
- `prose` forms (31): prose-only variants; the DCM `DEFINE` variants are valid only inside DCM
  project files, not as standalone SQL.
- Hub pages with no syntax block of their own (`ALTER`, `CREATE`, `CREATE OR ALTER`, `DESC`,
  `DROP`, `SHOW`, `UNDROP`, `USE`) catalogue no form; their commands have their own pages.
- `no_syntax_block` pages that still document syntax, to be catalogued by hand before the
  denominator is final:
  - Data types and literals written as tables, not syntax blocks: `data-types-numeric`,
    `data-types-text`, `data-types-logical`, `data-types-unstructured`, `data-types-user-defined`,
    and the operator pages `operators-arithmetic`, `operators-comparison`, `operators-logical`,
    plus `expressions-conditional` and `expressions-byte-bit`.
  - Function pages whose syntax is in a plain code block (`FL_*` file functions, `DMF_*`,
    `AI_COMPLETE`, `TO_FILE`/`TRY_TO_FILE`, `AS_*` and `IS_*` hub pages, and others listed with
    status `no_syntax_block`).
  - Class hub pages whose methods have their own pages.
- `out_of_scope` pages (27): views and usage pages (billing, data sharing usage, local, monitoring,
  telemetry, trust center), which document columns, not syntax. Each still needs a reviewed
  exclusion record.
- The `ddl`/`security_admin` split is by title words (users, roles, policies, integrations,
  accounts, grants, shares, ...); it is a reporting split, not part of a form's identity.
