# Syntax coverage

This file describes how SQLQuack measures how much documented Snowflake syntax its parser handles.
The figure that gets published is the share of catalogued documented syntax **forms** that meet
every obligation declared for them. Test counts, session counts and weights never count.
Syntax, semantic analysis and translation equivalence are separate claims and are reported as
separate numbers. Hazards in `coverage/hazards.toml` stay the measure of translation equivalence.

Everything lives under `coverage/syntax/`. The catalogue owner's build
(`scripts/syntax_catalogue/`, described in `coverage/syntax/README.md`) generates the sources and
the catalogue. The evidence owner's importer generates the receipts. The loader in
`crates/sqlquack-syntax-coverage` refuses all of the following, and names the file and record for
each:

- an unknown field;
- a malformed id;
- a mismatched hash;
- a missing family file;
- a stray file;
- an id declared twice.

To validate a tree, run `cargo run -p sqlquack-syntax-coverage -- validate [--root coverage/syntax]`.

## Families

There are ten families: `query`, `dml`, `ddl`, `transaction_session`, `security_admin`,
`utility`, `scripting`, `functions`, `expressions` and `classes`. Each family has a
`catalogue/<family>.toml`, an `obligations/<family>.toml` and a `cases/<family>.jsonl`. All three
must exist, and they may be empty.

## Sources (`sources.toml`, `indexes.toml`)

There is one `[[source]]` per documentation page:

- `id`: `doc.<page-slug>`.
- `url`: an `https` URL.
- `title`.
- `family`: empty only when `status` is `out_of_scope`.
- `retrieved_utc`: in `YYYY-MM-DDTHH:MM:SSZ` form.
- `page_sha256` and `syntax_sha256`: the hashes of the fetched page and of its syntax blocks.
- `syntax_blocks`.
- `status`: one of `forms`, `restated`, `no_syntax_block` or `out_of_scope`. A `restated` page
  repeats grammar that a form on another page already covers in full (decision D80). It has no
  form of its own and adds nothing to the denominator. At least one form must list it in
  `also_sources`.
- `decisions`: the extraction rules that changed the page's form count.

`indexes.toml` lists each documentation index the pages were read from, by `name` and `sha256`.
The pages and their prose stay private and untracked.

## Catalogue (`catalogue/<family>.toml`)

A `[[form]]` is one documented syntax block or variant (decision D62). Options and alternatives
inside it are obligations, not forms.

- `id`: `sf.<family>.<page-slug>[.<variant>…]`. No segment may read as an obligation number such
  as `o001`. Ids are never reused. A rename is recorded in the optional `aliases.toml` as
  `[[alias]] from`, `to` and `reason`.
- `family`: the file's family.
- `source` and `anchor`: a pinned source id, and the heading on its page. `source` may not be a
  `restated` page.
- `also_sources`: optional. A sorted list of distinct pinned source ids for the other pages that
  restate the same grammar byte for byte. It never contains `source`, and it never names a
  `no_syntax_block` or `out_of_scope` page. It is not part of `grammar_sha256`. The form counts
  once, under `source`, in the summary and in the page rollup.
- `kind`: one of `statement`, `call`, `special` or `prose`.
- `grammar`: the normalized productions. It is empty only for a prose form.
- `grammar_sha256`: the SHA-256 of the productions joined by `" || "`, or `prose:<heading>` for a
  prose form.
- `uses`: the names of the page definitions the form uses.
- `status`: `catalogued` or `absent_from_source`. An absent form keeps its id.

A `[[definition]]` is one of the page's `name ::= …` sub-grammars, with `id` (`<source>::<name>`),
`source`, `name` and `grammar`. An exclusion is never a catalogue field. It is a review record
that names the form and its reason.

## Obligations (`obligations/<family>.toml`)

An `[[obligation]]` has these fields:

- `id`: `<form id>.o<NNN>`, where `o000` is never used.
- `kind`: one of `alternative`, `option_present`, `option_absent`, `required_operand_missing`,
  `operand_type_class`, `positive`, `negative_neighbour`, `boundary_below`, `boundary_at`,
  `boundary_above`, `identifier_class`, `nesting_depth`, `interaction` or
  `interaction_all_options`.
  - `operand_type_class`: a valid operand of the named class parses. A wrong-class operand is a
    `negative_neighbour` whose `target` names the operand.
  - `interaction`: two options or alternatives whose combination a page's usage notes constrain.
    It must carry the `anchor` of that note.
  - `interaction_all_options`: one per form with two or more independent options. A case that
    uses every option together satisfies it. It takes no `target`, `pair` or `depths`
    (decision D82).
- `required`.
- `target`: the alternative or option id. It is required for the alternative, option, operand
  and boundary kinds.
- `depths`: required for `nesting_depth`, and only there.
- `pair`: two distinct ids, required for `interaction`, and only there.
- `anchor`: for limits that are documented only in prose.
- `note`.

Derive obligations mechanically from the grammar wherever possible: every `[ ]`, `{ | }` and
placeholder. Add prose-documented limits by hand, with their anchor. Where Snowflake documents no
limit, the obligation's `note` says `no documented bound`, and its `depths` list the depths that
were tested. Nothing claims to be exhaustive.

## Cases (`cases/<family>.jsonl`)

Each line is one JSON object. Blank lines are refused.

- `case_id`: unique across every family, with no whitespace. Receipts join on it, so a case
  recorded before it was mapped keeps its packet id.
- `obligations`: one or more obligation ids of the same family.
- `sql` and `sql_sha256`.
- `setup` and `setup_sha256`: the ordered setup statements.
- `session` and `session_sha256`: the session parameters, with JSON values.
- `capture`: one of `execute`, `compile_only_explain`, `execute_canonical_recorder`, `describe`
  or `execute_ddl_in_run_database`. The last runs DDL inside a database that the run creates and
  drops. Its setup names that database, the run nonce and the per-case schema with the
  placeholders `<run_database>`, `<run_nonce>` and `<case_schema>`, so the setup hash stays the
  same from run to run.

`setup_sha256` is the SHA-256 of the compact JSON array, which Python writes with
`json.dumps(setup, separators=(",", ":"), ensure_ascii=False)`. `session_sha256` is the same over
the object with sorted keys. Setup statements may create objects in the scratch schema. The setup
hash binds a receipt to exactly that setup.

## Receipts (`receipts/*.jsonl`)

A receipt is the normalized sidecar of one genuine capture, in the format
`sqlquack-syntax-receipt/1` that the evidence owner's importer (`scripts/syntax_receipts/`)
writes. A hand-written receipt is not evidence. Receipt files and each importer run report
(`<name>.report.json`) beside them are append-only. The evidence guard rejects any change to a
committed one.

- `format`, `case_id`, `sql_sha256`, `setup_sha256` and `session_sha256`. The hashes must equal
  the case's.
- `session_parameters`: its hash must equal `session_sha256`.
- `capture_method` and `run_id`.
- `query_id`, `query_id_sha256` or both. Canonical-recorder runs keep only the hash. A receipt
  with neither loads, but it is inadmissible: it can never satisfy an obligation.
- `warehouse_version`, and `recorded_at` in RFC 3339 UTC.
- `outcome`, `errno` and `sqlstate`.
- `phase`: one of `accepted`, `syntax_error`, `semantic_error`, `runtime_error` or
  `unsupported_feature`.
- `phase_basis`: why the importer chose the phase.
- `raw_record_sha256`, `run_audit_sha256` and `packet_sha256`: the hashes of the private raw
  record, the private run audit and the submitted packet.
- Optional provenance: `case_hash`, `input_sha256`, `statement_index`, `statements_in_input`,
  `statements_in_request`, `completed_statements`, `after_sql` and `category`.
  `completed_statements` is the number of statements in the request that completed before it
  stopped. It is provenance only and does not change the phase.
- Optional `reason_category` and `reason_category_basis`, always together, and only on a phase
  after parsing (`semantic_error`, `runtime_error` or `unsupported_feature`). The category is one
  of `missing_object`, `invalid_option_or_value`, `type` or `other`. The basis says what the
  importer read it from, such as `errno 2003`. Older receipts have neither.

- Optional `supersedes_receipt_sha256`: the SHA-256 of a published receipt line of the same case,
  hashed without its newline, that this receipt replaces.

A receipt is evidence only while it is current. A later receipt can supersede it, or the run report
beside its file can withdraw it: the report's `withdrawn` list names withdrawn receipts by
`receipt_sha256`. A retired receipt stays published but is dropped before checking. A
supersession or withdrawal that names no published receipt, or a receipt of another case, is a
format problem.

Any other field is refused. No account identifier, host, URL, user or email address may appear.
A `compile_only_explain` receipt shows only that the statement compiled. Its `accepted` phase
says nothing about runtime.

## Reviews (`reviews/*.toml`)

A `[[review]]` has these fields:

- `id`, `reviewer_lane` (`a`, `b`, `c` or `backend`) and `family`.
- A family review binds `catalogue_sha256` and `obligations_sha256`, the hashes of the reviewed
  file bytes. A page review has an empty `family` and binds only `sources_sha256`, the hash of
  `sources.toml`.
- `parser_tree`: the full Git tree id of the reviewed parser source. A family review must have it.
  A page review verifies no parsing, so it may omit the field. The checker ignores it on page
  reviews, so a parser change never makes a page review stale.
- `reviewed_utc`, plus a `verdict` of `approved` or `changes_requested`.
- `[[review.exclusion]]`: exactly one of `form` (a form id of the review's family) or `source` (a
  page id, used for out-of-scope and syntax-less pages), plus a `reason`. A page review excludes
  pages only, and a family review excludes forms only.
- `note`.

## Verification

The checker joins the catalogue, obligations, cases and receipts with freshly computed parser
results. An obligation is satisfied only when at least one case cites it and every citing case
agrees. A case agrees when all of these hold:

- It has a receipt whose SQL, setup and session hashes and capture method equal the case's, and
  that has a query id or its hash. A receipt that is not bound this way is stale.
- All of its admissible receipts record the same phase.
- The current parser's verdict matches that phase: a `syntax_error` receipt matches a local
  rejection, and every other phase matches a local acceptance.
- An acceptance is structural: no statement contains a Command or Raw fallback node.

A `semantic_error`, `runtime_error` or `unsupported_feature` receipt with a structural local
acceptance satisfies the obligation for syntax and is counted as `semantic_only`.

A form is verified only when it has at least one required obligation, every required obligation
is satisfied, and an approved review of its family matches the current catalogue and obligation
file hashes and the current parser tree. The parser tree is `git rev-parse
HEAD:crates/sqlquack-syntax`, and none at all while that path has uncommitted changes, so every
family review is stale then. A form is excluded only by an approved review that is current in the
same way. Absent forms keep their ids and are never counted. The headline denominator is the
catalogued forms: 3,348 on the frozen catalogue `e863d7de` (decision D81).

Anything missing, ambiguous, stale, mismatched or opaque fails closed with a named reason:
`no_obligations`, `missing_case`, `missing_receipt`, `stale_receipt`, `ambiguous_receipts`,
`disagreement`, `opaque_fallback`, `unreviewed` or `stale_review`. A reference that does not
resolve fails every check command. Examples are an obligation of an uncatalogued form, a case
citing an undeclared obligation, an alias whose source is not absent or whose target is unknown,
and an exclusion naming an unknown form or page.

## Reports

`cargo run -p sqlquack-syntax-coverage -- <command> [--root coverage/syntax] [--json]` prints
results on stdout and progress on stderr:

- `summary`: the verified share of catalogued forms, obligation completion, forms per reason, and
  per-family scope.
- `family <family>`: every form of one family with its state, completion and reasons. The JSON
  lists each obligation with every citing case, its receipt phase and the local verdict.
- `missing [--family <family>]`: every unsatisfied required obligation and its reason.
- `pages [--family <family>]`: the page rollup, a secondary view. A form counts under its own
  `source` only.
- `boundaries --form <id>`: the form's boundary and nesting-depth obligations, with their bounds,
  notes and cases.
- `snapshot --catalogue-commit <commit>`: every form's state and every satisfied obligation as
  JSON. It is refused unless the catalogue, `sources.toml` and `aliases.toml` in the working tree
  equal those of the commit.
- `diff BASE [HEAD]`: one snapshot against a later one, or against the current tree when HEAD is
  omitted.

## Baseline

`coverage/syntax/baseline.json` is a snapshot on the frozen catalogue. `make syntax-coverage`,
part of `make check-ci`, runs `diff` against it and fails in two cases. The first is a coverage
regression: a form the baseline records as verified or excluded that no longer is, or a satisfied
obligation that no longer is. The second is a catalogue change: a changed sources, aliases or
catalogue file, or a form added or removed. Catalogue changes are reported separately from
coverage changes. Gains pass. After a reviewed gain or catalogue change, re-record the baseline
with `snapshot` and commit it with the change. A family review binds the parser tree, so any
parser commit makes verified forms stale until their reviews are renewed. That shows up as a
regression.
