# Evidence coverage

A rule converts only what recorded cases prove. Every case passing is not enough: reviews kept
finding rules that accepted shapes and values no case recorded, such as GET with a computed key
starting with `$`, ARRAY_POSITION of `1` among elements written `1.0`, window aggregates over
explicit frames, FLOAT to NUMBER at unrecorded scales, and an infix `NOT ILIKE ANY`. The coverage
check finds these mechanically, in two layers.

- **Shapes.** Each rule declares the domain it accepts. The check enumerates the domain's
  combinations and reports each one with no passing recorded case.
- **Hazards.** A catalogue lists known-dangerous inputs for each type family. Each argument family
  a rule accepts needs a passing case for each of that family's hazards.

`sqlquack verify` reports both layers. Rollout:

1. Report only (now). `--require-coverage` turns the report into a gate, but the gate does not
   pass the flag yet.
2. The generated hazard cases are recorded. Every rule they expose is fixed or narrowed: it refuses
   the uncovered shape, or a guard makes it raise.
3. The gate passes `--require-coverage`, so no rule accepts more than its evidence.

## What a conversion records

Each converted expression node is a *frame*. A frame records the rules opened while it converts,
its construct key, its arguments and its facets. When the node converts, each of those rules gets
the frame's *shape* in the conversion's provenance (`shapes` in `verify --json`).

- **Construct.** `fn:<NAME>` for calls, `node:<kind>` for other nodes, `query` for query-level
  rules.
- **Arguments.** The child expressions the node converted, each counted once, in source order. If
  any child has no source location (a node built during conversion), they keep conversion order.
- **Each argument.**
  - *Family:* `integer` (NUMBER of scale 0), `decimal`, `float`, `text`, `boolean`, `date`,
    `time`, `timestamp_ntz`, `timestamp_ltz`, `timestamp_tz`, `variant`, `null` (an untyped
    NULL literal) or `query` (a subquery).
  - *Kind:* `literal`, or `value` for a column or any computed expression.
  - *Source:* the column or literal that feeds it, when it is one.
- **Facets.** Dimensions a rule's code tells apart that the arguments do not show:

| Facet | Values | Recorded by |
|---|---|---|
| `target` | the cast target's family | casts |
| `float_scale` | the NUMBER scale a FLOAT is cast to | casts |
| `quantifier` | `none`, `any`, `all` | LIKE and ILIKE |
| `negated` | `infix` when written `x NOT LIKE p` | LIKE and ILIKE |
| `function` | the window function's name | window functions |
| `frame` | `unordered`, `ordered`, `whole_partition`, `rows`, `range` | window functions |
| `partition` | `none`, the one partition key's family, `several` | window functions |

A window's partition and order keys convert outside its shape, so its arguments are the
function's own.

A rule opened inside another rule's construct gets that construct's shape too. For example,
`rule.comparison` applied inside DECODE gets a `fn:DECODE` shape. Its domain declares that form,
usually without arguments.

## Declaring a domain

`coverage/domains/*.toml` declares each rule's domain:

```toml
version = 1

[[rule]]
id = "rule.variant_computed_get"

[[rule.form]]
construct = "fn:GET"
arguments = [
  { families = ["variant"] },
  { families = ["text", "integer"], kinds = ["value"] },
]
template = "GET({0}, {1})"
category = "semi-structured-array"
```

**What a form accepts.**
- **Construct:** the form's `construct`, and the `also` list of constructs accepted alike, such as
  the other comparison operators. Each construct counts its own combinations; hazards count once.
- **Arguments:** one entry per position. Each entry lists the families the position accepts and,
  only when the rule treats literals and computed values differently, the kinds it accepts.
  - `repeat = true`: the last position may repeat, as in ILIKE ANY's list of patterns.
  - No `arguments` key: arguments are not a dimension of the form.
  - `arguments = []`: the form takes none, like a cast of a number literal, which reads the digits
    without converting the operand.
  - `grammar = true` on a position: a literal read as a grammar (a format, pattern, zone, flag or
    lambda body), which takes no hazards.
  - `one_wise = true`: each position's families need a case alone, not crossed with the other
    positions. Use it where the rule reads arguments independently (CONCAT, DECODE results, the
    parts of DATE_FROM_PARTS); crossing them would explode without testing anything new.
- **Facets:** the values each facet may take. A shape without a facet records `none`.
- **`each`:** facets whose values each need one passing case, not crossed with the rest, such as
  a window's partition key family.
- **Undeclared facets:** a shape carrying a facet the form does not declare is outside it.

**Combinations.** The domain's combinations are the product of each form's argument families,
declared kinds and facet values. Declare a combination only where the code accepts it. If the code
accepts a combination the domain does not declare, the check reports the shape as accepted outside
the domain. A rule whose domain cannot be stated declares `undeclarable = "<why>"` instead of
forms, and is reported as a finding.

**For generating cases.**
- `template`: the call with `{0}`, `{1}`… for the arguments.
- `query`: optional, for grouped and windowed forms. It uses `{expression}` and the group or
  partition column `g`.
- `category`: the case category.
- `rows = "group"` or `"window"`: the form reads row sets, which adds that family's hazards.
- `variants`: further templates, one per other construct or facet value (each `also` construct,
  window function and frame, cast target). Each variant gets a benign case per combination.
- `benign` and `sql_types`: per-position overrides of the family's benign cell and column type,
  for positions that need a specific value (a time zone, a format, a two-digit scale).
- `divergences`: maps a hazard to the divergence its case expects, keyed `<position>.<family>.<id>`,
  `<family>.<id>` or `<id>`, the most specific first. An empty value says the case expects none:
  Snowflake raises too, or DuckDB never reads the value. Fill it in once a recording, or a DuckDB
  run of the converted case, shows the guard raising; `expected_divergence` is not hashed, so
  setting it needs no re-recording. A mapped hazard gets a case of its own. The key `tied` names
  the divergence a combination case expects when two rows of one group hold equal `ordinal`
  arguments but different other values, as the equal row of a repeated family can make them.
- `ordinal`: positions filled with each row's number in hazard cases, such as WITHIN GROUP order
  keys, so rows never tie: Snowflake's order of tied rows is no contract. Combination cases keep
  their values, so their ties are real and `tied` declares the guard they meet.
- `row_groups = true`: grouped cases give each row its own group, for aggregates such as ANY_VALUE
  whose result over a group of different values is Snowflake's choice.

## Probes

A shape the code accepts outside its rule's `conditions` is a finding. The domain stays the truth,
so the shape is not declared as a form. It is declared as a probe instead:

```toml
[[rule.probe]]
construct = "node:between"
arguments = [{ families = ["float"] }, { families = ["integer", "decimal"] }, { families = ["integer", "decimal"] }]
template = "{0} BETWEEN {1} AND {2}"
category = "comparison"
extra_rows = [["9007199254740992", "9007199254740993", "9007199254740994"]]
```

A probe takes the same keys as a form and generates the same cases, but it never accepts a shape:
the check still reports the probed shape as accepted outside the domain. Once its cases are
recorded, a fully proven probe becomes a form, and the code refuses whatever is not proven.

- `extra_rows`: rows, one cell per non-literal argument, for one further case. Use it for values a
  single hazard cannot pair, such as a FLOAT tie with a NUMBER just past 2^53.
- An argument family the catalogue does not know, such as `query`, generates the form's `benign`
  cell for its position as a literal stand-in, for example a subquery.

## The hazard catalogue

`coverage/hazards.toml` is versioned: raise `version` with every change. Never remove or weaken a
hazard to close a gap. Each review finding becomes a hazard there, protecting every rule of its
family, or a facet or kind in the domains.

**Isolation.** Generated cases put all of a family's hazards into one case per argument, except
those marked `isolate = true`, which often make one engine raise for the whole query, and those a
form expects to raise; each of those gets a case of its own. A case whose recording is an error
covers only a hazard its column holds alone, since the error hides every other row.

**Divergences and unrecordable hazards.** A hazard's `divergence` names the divergence any
converted query holding it relies on, because no converted input holds the value, such as a
sub-microsecond TIMESTAMP_LTZ; its cases stand alone and expect it unless a form maps the hazard
otherwise. A hazard marked `unrecordable = "<why>"`, such as a nanosecond timestamp past 2262
that the recording connector cannot convert, generates no case; the report counts it apart,
neither covered nor a gap.

**Families.** Each family names the SQL type generated cases declare (`sql_type`) and two
unremarkable values (`benign`, `other`).

**Argument hazards.** A case shows a hazard when the column feeding the argument holds one of the
hazard's `cells`, compared exactly as source literals, or a cell containing one of its `contains`
texts. A literal argument shows it the same way. `pair = "case_only"` is shown instead by two
cells of the column differing only by case.

**Row-set hazards** (families `group` and `window`). A case shows one through `sql_contains`, text
in the case's SQL, or through `group`, a property of some group of the input column `g`:
- `all_null`: a group whose values are all NULL;
- `mixed`: a group holding different values;
- `null_beside_value`: a group holding NULL and a value;
- `duplicates`: a group holding the same value more than once;
- `single_row`: a group of one row.

**What counts as covered.** Only a case that passed, or diverged as its policy expects, covers a
combination or a hazard. A diverged case whose rows were not each proven alone (each matching its
recorded row or raising the divergence) covers a hazard only as an error recording does: when its
column holds that hazard alone, since the raise may hide the other rows.

## Generating cases

`python -m scripts.generate_cases` writes `cases/generated/hazards/` from the domains and the
catalogue:
- each templated form, its template and each variant, each declared combination of argument
  families and kinds (crossed, or one-wise): one case of two benign rows;
- each `each` partition value of a windowed form: one case partitioned by a key of that family,
  by two keys, or not at all;
- each argument position and family: one case holding that family's hazards in that argument
  (isolated hazards apart), the other arguments benign;
- each row-set hazard of a form that reads rows: one case.

Identical cases are written once. `sqlquack pending` then lists them for recording.

## The report

`sqlquack verify` prints coverage totals on stderr, unrecordable hazards counted apart. In text mode it also prints, for each rule with
a gap:
- each uncovered combination;
- each uncovered hazard;
- each shape accepted outside the domain.

`--coverage-json <file>` writes the full report. `--domains` and `--hazards` choose other files.
`make verify-recorded` writes it to `target/coverage-report.json` (`COVERAGE_REPORT`), and
`REQUIRE_COVERAGE=1` passes `--require-coverage`.

`--coverage-projected` plans a recording: it counts every pending case that converts as passed,
and lists the pending cases the converter refuses, which a recording would not cover. It cannot be
combined with `--require-coverage`.
