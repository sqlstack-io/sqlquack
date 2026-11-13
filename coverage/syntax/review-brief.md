# Snowflake form review brief, version 1

This brief is frozen. A review records which version it applied, and a new criterion becomes a new
version, never an edit of this one. Reviewers judge the obligations the generator declares and
criteria (a) to (d) below, and nothing else. A gap no declared obligation covers is not grounds for
rejection: list it under "Proposed obligation kinds" in the final message instead.

The text below is the brief given to each reviewer, with `{MANIFEST}`, `{N}`, `{ALLSQL}`, `{OUT}` and
`{WORKTREE}` filled in.

---

You are reviewing Snowflake syntax-coverage dossiers for the public SQLQuack parser, one verdict per
form. This is a read-only review: write only the verdicts file named below, and change no other
file.

Input: the manifest `{MANIFEST}` lists {N} forms, one per line (form id, then the dossier path).
Each dossier (`<form>.md`) holds the form's catalogue entry with its grammar and docs source, every
obligation with its cases, each case as `[receipt phase | case expect | SQLQuack verdict] SQL`, the
SQLQuack parse tree under each case, and, for function and method forms, a section of arity cases.
Trees longer than 4,000 characters are cut off; the full trees are in `{ALLSQL}.out`, line for line
with `{ALLSQL}`.

Approve a form only when all four criteria and the outcome check hold:

(a) Grammar: the catalogue grammar matches the Syntax block of the form's docs page token for
token, after collapsing whitespace. Quirks in the docs (a stray `|`, an unclosed bracket, a missing
space) are copied as printed and are fine. A form whose page has no Syntax block is judged against
the page prose. The dossier's `docs` line gives the page URL.

(b) Coverage: every obligation the dossier lists has at least one case, and that case does what its
obligation says: a missing case omits its operand, a wrong-kind case fills it as (c) describes, an
alternative case takes its branch, and a present or absent case includes or leaves out its clause.
Judge only the listed obligations. An arity case with a receipt, in the dossier's "Arity cases"
section, counts as that argument's missing case: an omitted function or method argument that still
leaves a well-formed call is found when the function name is resolved, not by the parser.

(c) Wrong-kind fills: a name gets a string literal (`'v1'`), a string operand gets a bare identifier
(`m1`), a number gets `'x'`, a bare word such as a contact purpose or privilege gets `'v1'`. Valid
cases keep every required clause.

(d) Trees preserve meaning: each clause or option lands in its own field or flag, not in a raw
string; quoting survives (`"v1 Q"`, `"SELECT"`); an omitted name is absent (null), never an
invented empty name `""`; no keyword or value the SQL wrote is lost. A case Snowflake accepted that
reads a following keyword as a name (`SHOW VIEWS IN APPLICATION ;` as a schema named APPLICATION)
is fine when the receipt shows Snowflake also got past its parser.

Outcome check: for every case, a `syntax_error` receipt needs SQLQuack `rejected`, and an
`accepted`, `semantic_error`, `runtime_error` or `unsupported_feature` receipt needs SQLQuack
`accepted`. Take receipts as given; do not question them.

Write `{OUT}` with exactly one line per manifest form, in manifest order: `<form id>`, a tab,
`approve` or `reject`, a tab, and a reason of at most 300 characters. A reject reason names the
criterion letter and the evidence, such as an obligation id, the SQL and the tree fragment. Reject
on any single failed criterion; when a judgement call could go either way, take the stricter
reading and say so in the reason.

Fetch docs pages into a new empty directory under /tmp, keep any helper scripts in a different
directory, run Python with `-I`, and delete both when done. Do not touch `{WORKTREE}` or run builds
or tests.

Your final message: the approve and reject counts, each rejection with its evidence, defects shared
by several forms, the judgement calls another reviewer might make differently, and under "Proposed
obligation kinds" any gap that no listed obligation covers, with one example form each.
