# sqlquack-syntax-bigquery

**Work in progress, unreviewed.** A BigQuery (GoogleSQL) scanner and parser that builds the shared
`sqlquack-syntax` tree. It follows GoogleSQL's grammar (`googlesql/parser/googlesql.tm` at
2026.11.1) and BigQuery's reserved keywords. It started as a copy of the Databricks crate, so
some doc comments still name the Spark grammar rules they came from.

`parse(sql)` returns every statement, or the error GoogleSQL's parser would report. `ParseError`
carries the message, the byte position and whether it is a syntax refusal (`Syntax error: ...`);
an unknown type name has `syntax = false`, since BigQuery refuses it in analysis.
`parse_strict(sql)` also refuses an input with a byte that no token, blank or comment covers.

## Covered

- Queries: `SELECT [DISTINCT | ALL] [AS STRUCT | AS VALUE]`, `* EXCEPT (...) REPLACE (...)`,
  backquoted and dashed project paths split into catalog, schema and table (a name with a
  backquoted dotted part, `` `proj.ds.t` ``, is `whole_quoted`, which GoogleSQL reads as a table only), every join kind,
  `UNNEST (...) [AS a] [WITH OFFSET [AS o]]`, `TABLESAMPLE SYSTEM`, `GROUP BY` with `ROLLUP`,
  `CUBE` and `GROUPING SETS`, `HAVING`, `QUALIFY`, `WINDOW`, `ORDER BY`, `LIMIT`, `OFFSET`,
  `UNION|INTERSECT|EXCEPT ALL|DISTINCT` with `[FULL|LEFT|INNER]`, `BY NAME` and
  `CORRESPONDING`, `WITH [RECURSIVE]`, `PIVOT`, `UNPIVOT` and the `|>` pipe operators.
- A pipe operator is a `PipeOperator` whose `this` is its input query and whose `expression` is
  a `( KEYWORDS operands... )` clause: `SELECT`, `EXTEND`, `WINDOW` and `AGGREGATE` hold a
  `Select` with no `FROM`; `SET`, `DROP` and `RENAME` a `Star` with `replace`, `except` or
  `rename`; `WHERE`, `ORDER BY`, `LIMIT [OFFSET]`, `AS`, `DISTINCT`, `JOIN`, `PIVOT`, `UNPIVOT`
  and `TABLESAMPLE` their operands. An `AGGREGATE` item or grouping key with `ASC`, `DESC` or
  `NULLS` is an `Ordered` over it, and `GROUP AND ORDER BY` adds a `GROUP AND ORDER BY`
  operand. They are not rewritten into standard clauses, which cannot keep a pipe step's table
  aliases, row order or value-table shape. A set operation after `|>`
  is the standard one with the input on the left. A `FROM` query alone is a `Select` with no
  select list, and `(query) AS alias` is `(query) |> AS alias`. `(query) ORDER BY ... LIMIT ...`
  on a query with an order or limit of its own is `query |> ORDER BY ... |> LIMIT ...`, which
  keeps the inner order and value-table shape that `SELECT * FROM (query)` would lose.
- Joins and commas bind left to right: an item after a join is a comma (`Implicit`) join in the
  chain. In a chain with consecutive `ON` or `USING` clauses, a `CROSS`, comma or `NATURAL` join
  takes the next from item at once and each condition closes the latest join still needing one,
  with the joins after it nested as its right side, as `join_processor.cc` builds them; such a
  condition is `deferred_condition` unless it directly follows its own join's right side. A
  `PIVOT` or `UNPIVOT` belongs to the one from item before it, never to a join, and
  `TABLESAMPLE` follows any from item and its alias as a `TableSample` over it. Set operations share one precedence, and
  a chain mixing operations or modifiers without parentheses is refused, as GoogleSQL does.
- An `UNPIVOT` label is a string, as a quoted alias, or an integer, as a bare one; GoogleSQL
  takes no other label. `OPTIONS` entries written with `+=` or `-=` hold a `( += value )` clause.
  A `CLUSTER BY` of bare columns is `ClusterByColumnsProperty`, and any other list a
  `ClusteredByProperty` of every expression.
- Expressions: string and bytes literals, kept apart, with GoogleSQL's escape and
  concatenation rules; typed literals (`DATE`, `TIME`, `DATETIME` and `TIMESTAMP` as literal
  nodes); arrays (`[...]`, `ARRAY(subquery)`), `STRUCT`, `INTERVAL`, `CASE`, `CAST`/`SAFE_CAST`
  with `FORMAT ... [AT TIME ZONE ...]`, `@param`, `@@variable`, `IN UNNEST`, quantified `LIKE`,
  `IS [NOT] UNKNOWN` apart from `IS NULL`,
  aggregate calls with `IGNORE NULLS`, `ORDER BY`, `LIMIT` and `HAVING MAX|MIN` inside, and
  `OVER` with named base windows and frames.
- A value whose type its syntax fixes, a typed literal with no literal node (`JSON`,
  `NUMERIC`, `BIGNUMERIC`, `RANGE<T>`) or a typed constructor (`ARRAY<T>[...]`,
  `STRUCT<...>(...)`), is a `Cast` whose `inferred_type` is its own target. No `CAST` has one,
  since GoogleSQL checks such a value at analysis, but a cast only when it is evaluated.
  Consumers must read `inferred_type` to tell a typed value from a `CAST` of the same operand.
- Paths and postfixes: number parts after a dot that follows a name, `)` or `]` (`foo.bar.25`,
  `a.1e2`, `f(x).1`, `a[0]. 1`, read as names), a dashed
  first part, ending in digits too (`my-proj-2.d.t`; a dash elsewhere is refused), field access and `.*` on any value (`f(x).*`), and
  chained calls (`x.f(args)`, `x.(path)(args)`) as `MethodCall`. `(query) AS alias` is
  the same tree as `(query) |> AS alias`; `FOR SYSTEM_TIME AS OF` follows a table's alias. GoogleSQL has no alias column lists (`AS t(a, b)`,
  `WITH c(a) AS`), no `col:path` and no `x COLLATE name`, so they are refused.
- `INSERT [INTO]`, `MERGE [INTO]` with `INSERT [(columns)] ROW`, `UPDATE ... FROM`,
  `DELETE [FROM]`, with `DEFAULT` as a value.
- `CREATE [OR REPLACE] [TEMP] TABLE` with columns, `NOT ENFORCED` keys, `PARTITION BY`,
  `CLUSTER BY`, `WITH CONNECTION`, `OPTIONS`, `LIKE`, `CLONE` and `COPY` (a source's
  `FOR SYSTEM_TIME AS OF` in `clone_at_clause`) and `AS query`; `CREATE [MATERIALIZED] VIEW`.
- The other `CREATE` kinds: `CREATE SCHEMA` as `CreateSchema`, and `[AGGREGATE] FUNCTION`,
  `TABLE FUNCTION`, `PROCEDURE`, `EXTERNAL SCHEMA`, `EXTERNAL TABLE`, `CONNECTION`,
  `SNAPSHOT TABLE | SCHEMA`, `[SEARCH | VECTOR | AI] INDEX`, `ROW ACCESS POLICY`, `DATA_POLICY`,
  `MODEL`, `CONSTANT`, `DATABASE`, generic entities and `MATERIALIZED VIEW ... AS REPLICA OF` as
  `CreateObject`: the scope in `modifier`, each clause a `( KEYWORDS operands... )` tuple in
  `operands`, and a routine parameter a tuple of its mode, name, type and its `AS`, `DEFAULT`
  and `NOT AGGREGATE` clauses. The shared `CreateFunction` and `CreateProcedure` have no place
  for `AGGREGATE`, `REMOTE`, a connection, the determinism levels or a parameter alias.
- `ALTER` of a table, a schema object kind, a generic entity, a row access policy, all row access
  policies or a data policy, each action a `( KEYWORDS operands... )` clause of `AlterObject`,
  with the grammar's own checks (`ALTER TABLE FUNCTION`, kinds `ALTER` does not take, `ON` only
  for index kinds).
- `DROP` of every kind: `DropTable`, `DropView`, `DropFunction` with its signature, and
  `DropObject` for the rest, with the grammar's checks (parameters only for `FUNCTION`, a drop
  mode only for `SCHEMA`, `ON` only for index kinds, no `DROP AGGREGATE FUNCTION`, no parameters
  for `DROP TABLE FUNCTION`); `UNDROP SCHEMA`; `TRUNCATE TABLE`.
- `GRANT` and `REVOKE` as `Grant` and `Revoke`: each privilege with its column paths (no
  privileges for `ALL [PRIVILEGES]`), the object kind words, the object path, and the grantees,
  a string's text with no `kind`, or `PARAMETER` (`@name`, `?`) or `SYSTEM_VARIABLE` (`@@name`)
  in `kind`.

## Gaps

A statement holding a construct the shared tree cannot hold becomes a `Command` with its text,
and `Parsed.gaps` names it with its byte range. A statement gap is named by its first word, such
as `declare`, `set`, `begin`, `if`, `loop`, `while` or `export`; script blocks are skipped to
their matching `END`. Other named gaps: `create` of a property graph, `graph` statements,
`label`, `graph_table`, `graph_subquery`, `match_recognize`, `generated_column`,
`proto_extension_field` (`x.(path)` with no call), `dot_star_modifiers` (`EXCEPT` or `REPLACE`
after `expr.*`),
`index_unnest` (an index's `UNNEST` paths), `table_source_where` and `table_clone_and_copy` (a
`CLONE` or `COPY` source's `WHERE`, or both), the drops that name a parent table
(`drop_row_access_policy`, `drop_all_row_access_policies`, `drop_index_on_table`: checked in
full, but the shared `DropObject` has no place for the table), `undrop_clauses`
(`IF NOT EXISTS`, `FOR SYSTEM_TIME AS OF` and `OPTIONS` on `UNDROP`), `truncate_where`,
`with_expression`, `select_with` (a `SELECT WITH <modifier>`), `aggregate_group_by_having` (an
aggregate's own `GROUP BY`), and the pipe forms `pipe_set_operation`, `pipe_call`,
`pipe_with` and `subpipeline_statement` (pipe operators with no input).

## Measured

- `corpora/bigquery-core` (934 files on `wip/bigquery-core`): 929 parse with no gap, 5 parse
  with named gaps (scripting and graph statements), 0 refused.
- The catalogue's 8,562 cases (`wip/bigquery-catalogue` a74f473e): of 3,278 accepts, 2,880 parse
  with no gap, 398 parse with named gaps (mostly graph, pipe, scripting and utility statements)
  and none is refused; of 5,281 rejects, 4,930 are refused, 349 are gaps and 2 parse. Those 2,
  `ALTER ORGANIZATION SET OPTIONS ( )` and `ALTER PROJECT ... SET OPTIONS ( )`, are held cases
  without a receipt. The grammar takes an empty options list for every generic entity, and the
  engine parsed `ALTER BI_CAPACITY ... SET OPTIONS ( )` and refused it only at analysis, so the
  parser does not refuse them.
