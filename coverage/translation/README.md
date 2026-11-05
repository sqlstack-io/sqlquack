# Translation results measure

How much documented Snowflake syntax SQLQuack translates to DuckDB with results matching
Snowflake, measured over the syntax catalogue in `coverage/syntax` (decision D139). It is reported
separately from syntax coverage, and every figure comes from the checker.

## Inputs

- Recorded statements: every evidence case under `cases/`, with Snowflake's recorded result.
  `sqlquack verify --translation-statements FILE` replays them on DuckDB and writes each
  statement's outcome and the constructs it uses.
- `form-links.toml`: how constructs link to catalogue forms. A function call links by name to every
  function form whose grammar calls that name; this file links operators, clauses, casts and the
  rest, and lists the constructs that name no form of their own.
- `not-translatable.toml`: forms with no meaningful DuckDB translation, in groups, each with its
  reason. A group leaves the denominator only once the auditor signs it off in
  `reviews/auditor-results.toml`, which binds its reason and forms by SHA-256.
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
| `matching` | Otherwise, at least one linked statement returns Snowflake's rows. |
| `syntax_only` | Otherwise, linked statements exist but none returns compared rows: only types or matching errors. |
| `unmeasured` | No recorded statement links to it. |

## Figures

- Forms: `matching` forms out of all catalogued forms less the signed-off not-translatable ones.
- Statements, over the statements recorded as evidence (not a sample of real-world SQL): `matching`
  statements out of all recorded ones, and, as a separate line, `matching_error` statements.
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

The check fails on a construct that neither links nor names no form, a link to a form the catalogue
does not hold, a signed-off not-translatable form with a matching statement, and any difference
from `results-baseline.json`. Against the target branch's baseline it prints the diff and fails if a
statement or form drops out of matching, or becomes `differs`.

The report's worklist lists differences first, then refusals by how many forms each keeps from
matching, then forms that need a recorded result.
