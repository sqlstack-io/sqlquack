# sqlquack-syntax-databricks

A Databricks SQL scanner and parser that builds the shared `sqlquack-syntax` tree. It follows
Apache Spark 4.2.0's ANTLR grammar (`SqlBaseLexer.g4` and `SqlBaseParser.g4`) with Spark's default,
non-ANSI keyword behaviour: no word is reserved, but the `strictNonReserved` words (`ANTI`, `CROSS`,
`EXCEPT`, `FULL`, `INNER`, `INTERSECT`, `JOIN`, `LATERAL`, `LEFT`, `MINUS`, `NATURAL`, `ON`,
`RIGHT`, `SEMI`, `UNION`, `USING`) cannot be bare identifiers such as table aliases.

`parse(sql)` returns every statement, or the error Databricks' parser reports. `ParseError` carries
the message, the byte position and whether it is a syntax refusal (`PARSE_SYNTAX_ERROR`, SQLSTATE
42601). Other parser checks, such as an unknown data type or typed literal, or the nesting limit,
have `syntax = false`; a typed literal whose text cannot be its value (`DATE 'm'`, `X'zz'`,
`INTERVAL 'm'`, a time or timestamp not starting as one) is `INVALID_TYPED_LITERAL`, a syntax
refusal. `parse_strict(sql)` returns `Result<Parsed, StrictFailure>` and also refuses
an input with a byte that no token, blank or comment covers.

Beyond the Spark grammar, the scanner takes Databricks' `?::` (try cast) operator, and the parser
takes Databricks' own additions and checks: `REPLACE TEMPORARY TABLE`, session temporary
metric views, a view's `COPY *` and `NOCOPY *`, `COPY INTO`, `OPTIMIZE`, `VACUUM`, `RESTORE`,
`UNDROP`, tags, masks, row filters and predictive optimization on `ALTER`, a primary key's
`TIMESERIES` column, `ENABLE` and `NOVALIDATE` after a key, column masks and row filters on
`CREATE TABLE`, and the refusals of `OR REPLACE` with `IF NOT EXISTS`, of `IF NOT EXISTS` and of
`TBLPROPERTIES` on a temporary view, of more than one `TIMESERIES` column, of `TBLPROPERTIES` or
`OPTIONS` keys without values, of `CLUSTER BY NONE` on `CREATE TABLE`, of a quoted (not `$$`) body
or of `HANDLER` without a language other than SQL after a function's parameter list, of `DROP INDEX`
(`INVALID_STATEMENT_OR_CLAUSE`), of an empty or blank-holding unquoted `REFRESH` path
(`INVALID_SQL_SYNTAX`), and of `SYSTEM SEQUENCE BY` without `STORED AS BITEMPORAL` in an `AUTO CDC`
flow, or the reverse.

Databricks' own statements the shared tree has no node for, such as those of catalogs, shares,
recipients, connections, credentials, external locations, volumes, governed tags, policies,
procedures, materialized views and streaming tables, `SET TAG` and `UNSET TAG`, are checked against
their documented syntax (`parse/_helpers/commands/documented.rs`, in the notation that
`parse/_helpers/tree/forms.rs` reads) and kept as a gap named for the statement.

## Covered

- Queries: `SELECT [DISTINCT | ALL]`, aliases, `*` and `t.*` with `EXCEPT`, multi-part names,
  aliases with column lists, every join kind including `LEFT SEMI`/`LEFT ANTI`, `NATURAL`, `USING`
  and `LATERAL`, `LATERAL VIEW [OUTER]`, table-valued functions, `VALUES`, subqueries, `GROUP BY`
  with `ALL`, `ROLLUP`, `CUBE` and `GROUPING SETS`, `HAVING`, `QUALIFY`, `WINDOW`, `ORDER`, `SORT`,
  `DISTRIBUTE` and `CLUSTER BY`, `LIMIT [ALL]`, `OFFSET`, set operations, `WITH [RECURSIVE]`,
  `PIVOT`, `UNPIVOT`, `MATCH_RECOGNIZE` (`PARTITION BY`, `ORDER BY`, `MEASURES`, `ONE ROW` /
  `ALL ROWS PER MATCH [SHOW EMPTY MATCHES]`, `AFTER MATCH SKIP PAST LAST ROW`, `PATTERN` with
  anchors, groups, alternation, `PERMUTE` and quantifiers, and `DEFINE`), `TABLESAMPLE`, hints and
  the `|>` pipe operators. The alias after `PIVOT`, `UNPIVOT` and `MATCH_RECOGNIZE` is an
  `identifier`, as in a select item or pipe `AS`, so it may be a strict-non-reserved word such as
  `ANTI` unless a join begins there; a table alias may not.
- Expressions: literals, including typed and suffixed ones and `INTERVAL`, `CASE`, `CAST`,
  `TRY_CAST`, `::`, calls with `DISTINCT`, `FILTER`, `IGNORE`/`RESPECT NULLS` and `OVER` with frames,
  lambdas, field, subscript and semi-structured `:path` access, the predicates, `EXISTS`, scalar
  subqueries, and the complex data types.
- `INSERT INTO`/`OVERWRITE`, `MERGE INTO`, `UPDATE`, `DELETE`, `COPY INTO` (`Copy`) and
  `LOAD DATA` (`LoadData`).
- `CREATE [OR REPLACE] TABLE`, `REPLACE TABLE`, `CREATE TABLE … LIKE` (a `Like` constraint),
  `CREATE TABLE … [SHALLOW | DEEP] CLONE` (`clone_source`, with a temporal clause),
  `CREATE [OR REPLACE] [TEMPORARY] VIEW`, `CREATE SCHEMA`, `CREATE [OR REPLACE] [TEMPORARY]
  FUNCTION` (SQL bodies, `AS $$…$$` bodies, `AS 'class' USING …`), `DROP TABLE`, `DROP VIEW`,
  `DROP SCHEMA`, `DROP FUNCTION`, `UNDROP TABLE | MATERIALIZED VIEW | VOLUME`.
- `ALTER TABLE` (`AlterTable`): `ADD COLUMN[S]`, `DROP COLUMN[S]`, `RENAME [COLUMN]`,
  `ALTER | CHANGE [COLUMN]` with `TYPE`, `COMMENT`, `SET | DROP NOT NULL`, `SET DEFAULT`,
  `DROP DEFAULT`, Hive's `CHANGE old new type`, `ADD | DROP PARTITION`, `PARTITION (…) SET
  LOCATION`, `ADD CONSTRAINT`, `DROP CONSTRAINT`, `SET | UNSET TBLPROPERTIES`, `SET | UNSET TAGS`,
  `SET LOCATION`, `CLUSTER BY`. `ALTER VIEW` (`AlterView`): `AS query`, `RENAME TO`,
  `SET | UNSET TBLPROPERTIES`, `[SET] OWNER TO`. `ALTER SCHEMA` (`AlterObject` of kind `SCHEMA`,
  its action naming the clause): properties, tags (named by identifiers, not strings), locations,
  collation, owner, `RETAIN DROPPED TO` and predictive optimization.
- `DESCRIBE` (`Describe`) of tables (`EXTENDED`, `FORMATTED`, `PARTITION`, `AS JSON`), queries,
  functions, procedures, schemas, `HISTORY`, `DETAIL` and the catalog objects; `EXPLAIN` (a
  `Describe` whose style is `EXPLAIN [mode]`, holding the statement). A parenthesized `DESCRIBE`
  in `FROM`, such as `FROM (DESCRIBE HISTORY t)`, is a `Subquery` holding the `Describe`.
- `SHOW` (`Show`) of tables, dropped tables, `TABLE EXTENDED`, views, schemas, catalogs,
  collations, functions, procedures, columns, `TBLPROPERTIES`, partitions, `CREATE TABLE`, the
  current schema, and `GRANTS` (the principal in `for_target`, the clause in `scope_kind` as the
  shared tree holds Snowflake's, such as `ON TABLE` or `TO RECIPIENT`, and the name in `scope`).
- `RESET [key]` (`ResetStatement`): the key as its name, such as `STATEMENT_TIMEOUT` or
  `spark.sql.ansi.enabled`, or no name for a bare `RESET`; text Spark's pattern does not read is
  refused. `EXPLAIN RESET` holds the same node.
- `FROM STREAM t` and `FROM STREAM(t)` are a `Stream` holding the relation read.
- `@name.method(args)`, a templating macro call left in the text (such as `@py.f(a)`), is a
  `MethodCall` on the column `@name` (a dotted `@name.a.f()` qualifies the column), the shape
  Polyglot gives it; its arguments are expressions.
- `SET` (`SetStatement`) of a configuration (matched as Spark matches the text after `SET`), the
  time zone, the path, the catalog and variables; `USE [CATALOG | SCHEMA | DATABASE]`;
  `COMMENT ON`; `TRUNCATE TABLE`; `GRANT` and `REVOKE`; `DECLARE [OR REPLACE] [VARIABLE]`;
  `CACHE [LAZY] TABLE` and `UNCACHE TABLE`.

## Gaps

A statement holding a construct the shared tree cannot hold becomes a `Command` with the
statement's text, and `Parsed.gaps` names each construct with its byte range. A gap statement is
accepted as opaque, never as parsed. For a statement gap (its first keyword), the text up to the
next top-level `;` is not checked further; every other gap's text is checked.

| Gap | Construct |
| --- | --- |
| a statement's first keyword, such as `analyze`, `reorg` or `call` | every statement not listed under Covered or below, such as `ANALYZE`, `REORG`, `CALL`, cursors and transactions |
| the statement's name, such as `create_catalog`, `alter_share`, `drop_policy`, `create_streaming_table`, `refresh_table` or `set_tag` | a Databricks statement with no shared-tree node, checked against its documented syntax: the documented `ALTER`, `CREATE` and `DROP` statements of catalogs (and foreign catalogs), connections, servers, credentials, external locations, governed tags, materialized views, streaming tables, policies, procedures, providers, recipients, shares and volumes; `DROP BLOOMFILTER INDEX`, `CONVERT TO DELTA`, `REFRESH FOREIGN`, `REFRESH MATERIALIZED VIEW`, `REFRESH [STREAMING] TABLE`, `REFRESH FUNCTION`, `SET TAG` and `UNSET TAG` |
| `analyze`, `fsck`, `generate`, `sync`, `reorg`, `call`, `clear_cache`, `list`, `execute_immediate`, `drop_variable` | `ANALYZE`, `FSCK REPAIR TABLE`, `GENERATE`, `SYNC` (a `SYNC TABLE` whose source is named other than its target is refused), `REORG`, `CALL`, `CLEAR CACHE`, `LIST path`, `EXECUTE IMMEDIATE`, `DROP TEMPORARY VARIABLE`: checked against their documented syntax, no shared node |
| `describe_policy`, `describe_credential`, `describe_location`, `describe_governed_tag` | `DESCRIBE POLICY`, `DESCRIBE {SERVICE \| STORAGE} CREDENTIAL`, `DESCRIBE EXTERNAL LOCATION`, `DESCRIBE GOVERNED TAG`: checked, no shared node |
| `show_policies`, `show_shares`, `show_all_in_share`, `show_groups`, `show_principals`, `show_volumes`, `show_statistics`, `show_governed_tags`, `show_securables` | the documented `SHOW` statements the shared `Show` does not hold: policies, shares, `ALL IN SHARE`, groups, users, providers, recipients, volumes, statistics, governed tags, connections, credentials and external locations; checked. Any other `SHOW` is refused, as Spark refuses it |
| `compound`, `signal`, `open_cursor`, `fetch_cursor`, `close_cursor` | `BEGIN … END` blocks and their labelled `LOOP`, `WHILE`, `REPEAT`, `FOR`, `IF`, `CASE`, `LEAVE`, `ITERATE`, handlers and conditions, `SIGNAL`, `RESIGNAL` and the cursor statements: checked against Spark's `compoundBody` (a whole block takes no label; `GET DIAGNOSTICS` is refused, as Databricks refuses it) |
| `add`, `list` | `ADD` and `LIST` of a file, jar or archive (another resource type is refused) |
| `deny`, `cache_select` | `DENY privileges ON securable TO principal` (read as `GRANT`), Databricks' `CACHE SELECT …` (its text unchecked but a string after `FROM`) |
| `refresh` | `REFRESH path`, checked as Spark checks the resource path |
| `alter_owner` | `ALTER name [SET] OWNER TO principal`, a securable named without its kind |
| `optimize`, `vacuum`, `restore`, `repair`, `msck` | `OPTIMIZE`, `VACUUM`, `RESTORE`, `[MSCK] REPAIR TABLE`: checked, but the shared tree has no node |
| `alter`, `create`, `drop`, `describe` | those statements of any other object kind with no documented form above, such as a group |
| `set` | `SET ROLE`, `SET RECIPIENT name`, `SET COLLATION name` and `SET QUERY_TAGS[key] = value`; any other `SET` text Spark's patterns do not read is refused (`INVALID_SET_SYNTAX`), as are `SET QUERY_TAGS` without a key, `SET PATH` with a one-part schema and `SET TIME ZONE` with no zone |
| `row_format`, `stored_as`, `bucket_spec`, `skew_spec` | the Hive table clauses |
| `temporary_view_using`, `metric_view`, `view_nocopy` | `CREATE TEMPORARY VIEW … USING`, `CREATE VIEW … WITH METRICS`, a view's `NOCOPY *` |
| `schema_binding`, `view_collation`, `view_partitioned_on` | `WITH SCHEMA …`, `DEFAULT COLLATION`, `PARTITIONED ON` on a view |
| `drop_table_force`, `drop_temporary_table`, `drop_temporary_function` | `DROP TABLE … FORCE`, `DROP TEMPORARY TABLE`, `DROP TEMPORARY FUNCTION` |
| `insert_options`, `insert_overwrite_directory`, `insert_replace_on`, `insert_replace_using`, `schema_evolution` | `INSERT … WITH (options)`, `INSERT OVERWRITE [LOCAL] DIRECTORY`, `REPLACE ON`/`REPLACE USING`, `WITH SCHEMA EVOLUTION` |
| `copy_columns`, `copy_by_position`, `copy_target_credential` | `COPY INTO t (columns)`, `COPY INTO t BY POSITION`, a credential on `COPY INTO`'s target |
| `multi_select_from` | `FROM … INSERT …` / `FROM … SELECT …` with more than one body |
| `changes`, `relation_options`, `watermark`, `identified_by`, `nearest_by`, `subquery_sample`, `parenthesized_relation_sample` | `CHANGES`, `WITH (options)` on a relation, `WATERMARK … DELAY OF INTERVAL …` (checked), `IDENTIFIED BY`, `NEAREST BY`, a sample or later relation clause after a parenthesized subquery or relation |
| `table_argument_partitioning` | `TABLE t PARTITION BY … ORDER BY …` as a function argument |
| `pivot_after_lateral_view`, `identifier_clause_function` | `PIVOT` after `LATERAL VIEW`, `IDENTIFIER(…)(…)` calls |
| `owner`, `managed`, `row_filter`, `column_mask`, `column_tags`, `view_tags`, `view_constraint` | `ALTER TABLE … [SET] OWNER TO`, `SET \| UNSET MANAGED` and `SET EXTERNAL`, row filters (on `ALTER TABLE` and `CREATE TABLE … WITH ROW FILTER`), column masks (on added and created columns), column and view tags, view constraints |
| `cluster_by_expression` | `CLUSTER BY ('text', …)` on `CREATE TABLE`: an expression, parsed and refused later than the parser |
| `drop_feature`, `drop_primary_key`, `drop_foreign_key`, `drop_constraint_cascade`, `drop_partition_purge` | `DROP FEATURE`, `DROP PRIMARY KEY`, `DROP FOREIGN KEY`, `DROP CONSTRAINT … CASCADE`, `DROP PARTITION … PURGE` |
| `constraint_validation` (also after a primary or foreign key in `CREATE TABLE`), `predictive_optimization`, `identity_sync`, `table_collation`, `unset_if_exists` | `ENABLE`/`NOVALIDATE` after an added constraint, `{ENABLE \| DISABLE \| INHERIT} PREDICTIVE OPTIMIZATION` on a table, `SYNC IDENTITY`, a table's `DEFAULT COLLATION`, `UNSET TBLPROPERTIES IF EXISTS` |
| `alter_column_position`, `change_column_position`, `change_column_not_null`, `replace_columns`, `replace_partitioning`, `recover_partitions`, `rename_partition`, `serde`, `partition_change_column`, `view_partition` | column moves, `REPLACE COLUMNS`, `REPLACE PARTITIONED BY WITH CLUSTER BY`, `RECOVER PARTITIONS`, partition renames, SerDe clauses, Hive column changes in a partition, partitions of a view |
| `describe_column`, `show_partition`, `show_create_table_serde` | `DESCRIBE t column`, `PARTITION (…)` on `SHOW`, `SHOW CREATE TABLE … AS SERDE` |
| `declare_cursor`, `undrop_with_id`, `function_credentials`, `function_secrets` | `DECLARE … CURSOR`, `UNDROP … WITH ID`, a function's `CREDENTIALS` and `SECRETS` |

## Lossy mappings

These parse without a gap, but the shared tree keeps less than the source said:

- Suffixed numbers (`1L`, `2BD`) become a `Cast` of the number to the suffix's type.
- `IS [NOT] DISTINCT FROM` becomes `NullSafeEq` (negated for `DISTINCT`), `RLIKE` and `REGEXP`
  become the `rlike` call, `DIV` becomes the `div` call, and `?::` becomes `TryCast`.
- `ROLLUP`, `CUBE` and `GROUPING SETS` become calls of those names; `WITH ROLLUP` and `WITH CUBE`
  become the same calls.
- `MINUS` becomes `EXCEPT`.
- A set operation or pipe operator that its body cannot hold, followed by `ORDER BY` or `LIMIT`,
  becomes `SELECT * FROM (body)`.
- Pipe `SET` and `DROP` become a `Star` with `replace` and `except`.
- `TIMESTAMP_LTZ` and `TIMESTAMP_NTZ` literals become a `Cast` to a custom type; collations on
  string types are dropped.
- `COPY *` on a view becomes `copy_grants`.
- A function's `RETURNS TABLE (columns)` keeps the column list as `returns_table_body` text; its
  `COMMENT`, `SPECIFIC` and `DEFAULT COLLATION` become `options` properties, and parameter
  comments and `NOT NULL` are dropped.
- `SHOW TBLPROPERTIES t (key)` holds the key as a string `scope` of `scope_kind` `PROPERTY`: the
  shared `Show` has no field for a property key.
- `GRANT`, `REVOKE`, `CACHE` and `UNCACHE` keep their object's name as one dotted identifier.
- A `SET` configuration value, and a time zone that is neither a string nor `LOCAL`, become `Var`
  text.
- A `MATCH_RECOGNIZE` pattern is kept as its source text, after it is checked.
- `ALTER SCHEMA … [SET] RETAIN DROPPED TO n unit` keeps `n unit` as one `Var` operand.

## Implementation notes

- Form patterns (`parse/_helpers/tree/forms.rs`) are space-separated sequences of: an upper-case
  word, matched without case; a punctuation token such as `(` or `=`; a lower-case slot naming a
  grammar rule (see `slot`); `[ … ]` for an optional part, `{ … }` for a group, `|` between
  alternatives, and a `...` or `,...` after an item for one or more of it, bare or
  comma-separated. Matching follows every reading at once, as a set of token positions, so an
  optional part never hides a later one. When no pattern reads a statement, the furthest failure
  is reported.
- The scanner has no `>>` or `>>>` operator: `>` is always one token, as Spark's lexer makes it
  inside `ARRAY<…>`, `MAP<…>` and `STRUCT<…>`, and the grammar joins adjacent `>`s into a shift.
  `REGEXP` is the same token as `RLIKE`. A doubled quote stands for one quote; where backslash
  escapes apply, a backslash escapes the byte after it.
- `MINUS` among the `strictNonReserved` words is the lexer's `SETMINUS`.
- `queryTerm` precedences, all left-associative: pipe operators (1), `UNION`, `EXCEPT` and `MINUS`
  (2), and `INTERSECT` (3). A pipe operator extends its input itself only when the input is a plain
  star select (for some operators, also one without `WHERE`).
- In `OPTIONS (…)` on `CREATE TABLE`, adjacent strings are the key and its value unless `=`
  follows them. A column's reference may also be written `FOREIGN KEY REFERENCES`.
- A view's `COPY *` means a replaced view keeps its grants, comments and tags.
- In `WATERMARK`, the `namedExpression`'s alias cannot be the `DELAY` that begins `DELAY OF`.
- `functionTableArgument` tries `TABLE ( multipartIdentifier )` first, so `TABLE (SELECT)` names a
  table.
- A `MATCH_RECOGNIZE` `PATTERN` term is a variable, `^`, `$`, a group or `PERMUTE (…)`, each with an
  optional quantifier.
- A function's characteristics may come before or after its body. After a parameter list, the body
  is a `$$` string, and `HANDLER` names a function of a language other than SQL.
- `BEGIN TRANSACTION` and `BEGIN WORK` start a transaction rather than a compound block.
- `DESCRIBE HISTORY` and `DESCRIBE DETAIL` of a quoted path name a table by its location.
  `USE CATALOG 'name'` names the catalog by a string.
- `SET PATH = …` text that the path elements do not read to its end is a configuration, as in
  Spark.
- In `GRANT … ON VIEW TO u`, a kind word followed by `TO` or `FROM` and then a principal is the
  securable's name.
