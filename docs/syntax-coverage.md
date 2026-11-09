# Syntax coverage

This file describes how SQLQuack measures how much documented Snowflake syntax its parser handles.
The figure that gets published is the share of catalogued documented syntax **forms** that meet
every obligation declared for them. Test counts, session counts and weights never count.
Syntax, semantic analysis and translation equivalence are separate claims and are reported as
separate numbers. Hazards in `coverage/hazards.toml` stay the measure of translation equivalence.

Everything lives under `coverage/syntax/`. The catalogue owner's build
(`scripts/syntax_catalogue/`, described in `coverage/syntax/README.md`) generates the sources and
the catalogue. The evidence owner's importer generates the receipts. The loader in
`tools/sqlquack-coverage-check` refuses all of the following, and names the file and record for
each:

- an unknown field;
- a malformed id;
- a mismatched hash;
- a missing family file;
- a stray file;
- an id declared twice.

To validate a tree, run `cargo run -p sqlquack-coverage-check -- validate [--root coverage/syntax]`.

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
  repeats grammar that a form on another page already covers in full. It has no
  form of its own and adds nothing to the denominator. At least one form must list it in
  `also_sources`.
- `decisions`: the extraction rules that changed the page's form count.

`indexes.toml` lists each documentation index the pages were read from, by `name` and `sha256`.
The pages and their prose stay private and untracked.

## Catalogue (`catalogue/<family>.toml`)

A `[[form]]` is one documented syntax block or variant. Options and alternatives
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

- `id`: `<form id>.o<NNN>`, where `o000` is never used. Numbers past `o999` take more digits
  without a leading zero (`o1000`), so each number has one spelling; ids keep their meaning and are
  listed by number, `o999` before `o1000`.
- `kind`: one of `alternative`, `option_present`, `option_absent`, `required_operand_missing`,
  `operand_type_class`, `positive`, `negative_neighbour`, `boundary_below`, `boundary_at`,
  `boundary_above`, `identifier_class`, `nesting_depth`, `interaction`,
  `interaction_all_options` or `extreme`.
  - `operand_type_class`: a valid operand of the named class parses. A wrong-class operand is a
    `negative_neighbour` whose `target` names the operand.
  - `interaction`: two options or alternatives whose combination a page's usage notes constrain.
    It must carry the `anchor` of that note.
  - `interaction_all_options`: one per form with two or more independent options. A case that
    uses every option together satisfies it. It takes no `target`, `pair` or `depths`.
  - `extreme`: a size or nesting value with no documented bound, such as 100 nested blocks or
    5,000 list items. It takes no `target`, `pair` or `depths`, is required, and has
    a `note`. Its case takes its expectation from Snowflake's receipt.
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

`limits.toml` holds the hand-added `[[limit]]` and `[[suppress]]` rows the obligation builder
reads, and the `[[reading]]` rows: how an ambiguous page grammar is read, with `form`,
`grammar_sha256` (the form's grammar hash, or `prose:<heading>` for a prose form), `grammar` (its
lines, none blank; empty for a lexical form the `reason` alone explains) and `reason`. The checker requires the
file, refuses any other table, a reading of a form that is not catalogued, and a form read twice,
and hashes each form's reading into its definition (below).

## Cases (`cases/<family>.jsonl`)

Each line is one JSON object. Blank lines are refused.

- `case_id`: unique across every family, with no whitespace. Receipts join on it, so a case
  recorded before it was mapped keeps its packet id.
- `obligations`: one or more obligation ids of the same family.
- `sql` and `sql_sha256`.
- `setup` and `setup_sha256`: the ordered setup statements.
- `session` and `session_sha256`: the session parameters, with JSON values.
- `capture`: one of `execute`, `compile_only_explain`, `execute_canonical_recorder`, `describe`,
  `execute_ddl_in_run_database` or `dcm_definition_file`. `execute_ddl_in_run_database` runs DDL
  inside a database that the run creates and drops. Its setup names that database, the run nonce
  and the per-case schema with the placeholders `<run_database>`, `<run_nonce>` and
  `<case_schema>`, so the setup hash stays the same from run to run. `dcm_definition_file` writes
  the case SQL as one definition file and plans it with `EXECUTE DCM PROJECT … PLAN` in a project
  the run owns and discloses. A receipt counts for a case only when its `capture_method` is the
  case's `capture`, so a standalone execute receipt never satisfies a DCM case.

- `expect`: optional, `accept` or `reject`. `reject` means a syntax error. `accept`
  means no syntax error: the statement was accepted, or refused only after parsing. A receipt whose
  phase contradicts it makes the case `unexpected_outcome`, so it is no evidence.
- `expect` is `observe` on an extreme case Snowflake has not answered yet; the case fails as
  `awaiting_observation` whatever its receipts. It is `terminates` on an extreme case Snowflake
  refused with a syntax error: any clean local verdict, an acceptance or a rejection, agrees, and
  only a local parse failure fails it. No receipt phase contradicts `terminates`.
- `stage`: optional, only with `accept`. One of `semantic`, `runtime` or `error`: where the
  accepted statement is expected to fail later. It is informational, and the syntax score never
  compares it.

`setup_sha256` is the SHA-256 of the compact JSON array, which Python writes with
`json.dumps(setup, separators=(",", ":"), ensure_ascii=False)`. `session_sha256` is the same over
the object with sorted keys. Setup statements may create objects in the scratch schema. The setup
hash binds a receipt to exactly that setup.

## Receipts (`receipts/*.jsonl`)

A receipt is the normalized sidecar of one genuine capture, in the format
`sqlquack-syntax-receipt/1` that the evidence owner's importer (`scripts/syntax_receipts/`)
writes. A hand-written receipt is not evidence. Receipt files and each importer run report
(`<name>.report.json`) beside them are append-only. Recorded cases and receipts live in the
evidence repository under `syntax/<dialect>/`, and `make evidence-pin-check` rejects a pin change
that edits or removes a committed one.

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
- Optional `account_state`: the account parameters the capture ran under, by name, each as
  `{ "value": …, "basis": … }` with both non-empty, such as
  `{ "CORTEX_MODELS_ALLOWLIST": { "value": "NONE", "basis": "…" } }`. It
  is provenance only and does not change the phase; when present it names at least one parameter.
- Optional `reason_category` and `reason_category_basis`, always together, and only on a phase
  after parsing (`semantic_error`, `runtime_error` or `unsupported_feature`). The category is one
  of `missing_object`, `invalid_option_or_value`, `type` or `other`. The basis says what the
  importer read it from, such as `errno 2003`. Older receipts have neither.

- DCM fields, only on a `dcm_definition_file` receipt: `dcm_project`, the disclosed project, and
  `definition_file_sha256`, which equals `sql_sha256`, are required. A failed PLAN keeps its
  whole text in `message` and the inner error, the text after its `… in file …:` line, in
  `inner_message`, which ends `message`. `errno` and `sqlstate` are the code the warehouse
  returned, which is the inner error's. `definition_line` and `definition_position` are the `line <n> at position <m>` the
  inner error states, or, when it states none, absent or those the whole message states.
  `inner_errno` is present only when the inner text itself carries a number. A changeset
  (`accepted`) has no inner error. The phase comes from the inner error: `syntax error` is
  `syntax_error`, any other failure after parsing is `semantic_error`. DCM's own pre-check
  (`DCM Project compilation failed in file …`) is no evidence of a parse: its receipt carries a
  `measure`, such as `dcm_precheck`, so it never counts as syntax evidence.
- Optional `message`: the warehouse's whole error message.
- Optional `connector_errno`: an error number the connector raised in the client after the server
  ran the statement, as query history shows. It is provenance only and does not change the phase.
- Optional `measure`: the measure other than syntax the receipt was recorded for, such as
  `missing_object_with_setup`. Such a receipt is never syntax evidence.
- Optional `supersedes_receipt_sha256`: the SHA-256 of a published receipt line of the same case,
  hashed without its newline, that this receipt replaces.

### Reconciled runs

The importer reads a run only when its audit has one `complete` event whose `raw_sha256` is the
hash of `capture/raw.jsonl`. A DDL run that stopped before writing that event can instead carry a
`reconciliation.json` in the format `sqlquack-run-reconciliation/1`. The importer
then reports the run as `reconciled` instead of `imported`, with the file's hash as
`reconciliation_sha256`, and its report counts `reconciled_receipts`. It refuses the whole run
when:

- the audit also has a `complete` event, or the run is not a DDL run;
- the file is not a JSON object;
- `format`, `status`, `run_id`, `audit_sha256`, `packet_sha256` or `recording_session_id` do not
  bind this run's audit;
- `failure` is empty;
- `observed_cleanup` lacks a `drop_query_id`, its `drop_status` is not `SUCCESS`, or `dropped` is
  not the start event's run database;
- `inventory_check.counts_equal` is not `true`, or `run_owned_names_remaining` is not empty;
- `raw_records.file_sha256` is not the hash of `capture/raw.jsonl`, or `raw_records.records`
  are not counted by `count`, indexed 0, 1, 2 and so on, and listed once each by `id` with a
  `sha256`.

Each raw line must be listed under its case `id`, with `sha256` equal to the line's hash without
its newline (the receipt's `raw_record_sha256`). A line that is not JSON, an unlisted line, a line
whose hash differs and a listed record with no line are refused one by one.

The record may hold one `unrecorded_submission`, from query history, for a case the run submitted
without writing a raw record. It becomes a receipt only when:

- its `source` is `query_history` and it carries a `query_id`;
- both `sql_sha256` and `query_text_sha256` equal the hash of the case SQL, and the case has no
  raw record;
- it carries no `sqlstate`, since query history has none (the receipt's is null);
- it carries the query's own `release_version`, which becomes the receipt's warehouse version;
- its `execution_status` is `SUCCESS` with no `errno`, or `FAILED_WITH_ERROR` with an integer
  `errno` and an `error_message`;
- its `start_time` is readable.

Its `raw_record_sha256` is the hash of the entry as canonical JSON.

A receipt is evidence only while it is current. A later receipt can supersede it, or a run report
in the receipts directory can withdraw it: the report's `withdrawn` list names each withdrawn
receipt by `receipt_sha256`, with the `reason`. Receipts files and reports are append-only in every
dialect, so a receipt with no newer recording is retired by adding a report that withdraws it, never
by deleting or editing a file. A retired receipt stays published but is dropped before checking. A
supersession or withdrawal that names no published receipt, or a receipt of another case, is a
format problem. So is a published receipt superseded more than once, and a supersession cycle.

Any other field is refused. No account identifier, host, URL, user or email address may appear.
A `compile_only_explain` receipt shows only that the statement compiled. Its `accepted` phase
says nothing about runtime.

## Unobservable obligations (`unobservable/<family>.toml`)

An `[[unobservable]]` entry names an obligation whose construct Snowflake answers only with an
internal error (603), so no capture can establish a phase. It has `obligation`,
`form`, `construct`, `reason`, `cases` (each `{ case_id, sql_sha256 }`), `runs` and `probes` (each
`{ probe, sql_sha256, result }`). The obligation must be declared and belong to `form` in that
family; each listed case must exist with that `sql_sha256` and cite the obligation; an obligation is
listed at most once. Such an obligation stays in the denominator, never counts as satisfied, and is
reported as `unverifiable_internal_error` ("unverifiable: Snowflake internal error"). Probe results
are diagnostics, never syntax evidence.

An entry may set `cause = "not_submitted"` when its cases were never sent to Snowflake, such as
side-effecting calls a capture guard holds. Such an entry has empty `runs` and `probes`, a
`reason` naming the rule that holds the cases, and at least one case, checked as above. The
obligation stays in the denominator, never counts as satisfied, and is reported as
`unverifiable_not_submitted`. An absent `cause` means a 603 internal error; any other value is
refused. A `[[diagnostic]]` in the same file has the same fields,
with `captured` in place of `cases`: a 603 record on a construct no obligation requires any more,
whose retired cases are not resolved against the cases file. Diagnostics are format-checked and
listed, never scored. The checker ignores `semantic-limits/`.

## Marks (`marks/<family>.jsonl`)

An optional file per family in which the catalogue records how it settled a case: one JSON object
per line with `case_id`, `sql_sha256`, `obligations` and `mark`. `mark` is `documented` for a
case that follows its documentation against Snowflake's syntax error, or `settled accept` or
`settled reject`. Each mark must name a case with that `sql_sha256` and those `obligations`,
and a case is marked at most once; any other value is refused. The checker uses the documented
marks in `diff` only.

## Reviews (`reviews/*.toml`)

A `[[review]]` has these fields:

- `id` and `family`.
- A family review binds `catalogue_sha256` and `obligations_sha256`, the hashes of the reviewed
  file bytes, unless it binds each listed form's `definitions` instead (below). A page review has an empty `family` and binds only `sources_sha256`, the hash of
  `sources.toml`.
- `parser_tree`: the full Git tree id of the reviewed parser source. A family review must have it.
  A page review verifies no parsing, so it may omit the field. The checker ignores it on page
  reviews, so a parser change never makes a page review stale.
- `reviewed_utc`, plus a `verdict` of `approved` or `changes_requested`.
- `forms` (optional, family reviews only): the form ids of the family this review verifies. A
  review with `forms` marks only the listed forms as reviewed, still bound to the catalogue and
  obligation hashes and the parser tree; forms it does not list stay `unreviewed`. A review without
  `forms` covers the whole family. An empty list, a form of another family or an uncatalogued form
  is an error. Two current reviews may list the same form; a review id declared twice is an error.
- `fingerprints` (optional, with `forms` only): each listed form's fingerprint when it was reviewed,
  as `{ "<form id>" = "<sha256>" }`, one for every listed form. While a form's recorded fingerprint
  equals its current one, the review stays current for that form whatever the parser tree; when it
  differs, the review is stale for that form only. A review without `fingerprints` stays bound to
  `parser_tree`.
- `definitions` (optional, with `forms` and `fingerprints` only): each listed form's definition
  hash when it was reviewed, as `{ "<form id>" = "<sha256>" }`, one for every listed form. A
  review with `definitions` binds each form by its definition and fingerprint alone and has no
  `catalogue_sha256`, `obligations_sha256` or exclusions. An edit to another form in the same
  catalogue or obligations file, or to another form's reading, leaves it current; any change to
  the form's own catalogue entry, obligations, reading or fingerprint makes it stale for that form.
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
- That phase is what the case's `expect` allows, when it has one.
- The current parser's verdict matches that phase: a `syntax_error` receipt matches a local
  rejection, and every other phase matches a local acceptance.
- An acceptance is structural: no statement contains a Command or Raw fallback node.

A `semantic_error`, `runtime_error` or `unsupported_feature` receipt with a structural local
acceptance satisfies the obligation for syntax and is counted as `semantic_only`.

Every local parse runs in a worker child process of the checker, bounded to 10 seconds and 4 GiB of
virtual memory, and restarted after any failure. A panic, a crash such as a
stack overflow or the memory bound, a timeout, or a complexity-guard error (`E_GUARD_*`) is a local
parse failure, never a syntax rejection, and fails that case as `local_parse_failure`; the run goes
on.

A form is verified only when it has at least one required obligation, every required obligation
is satisfied, and an approved review of its family is current for it. A review with
`definitions` is current when the form's recorded definition hash and fingerprint both equal its
current ones. Any other review is current when it matches the current catalogue and obligation file
hashes and either the form's recorded fingerprint or, without one, the current parser tree.
The parser tree is `git rev-parse HEAD:crates/sqlquack-syntax`, and none at all while that path has
uncommitted changes, so every review without fingerprints is stale then.

A form's fingerprint is the SHA-256 of one line per distinct case citing any of its obligations,
sorted by case id: the case id, its `sql_sha256` and its local verdict, tab-separated, each line
ending in a newline. The verdict is `accepted` followed by a tab and the SHA-256 of the parsed
statements as compact JSON, exactly as serialised, `rejected` followed by a tab, or `failed:<Failure>`
followed by a tab. `family --json` prints each form's `fingerprint`. A parser change that leaves
every case of a form parsing into the same tree, and every rejection still a rejection, leaves its
fingerprint and so its review unchanged; a changed tree or verdict, or an added or removed case,
changes it. A form's definition hash is the SHA-256 of the compact JSON array of its catalogue
entry and its own obligations in file order, as loaded, followed by its `[[reading]]` from
`limits.toml` when it has one; `family --json` prints it as
`definition_sha256`. A form is excluded only by an approved review that is current in the
same way. Absent forms keep their ids and are never counted. The headline denominator is the
catalogued forms: 3,348 on the frozen catalogue `e863d7de`.

Anything missing, ambiguous, stale, mismatched or opaque fails closed with a named reason:
`no_obligations`, `missing_case`, `unverifiable_internal_error`, `missing_receipt`,
`awaiting_observation`, `stale_receipt`, `ambiguous_receipts`, `unexpected_outcome`,
`local_parse_failure`, `disagreement`, `opaque_fallback`, `unreviewed` or `stale_review`. A reference that does not
resolve fails every check command. Examples are an obligation of an uncatalogued form, a case
citing an undeclared obligation, an alias whose source is not absent or whose target is unknown,
and an exclusion naming an unknown form or page.

## Reports

`cargo run -p sqlquack-coverage-check -- <command> [--root coverage/syntax] [--json]` prints
results on stdout and progress on stderr:

- `summary`: the verified share of catalogued forms, obligation completion, forms per reason, and
  per-family scope.
- `family <family>`: every form of one family with its state, completion and reasons. The JSON
  lists each obligation with every citing case, its receipt phase and the local verdict.
- `missing [--family <family>]`: every unsatisfied required obligation and its reason.
- `stale [--family <family>]`: every form whose reviews are all stale, with its current fingerprint
  and definition hash, and each naming review's recorded fingerprint, definition hash and parser
  tree.
- `pages [--family <family>]`: the page rollup, a secondary view. A form counts under its own
  `source` only.
- `boundaries --form <id>`: the form's boundary and nesting-depth obligations, with their bounds,
  notes and cases.
- `snapshot --catalogue-commit <commit>`: every form's state and every satisfied obligation as
  JSON, with the receipted cases Snowflake parsed split into those the parser accepts (opaque or
  not) and those it rejects. It is refused unless the catalogue, `sources.toml` and `aliases.toml` in the working tree
  equal those of the commit.
- `diff BASE [HEAD]`: one snapshot against a later one, or against the current tree when HEAD is
  omitted.

## Baseline

`coverage/syntax/baseline.json` is a snapshot on the frozen catalogue. `make syntax-coverage`,
part of `make check-ci`, runs `diff` against it and fails in two cases. The first is a coverage
regression: a form the baseline records as verified or excluded that no longer is, a satisfied
obligation that no longer is, or a receipted case Snowflake parsed that the base parser accepted
and the current one rejects, or a case Snowflake refused as syntax that the base parser rejected
and the current one accepts. Both join each case to its receipts by SQL, setup and session hashes,
whatever the case expects: Snowflake parsed a case when every receipt has a non-syntax phase and an
errno other than 1003, and refused it when every receipt is a syntax error with errno 1003 or 921;
a positionless 1003 or receipts that disagree count as neither. The first line catches a case that
moves from an opaque acceptance, which satisfies no obligation, to a rejection; a pull request that
has one explains each case. A case marked `documented` that the current parser now accepts
is listed on its own line for review and does not fail the diff. A
snapshot taken before these case lists existed has none, so it reports no such case. The second is a catalogue change: a changed sources, aliases or
catalogue file, or a form added or removed. Catalogue changes are reported separately from
coverage changes. Gains pass. After a reviewed gain or catalogue change, re-record the baseline
with `snapshot` and commit it with the change. A review without fingerprints binds the parser
tree, so any parser commit makes its verified forms stale until it is renewed; a review with
fingerprints goes stale only for the forms whose cases now parse differently. Either shows up as a
regression.

## Receipt contradictions

The case lists above only reach receipts through the catalogue: a receipt whose case id no form
cites, such as a retired or hand-written case, never counts. A snapshot therefore also records
`receipt_contradictions`: every current receipt, cited or not, whose statement the parser
contradicts. Statements are grouped by SQL, setup and session hashes and judged as above: the parser
rejects a statement Snowflake parsed, or accepts one Snowflake refused as syntax. The SQL comes from
the case that names the hash or from the `inputs` of the run reports beside the receipts, each
checked against its hash. A receipt neither gives is listed in `unjudged_receipts` and judged by
nothing. `diff` fails on a receipt the head contradicts and the base did not. A receipt of a case
marked `documented` is listed on its own line instead. Receipts both sides
contradict, and every head contradiction when the base snapshot predates these fields, are listed in
the JSON without failing.

The tracked baseline and receipts are a subset of the evidence, so a merge gate takes both snapshots
on the full catalogue and evidence: `snapshot` with the target branch's binary, `snapshot` with the
head's binary, then `diff BASE HEAD`.

## Translation

`syntax-coverage translation --sqlquack <binary> [--policy sqlquack.toml] [--family <family>]
[--jobs 4] [--json]` measures what conversion makes of the syntax Snowflake parsed. It never
changes a coverage verdict. The `sqlquack` binary must find `libduckdb.so`, for example through
`LD_LIBRARY_PATH`.

A case counts when its current receipt phase is `accepted`, `semantic_error` or `runtime_error`
and it does not expect a rejection. Each counted case runs in its own
`sqlquack convert --loud-divergences-only --json --policy <policy> -` process, bounded to 120
seconds and 4 GiB. Its outcome is one of:

- `translated`: convert converted it. For a query, that includes DuckDB parsing and binding the
  printed SQL over the declared relations. `binds` is reported only for cases with an empty setup,
  because a setup's tables are not replayed.
- `by_design`: refused only for constructs in `coverage/translation/by-design.toml` whose entry
  a reviewer signed off in `coverage/translation/reviews/by-design.toml`. A sign-off names
  the construct and the SHA-256 of the entry's reason, so a changed reason needs a new sign-off.
  Until then the case counts as not yet translated.
- `declared_value_divergence`: refused only because the policy refuses a known divergence that
  can return another value.
- `needs_schema`: refused only for unresolved names even with the case's census schema.
- `no_relation_in_scope`: refused only for unresolved names, with no census entry. It is a syntax
  probe and is never scored.
- `did_not_finish`: conversion ran past its bound or gave no report. It counts as not translated.
- `not_yet_translated`: anything else, with each refusal as `category: construct`.

A form's state comes from its counted cases. It is `not_measurable` when Snowflake parsed none of
them. It is `needs_schema` or `no_relation_in_scope` when none of its cases is measured. It
`translates` when every measured case translates. Otherwise it is `by_design` or
`declared_value_divergence` when every case that does not translate is, and `not_yet_translated`
otherwise. The headline is "X of Y measurable forms translate", where the measurable forms
exclude not-measurable, needs-schema and no-relation forms, followed by the refusals that block
the most forms. Results-match stays "not measured: no recorded result" until syntax cases record
Snowflake rows.

`coverage/syntax/census/<family>.jsonl` holds one line per case that reads relations:
`{"case_id": …, "relations": [{"name": …, "columns": [{"name": …, "type": …}]}]}`. It is written as a
source file of a project folder that convert reads with `--project`. Names are written as in the SQL and resolved as Snowflake resolves
them: an unquoted name is upper-cased, and a quoted one keeps its case. An unknown case id, a duplicate or an empty relation or column list
fails the command. `by-design.toml` holds `[[by_design]]` entries with `construct` (a construct
key such as `fn:SYSTEM$WAIT`), `reason` and `duckdb_note`. The review file holds `[[signoff]]`
entries with `construct`, `reason_sha256` and `reviewed_utc`. Both files are optional.
