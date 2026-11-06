# sqlquack-syntax-postgres

A PostgreSQL 18 scanner and parser that builds the shared `sqlquack-syntax` tree. It follows the
18.6 sources (`scan.l`, `gram.y`, `parser.c` and `kwlist.h` from `postgresql-18.6.tar.bz2`, sha256
`555610c24d53e4316da5b7d3fc25c279d96856d5e0e23ee308c328c5fa881d9f`).

`parse(sql)` returns every statement, or the error PostgreSQL's raw parser reports. `ParseError`
carries the message, the byte position and whether the SQLSTATE is 42601. Other parser errors, such
as an invalid Unicode escape (22025), the `UNIQUE` predicate (0A000) or the stack depth limit
(54001), have `syntax = false`.

## Gaps

A statement holding a construct the shared tree cannot hold becomes a `Command` with the
statement's text, and `Parsed.gaps` names each construct with its byte range. The text up to the
next top-level `;` is not checked further, so a gap statement is accepted as opaque, never as
parsed.

| Gap | Construct |
| --- | --- |
| a statement's first keyword, such as `create` or `alter` | every statement other than `SELECT`, `VALUES`, `TABLE`, `INSERT`, `UPDATE`, `DELETE`, `TRUNCATE` and `COPY` |
| `copy_where` | `COPY … FROM … WHERE condition` |
| `merge` | `MERGE` |
| `json`, `json_array`, `json_arrayagg`, `json_exists`, `json_object`, `json_objectagg`, `json_query`, `json_scalar`, `json_serialize`, `json_value`, `json_table` | the SQL/JSON call syntax |
| `xmlconcat`, `xmlelement`, `xmlexists`, `xmlforest`, `xmlparse`, `xmlpi`, `xmlroot`, `xmlserialize`, `xmltable` | the SQL/XML call syntax |
| `treat`, `merge_action` | `TREAT (… AS type)`, `MERGE_ACTION()` |
| `is_json`, `is_document`, `is_normalized` | `IS [NOT] JSON …`, `IS [NOT] DOCUMENT`, `IS [NOT] [form] NORMALIZED` |
| `at_local` | `AT LOCAL` |
| `prefix_operator` | a prefix operator other than `-`, `+`, `NOT` and `~`, such as `\|/ x` or `@ x` |
| `quantified_operator` | `op ANY/ALL (…)` with an operator other than `=`, `<>`, `<`, `<=`, `>`, `>=` |
| `order_by_using_operator` | `ORDER BY … USING op` with an operator other than `<` and `>` |
| `setof` | `SETOF` in a type name |
| `group_by_distinct` | `GROUP BY DISTINCT` |
| `cte_search`, `cte_cycle` | `SEARCH` and `CYCLE` after a common table expression |
| `set_operation_with_ties`, `set_operation_locking` | `FETCH … WITH TIES` and `FOR UPDATE`-style locking on a set operation |
| `rows_from`, `ordinality`, `column_definition_list` | `ROWS FROM (…)`, `WITH ORDINALITY` on a special call, `AS (col type, …)` after a function |
| `tablesample` | `TABLESAMPLE method (…) [REPEATABLE (…)]` |
| `join_using_alias`, `joined_table_column_aliases` | `USING (…) AS alias`, column aliases on a parenthesized join |
| `returning_with` | `RETURNING WITH (OLD AS …, NEW AS …)` |
| `overriding` | `INSERT … OVERRIDING {USER \| SYSTEM} VALUE` |
| `set_target_indirection`, `multiple_column_set` | `SET a.b = …` or `SET a[1] = …`, and `SET (a, b) = …` |
| `where_current_of` | `WHERE CURRENT OF cursor` |
| `index_element_collate`, `index_element_opclass` | `COLLATE` and operator classes in an `ON CONFLICT` target |
| `delete_using_non_table` | `DELETE … USING` with a join, subquery or function |

## Lossy mappings

These parse without a gap, but the shared tree keeps less than the source said:

- `IS [NOT] UNKNOWN` becomes `IsNull`, which means the same for a boolean.
- `CURRENT_DATE`, `CURRENT_TIMESTAMP [(p)]` and the other SQL-value functions become `Function`
  nodes, with `no_parens` set when written without parentheses.
- `EXTRACT`, `POSITION`, `SUBSTRING`, `OVERLAY`, `TRIM`, `NORMALIZE` and `COLLATION FOR` become the
  plain calls PostgreSQL turns them into (for example `TRIM(LEADING …)` becomes `ltrim`).
- `LIMIT ALL` becomes `LIMIT NULL`; `FETCH FIRST ROW ONLY` without a count on a set operation
  becomes `LIMIT 1`.
- `INTERVAL (p) '…'` keeps the interval type but not its precision.
- `VALUES` given `ORDER BY`, `LIMIT` or `WITH` becomes `SELECT * FROM (VALUES …)`.
