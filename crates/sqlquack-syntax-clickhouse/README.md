# sqlquack-syntax-clickhouse

A ClickHouse 26.1 lexer and parser that builds the shared `sqlquack-syntax` tree. The lexer follows
ClickHouse's `Lexer.cpp` rule for rule, and the parser follows its `Parser*` classes.

`parse(sql)` returns every statement, or the error ClickHouse's parser reports. `parse_strict` is
the same, because every byte of the input lands in a token, so a lexical fault is already a parse
error.

## Gaps

A statement holding a construct the shared tree cannot hold parses as a `Command`, and `Parsed.gaps`
names the construct: for example `alter_action`, `alter_in_partition`, `apply_column_transformer`,
`into_outfile`, `interpolate`, `optimize_deduplicate_by`, `query_parameter`, `setting_map`,
`statement_output`, `table_element`, `truncate_no_delay`, `ttl_action`, `with_ties` and
`unsupported_statement`.

`OPTIMIZE` and an `ALTER` with an `UPDATE` action have no node in the shared tree, so they parse
whole as a `Command` with no gap.

## Local parser limits

The parser refuses some deep or long inputs that ClickHouse 26.1 accepts, so that no input exhausts
the caller's stack, and accepts a few that ClickHouse refuses through a setting rather than its
grammar. These are declared differences, not gaps for a checker to find. SQLBuild generates
nothing near any of them.

| Limit | Locally (refusals say "Maximum parse depth exceeded") | ClickHouse 26.1 |
| --- | --- | --- |
| `nesting-depth-200` | refuses past 200 nested levels, each chain link counting as one: postfixes (`::`, `.1`, `[1]`, `IS NULL`), `LIKE`, `IN`, `BETWEEN`, set operations, and the parts of a dotted column name beyond the second | accepts up to `max_parser_depth` (1,000), such as a column name of 300 dotted parts |
| `tree-depth-512` | refuses a statement whose finished tree is more than 512 levels deep (the shared parser's AST guard), counted iteratively after it parses, such as 60 nested parentheses each holding 60 casts | refuses 60 by 60 nested casts too, with `TOO_DEEP_AST` from `max_ast_depth` (1,000) |
| `flat-and-chain-512` | refuses a flat `AND` or `\|\|` chain past 512 terms | accepts 5,000 terms |
| `flat-plus-chain-512` | accepts a flat `+` chain of 499 to 512 terms | refuses it with `TOO_DEEP_AST` from `max_ast_depth`, as it nests one call per term past 498 |

## Semantic profile limits

The ClickHouse profile in `sqlquack-semantics` reports E222 for a qualifier that names no table,
alias, CTE, lambda parameter or catalogued column, even when a relation in scope has no column
list. ClickHouse also reads `x.y` as a subcolumn of a column `x`, so over a relation whose columns
are unknown the profile reports these six forms wrongly. ClickHouse 26.1 accepts each of them when
the column exists, and the previous analyser reported them the same way.

| Form | Example over an open `raw` | Column ClickHouse reads |
| --- | --- | --- |
| Nested | `SELECT zq.id FROM raw` | `zq Nested(id Int64)` |
| Dotted name | `SELECT zq.id FROM raw` | `` `zq.id` Int64 `` |
| Tuple element | `SELECT tup.a FROM raw` | `tup Tuple(a Int64)` |
| JSON path | `SELECT j.k FROM raw` | `j JSON` |
| Map subcolumn | `SELECT m.keys FROM raw` | `m Map(String, Int64)` |
| Variant subcolumn | `SELECT v.String FROM raw` | `v Variant(Int64, String)` |
