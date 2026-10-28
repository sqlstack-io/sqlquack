# Agent Instructions

## Product Rules

- SQLQuack refuses to guess. Never emit a DuckDB node that no verified rule produced, and never pass
  a source construct through because it prints identically in DuckDB. Identical syntax is not
  evidence of identical meaning.
- Every refusal names its construct, its source position and its category: missing rule, not
  expressible in DuckDB, or parse fidelity. Add categories rather than overloading one.
- Do not trust or enable polyglot's `transpile` feature or any of its dialect conversions. Use
  polyglot for parsing, name resolution and printing only.
- The source dialect is a parameter of every entry point. Do not hard-code Snowflake below the
  command line.
- Entries in the spelling-only field list and normalisations (`sqlquack-core` comparison constants)
  weaken the re-parse checks. Each needs a reason and a test showing two spellings differ only
  there. Do not add one to make a failing check pass.

## Evidence Integrity

- Recorded warehouse results are the only arbiter of correctness. Agents never record, re-record,
  edit or delete recorded results, and never contact a warehouse. Report pending cases to the
  maintainer instead.
- Recorded results are append-only and named by the hash of their case. A changed case is a new
  case.
- Everything that could be weakened to make a failing case pass (comparator tolerances, required
  edge-case inputs, known divergences) lives in one reviewed policy file. There are no per-case or
  per-rule overrides elsewhere.
- Tests that need a result use DuckDB stand-ins written to temporary directories and marked as
  test fixtures. Never commit a stand-in under `evidence/results`; the evidence guard rejects any
  result the recorder did not write.
- When a case fails, either fix the rule or make it refuse. Never widen a rule to raise the
  conversion rate.

## Testing

- Every behaviour change ships with tests. Prefer tests that run the real pipeline and the real
  `sqlquack` binary over tests of isolated helpers.
- Tests follow the fensu Rust test layout: a `tests.rs` harness of `#[path]` modules, a sibling
  `test_types.rs` with test-case structs, a local `test_cases` array per test, one loop, and
  assertion messages carrying `test_case.description`.
- Run focused tests with `cargo test -p <crate> <filter>`. `make test` runs the workspace suite.
- Before pushing, run `make check-ci`.
- Every wrong value or hazard that a review or a recording finds also gets a matching synthetic
  case in SQLBuild's DuckShop edge-case area, as SQLBuild's `AGENTS.md` describes under "DuckShop
  Edge Cases". Report each one in neutral terms so it can be added there.

## Public Repository Hygiene

- Treat every tracked file, fixture, filename, commit, branch, pull request, review comment and CI
  log as publicly visible, including history.
- Use only neutral synthetic examples such as orders, customers, products, inventory, fulfilment
  and support tickets. Never copy organisation-, client-, provider-, environment- or
  production-specific names, SQL, data, query output, identifiers or counts into the repository,
  even when the source was read-only.
- Measurements on private projects stay private. A bug they expose becomes a rewritten neutral
  reproduction.
- Load and follow the `public-repository-hygiene` skill before editing or publishing. Run its
  scanner over the working tree and introduced history before every push. Never bypass the
  configured Git hooks; a missing or failing scanner blocks publication.
- Do not copy the private restricted-vocabulary list or scanner into this repository.

## Command-Line User Experience

- Potentially slow phases print a concise start message and an explicit completion or failure
  message on stderr.
- Machine-readable output (`--json`) goes to stdout alone; progress goes to stderr.
- Reports never print SQL text or file names unless the user asks for them.

## Native Builds

- Polyglot is one large crate. Keep crates small so a rule change does not rebuild everything, and
  do not add dependencies that rebuild polyglot.
- Pin polyglot to the same revision as SQLBuild and bump it deliberately, with a census before and
  after.

## Subagent Verification

- The primary agent owns the verification plan. Implementation subagents run only tests covering
  their change plus focused static checks; review subagents are read-only.
- Follow-up reviews verify only previously reported findings and their boundaries.

## Delivery Workflow

- Consolidate related work for one release into one branch and one pull request.
- Run focused tests and `make check-ci` before pushing; CI is the broad gate.
- Pull request titles follow Conventional Commits. Descriptions have `## Why`, `## Changes` and
  `## Verification` sections.
- Monitor CI after every push and address failures before considering delivery complete.

## Review Discipline

- Treat review findings as hypotheses to validate against supported behaviour and realistic
  execution paths.
- Prioritise correctness: a conversion that changes meaning is the worst possible defect, and a
  refusal is always an acceptable fix.
- Keep fixes within scope. Escalate anything that expands supported behaviour, recorded evidence or
  the policy file to the maintainer.
