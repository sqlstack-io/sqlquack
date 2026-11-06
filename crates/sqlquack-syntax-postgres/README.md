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
statement's text, and `Parsed.gaps` names each construct with its byte range. A gap statement is
accepted as opaque, never as parsed. For a statement gap (its first keyword), the text up to the
next top-level `;` is not checked further. The SQL/JSON and SQL/XML calls, `TREAT`, `XMLTABLE` and
`JSON_TABLE` are checked against the full grammar, including gram.y's own errors: an unknown
`FORMAT JSON ENCODING` (22023), a non-constant `JSON_TABLE` path (0A000), and repeated or unknown
`XMLTABLE` column options (42601).

| Gap | Construct |
| --- | --- |
| a statement's first keyword, such as `create` or `alter` | every statement other than `SELECT`, `VALUES`, `TABLE`, `INSERT`, `UPDATE`, `DELETE`, `MERGE`, `TRUNCATE` and `COPY` |
| `copy_where` | `COPY … FROM … WHERE condition` |
| `json`, `json_array`, `json_arrayagg`, `json_exists`, `json_object`, `json_objectagg`, `json_query`, `json_scalar`, `json_serialize`, `json_value`, `json_table` | the SQL/JSON call syntax |
| `xmlconcat`, `xmlelement`, `xmlexists`, `xmlforest`, `xmlparse`, `xmlpi`, `xmlroot`, `xmlserialize`, `xmltable` | the SQL/XML call syntax |
| `treat`, `merge_action` | `TREAT (… AS type)`, `MERGE_ACTION()` |
| `is_json`, `is_document`, `is_normalized` | `IS [NOT] JSON …`, `IS [NOT] DOCUMENT`, `IS [NOT] [form] NORMALIZED` |
| `at_local` | `AT LOCAL` |
| `prefix_operator` | a prefix operator other than `-`, `+`, `NOT` and `~`, such as `\|/ x` or `@ x` |
| `quantified_operator` | `op ANY/ALL (…)` with an operator other than `=`, `<>`, `<`, `<=`, `>`, `>=` |
| `order_by_using_operator` | `ORDER BY … USING op` with an operator other than `<` and `>` |
| `setof` | `SETOF` in a type name |
| `cte_search`, `cte_cycle` | `SEARCH` and `CYCLE` after a common table expression |
| `set_operation_with_ties`, `set_operation_locking` | `FETCH … WITH TIES` and `FOR UPDATE`-style locking on a set operation |
| `rows_from`, `ordinality`, `column_definition_list` | `ROWS FROM (…)`, `WITH ORDINALITY` on a special call, `AS (col type, …)` after a function |
| `tablesample` | `TABLESAMPLE method (…) [REPEATABLE (…)]` |
| `join_using_alias`, `joined_table_column_aliases` | `USING (…) AS alias`, column aliases on a parenthesized join |
| `returning_with` | `RETURNING WITH (OLD AS …, NEW AS …)` |
| `overriding` | `OVERRIDING {USER \| SYSTEM} VALUE` in `INSERT` or a `MERGE` insert |
| `set_target_indirection`, `multiple_column_set` | `SET a.b = …` or `SET a[1] = …`, and `SET (a, b) = …` from anything but a row of as many items, which is split into one assignment each |
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
