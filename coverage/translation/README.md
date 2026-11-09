# Translation results measure

How much documented Snowflake syntax SQLQuack translates to DuckDB with results matching
Snowflake, measured over the syntax catalogue in `coverage/syntax`. It is reported
separately from syntax coverage, and every figure comes from the checker.

## Inputs

- Recorded statements: every evidence case under `cases/`, with Snowflake's recorded result.
  `sqlquack verify --translation-statements FILE` replays them on DuckDB and writes each
  statement's outcome and the constructs it uses.
- `form-links.toml`: how constructs link to catalogue forms. A function call links by name to every
  function form whose grammar calls that name; this file links operators, clauses, casts and the
  rest, and lists the constructs that name no form of their own.
- `not-translatable.toml`: forms with no meaningful DuckDB translation, in groups, each with its
  reason. A group leaves the denominator only once a reviewer signs it off in
  `reviews/results.toml`, which binds its reason and forms by SHA-256.
- `obligations/<family>.toml`: what each form must be shown to do. Each
  `[[obligation]]` has `id` (`<form>.t<NNN>`), `form`, `kind` (`type_combination`,
  `null_argument`, `boundary` with a `subkind`, `error_parity`), `target`, `note`, `catches`,
  `reading`, `cases` (references `cases/translation/<family>/<file>.toml#<case>`) and `required`.
  Two shapes have no case:
  - held: `required = true`, `cases = []`, `held = "<reason>"`, for a case held back (a parse gap,
    a case waiting on another PR or a receipt). It is never met, so its form cannot match.
  - unmeasured: `required = false`, `cases = []`, `reason = "<reason>"`, for a documented
    alternative no case can measure. It is never met and does not block its form.
- `retired-obligations.toml`: withdrawn obligation ids, each `[[retired]]` with `id`, `form`,
  `reason` and `replaced_by` (live obligation ids). A retired id is never reused.
- `results-baseline.json`: every statement's outcome and every form's state, as the checker last
  measured them.

## Outcomes

A statement:

| Outcome | Meaning |
| --- | --- |
| `matching` | Translated, and DuckDB returned Snowflake's recorded rows. |
| `matching_error` | Translated, and DuckDB raised the same kind of error Snowflake recorded. |
| `shape_only` | Only types were recorded, or only two Snowflake spellings were compared. |
| `declared_divergence` | Translated, and DuckDB raised a declared divergence's guard where Snowflake returned rows. |
| `refused` | SQLQuack refused it, with its `SQ` code; an opt-in divergence counts as refused. |
| `matching_rejection` | SQLQuack refused it, and its case declares `compile_error = true` and records Snowflake refusing to compile it (SQLSTATE class `42` or `0A`), and the refusal is specific to the invalid use: its code is not `SQ0002`, and no case of the same case file without `compile_error` is refused on one of its refused construct keys. It counts as `matching_error`; otherwise, or for a recorded runtime error, the statement stays `refused`. |
| `unrecorded_translates`, `unrecorded_refused` | No Snowflake result is recorded yet. |
| `differs` | DuckDB's result differs from Snowflake's: a bug. |

A form takes its state from the statements linked to it. A refused statement, or one whose guard
raised, counts only against the forms of the constructs its refusal names, or against all of its
forms when it names none. Any other statement counts for every form it links to.

| State | Rule |
| --- | --- |
| `not_translatable` | Listed in a signed-off group. Outside the denominator. |
| `differs` | Any linked statement differs. |
| `refused` | Any linked statement is refused for it, raised a guard for it, or cannot be judged. |
| `no_obligations` | Otherwise, a linked statement returns Snowflake's rows, but the form declares no required obligation, so nothing shows what it must do. |
| `unmet_obligations` | Otherwise, a linked statement returns Snowflake's rows, but a required obligation is unmet or held. |
| `matching` | Otherwise, at least one linked statement returns Snowflake's rows, and it declares required obligations, every one met. |
| `syntax_only` | Otherwise, linked statements exist but none returns compared rows: only types, matching errors or matching rejections. |
| `unmeasured` | No recorded statement links to it. |

An obligation is `met` when it cites at least one case and every cited case is `matching` or
`matching_error` or `matching_rejection`; otherwise it is `unmet`, or `held` or `unmeasured` by its shape. The report lists
each obligation's state, the `held` and `unmeasured` obligations with their reasons, and the
`uncited_cases`: recorded translation cases no obligation cites, which meet no obligation.

## Figures

- Forms: `matching` forms out of all catalogued forms less the signed-off not-translatable ones.
- Statements, over the statements recorded as evidence (not a sample of real-world SQL): `matching`
  statements out of all recorded ones, and, as a separate line, `matching_error` and
  `matching_rejection` statements together.
- Syntax only: a separate count, never part of either figure.
- Capture queue: translatable function, expression and query forms with no matching or refused
  statement. DDL, DML, session and utility forms without one are a separate gap: measuring them
  needs a receipt of a statement's effect, which is not built.

## Commands

```sh
make verify-recorded           # verify, then the measure against the baseline and the target branch
syntax-coverage results target/translation-statements.json --report target/translation-results.json
syntax-coverage results target/translation-statements.json --write-baseline
```

The check fails on obligations it cannot read whole: a missing `obligations/` directory, an
unknown key, a form the catalogue does not hold or that is absent from its source, an id that is
not the form's or is declared or retired twice, a case outside its family's folder, cited twice,
not recorded, or in a case file whose `construct` is another form, a boundary without a known
subkind, a required obligation with neither cases nor a `held` reason, a blank reason, and
a retired id that is reused or replaced by an id that is not a live obligation. It also fails on a
construct that neither links nor names no form, a link to a form the catalogue
does not hold, a signed-off not-translatable form with a matching statement, and any difference
from `results-baseline.json`. Against the target branch's baseline it prints the diff and fails if a
statement or form drops out of matching, a met obligation stops being met, an obligation is
dropped without being listed in `retired-obligations.toml`, or anything becomes `differs`.

The report's worklist lists differences first, then refusals by how many forms each keeps from
matching, then forms that need a recorded result.
