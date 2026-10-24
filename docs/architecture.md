# Architecture

## Principle

SQLQuack never passes a construct through unconverted. Identical syntax in two dialects is not
evidence of identical meaning, so every node of the output must come from a rule that is verified
against recorded Snowflake results, under stated type conditions. Anything else is refused.

## Pipeline

```
source SQL ─ parse (polyglot, source dialect) ─▶ tree
  ├─ fidelity: print back in the source dialect, re-parse, compare    (else refuse: parse fidelity)
  ├─ classify: every node, function, cast type, join kind and clause  (one construct each)
  ├─ convert:  each construct must be claimed by a rule               (else refuse: missing rule,
  │                                                                     or not expressible)
  ├─ print:    polyglot's DuckDB printer
  └─ check:    re-parse the DuckDB SQL and compare with the intended tree (else refuse: printer)
```

Today the rule catalogue is empty, so the convert step refuses every construct. The source dialect is
a parameter of every entry point; Snowflake is the only one implemented.

## Crates

| Crate | Role |
|---|---|
| `sqlquack-core` | Parsing with complexity guards, construct classification, refusals, the tree comparator and both re-parse checks. |
| `sqlquack-snowflake` | The Snowflake typing layer (empty). Polyglot leaves many Snowflake expressions untyped, so exact types (`NUMBER(p, s)`, timestamp kinds, semi-structured values) are computed here. |
| `sqlquack-rules` | The rule catalogue and declared refusals (empty). |
| `sqlquack-duckdb` | DuckDB building blocks shared by rules (empty). |
| `sqlquack-compare` | The strict comparator for recorded results (stub). |
| `sqlquack` | The `sqlquack` command line and the library entry points: the pipeline and the census. |

Polyglot is pinned to one revision with `default-features = false`. Its `transpile` feature stays
off: none of its own conversions are used.

## Constructs

A construct is a key `<family>:<name>`:

- `node:<kind>`: a polyglot AST node kind, such as `node:select`, `node:eq` or `node:coalesce`;
- `fn:<NAME>`: a function polyglot keeps as a generic call, by upper-case name; calls to project
  functions are one construct, `fn:<udf>`;
- `cast:<TYPE>`: the base target type of a cast, as polyglot prints it in the source dialect;
- `join:<kind>`: a join kind;
- `clause:<name>`: a query clause or set-operation form, such as `clause:qualify` or
  `clause:union_all`;
- `syntax:<name>`: a spelling recorded separately, such as `syntax:double_colon_cast`.

Clause classification destructures polyglot's query structs field by field, so a polyglot upgrade
that adds a clause fails to compile until the clause is classified.

Each construct carries a source position. Polyglot keeps positions only on identifiers, columns,
tables, stars and some calls; any other node takes the first position inside it, or else its
parent's.

## Refusals

Every refusal has a category, a construct (absent when a whole statement is refused), a position and
a reason:

- **missing rule**: no verified rule converts the construct yet;
- **not expressible in DuckDB**: declared in the catalogue with a reason and evidence;
- **parse fidelity**: the SQL does not parse, or printing it back changes its tree.

## Tree comparison

Both re-parse checks compare trees as JSON after removing a declared list of spelling-only fields
(source positions, comments, `::` versus `CAST`, optional keywords and similar) and applying a
declared list of normalisations (function-name case, dollar-quoted strings, comment wrappers). Each
entry weakens the checks, so each has a reason and a test showing that two spellings parse to trees
that differ only there. A mismatch reports the first differing JSON path.

## Source fidelity

A statement is printed back in its own dialect and re-parsed. If the tree is unchanged it is
faithful. If it changes once and then settles, the change is a canonicalisation (for example `TOP`
becoming `LIMIT`); canonicalisations are refused until a recorded case proves both forms agree on
Snowflake. A tree that keeps changing is refused as unstable.

## Census

`sqlquack census` runs the pipeline over every `.sql` file and reports, per construct, how many files
use it, plus how many files would convert as the top N constructs get rules, in frequency order and
in greedy order (each next rule is the one that completes the most files). SQLBuild reference calls
(`__ref`, `__source`, `__seed`, `__udf`, `__table_fn`, cursor bounds) are replaced with neutral
placeholders before parsing. The report holds counts and construct keys only.

## Evidence

Recorded warehouse results are the only arbiter of correctness. Recording runs outside CI, through
the maintainer; results are append-only and named by the hash of their case, and everything that
could be weakened to make a failing case pass lives in one reviewed policy file.
