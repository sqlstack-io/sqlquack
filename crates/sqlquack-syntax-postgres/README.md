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
| a statement's first keyword, such as `create` or `alter` | every statement other than `SELECT`, `VALUES`, `TABLE`, `INSERT`, `UPDATE`, `DELETE`, `MERGE`, `TRUNCATE`, `COPY`, `COMMENT`, `SECURITY LABEL`, `CREATE [OR REPLACE] FUNCTION` or `PROCEDURE`, `CREATE TABLE` and `CREATE TABLE AS`, `CREATE [OR REPLACE] AGGREGATE`, `CREATE DATABASE`, `CREATE [OR REPLACE] [MATERIALIZED] VIEW`, `CREATE INDEX`, `SEQUENCE`, `TRIGGER`, `TYPE` and `FOREIGN TABLE`, the catalog `CREATE` statements listed under their own heading, and every `ALTER` other than `ALTER DEFAULT PRIVILEGES` |
| `copy_where` | `COPY … FROM … WHERE condition` |
| `json`, `json_array`, `json_arrayagg`, `json_exists`, `json_object`, `json_objectagg`, `json_query`, `json_scalar`, `json_serialize`, `json_value`, `json_table` | the SQL/JSON call syntax |
| `xmlconcat`, `xmlelement`, `xmlexists`, `xmlforest`, `xmlparse`, `xmlpi`, `xmlroot`, `xmlserialize`, `xmltable` | the SQL/XML call syntax |
| `treat`, `merge_action` | `TREAT (… AS type)`, `MERGE_ACTION()` |
| `is_json`, `is_document`, `is_normalized` | `IS [NOT] JSON …`, `IS [NOT] DOCUMENT`, `IS [NOT] [form] NORMALIZED` |
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

## CREATE TABLE

`CREATE TABLE` and `CREATE TABLE AS` become `CreateTable`. Columns go in `columns`, typed-table and
`AS` columns with no type. Each column's storage, compression, options, collation and
constraints are keyword tuples in `ColumnDef.options`, in source order, such as
`(NOT NULL, NO INHERIT)` or `(CONSTRAINT, c, REFERENCES, p, (ON DELETE, SET NULL, (a)))`. Table
constraints and `LIKE` are `(ELEMENT, n, tuple)` in `properties`, where `n` counts the columns before
them. `OF`, the partition bound, `PARTITION BY`, `USING`, `WITH`, `WITHOUT OIDS`, `ON COMMIT` and
`TABLESPACE` are keyword tuples there too. `UNLOGGED` is the `table_modifier`. The grammar's own
errors are kept: repeated `COLLATE`, `GENERATED BY DEFAULT AS (…)`, conflicting constraint
attributes and missing hash bounds (42601); attributes a constraint kind cannot take,
`MATCH PARTIAL` and an `ON UPDATE` column list (0A000); an unknown partitioning strategy (22023);
and a repeated hash bound (42710).

## CREATE AGGREGATE

`CREATE AGGREGATE` becomes a `CreateObject` of kind `AGGREGATE`. Its argument list is the call in
`operands`, as `COMMENT ON AGGREGATE` names it, and each definition element is a property whose
value is absent when written bare. The pre-8.2 form has no argument list and the source
`OLD STYLE`. An `OUT` or `INOUT` argument, and a `VARIADIC` direct argument without one `VARIADIC`
aggregated argument of the same type name, are refused (0A000). Type names are compared as
PostgreSQL's `equal()` compares them, so `int` matches `integer` and `pg_catalog.int4` but not `int4`.

## CREATE DATABASE

`CREATE DATABASE` becomes `CreateDatabase`, each option a `(name, value)` tuple in `properties`, in
order. `CONNECTION LIMIT` is named `connection_limit`, as PostgreSQL names it; a value is a number,
a string or word, a boolean, or `DEFAULT`.

## CREATE VIEW, INDEX, SEQUENCE, TRIGGER, TYPE and FOREIGN TABLE

`CREATE VIEW` and `MATERIALIZED VIEW` become `CreateView`. `UNLOGGED`, `RECURSIVE`, `WITH (…)`,
`USING`, `TABLESPACE`, `WITH NO DATA` and `(CHECK OPTION, LOCAL | CASCADED)` go in `options`;
`WITH DATA` and a bare `WITH CHECK OPTION` are the defaults PostgreSQL gives them. A recursive view
with a check option is refused (0A000), as gram.y refuses it. `CREATE INDEX`, `SEQUENCE`, `TRIGGER`
and `TYPE` become `CreateObject`, with the persistence or the `UNIQUE`, `CONSTRAINT` or `RECURSIVE`
word in `modifier` and each clause a keyword tuple in `operands`, in grammar order. An index without
`USING` records `btree`, and `NULLS DISTINCT` is the default. Trigger events are listed in
PostgreSQL's bit order (`INSERT`, `DELETE`, `UPDATE`, `TRUNCATE`), `FOR EACH STATEMENT` is the
default, `EXECUTE PROCEDURE` equals `EXECUTE FUNCTION`, and each function argument is the string
gram.y makes of it. A sequence's options are those of `ALTER SEQUENCE`. `CREATE TYPE` records `AS`,
`AS ENUM` or `AS RANGE` in `source`, with the attributes or labels in `operands` and a definition
list in `properties`. `CREATE FOREIGN TABLE` becomes `CreateTable` with the `table_modifier`
`FOREIGN` and `(SERVER, s)` and `OPTIONS` in `properties`. `CREATE ASSERTION` is parsed and refused
(0A000). `CREATE` followed by a word no statement allows is refused (42601).

## Catalog CREATE statements

`CREATE ACCESS METHOD`, `CAST`, `COLLATION`, `[DEFAULT] CONVERSION`, `DOMAIN`, `EVENT TRIGGER`,
`EXTENSION`, `FOREIGN DATA WRAPPER`, `LANGUAGE`, `OPERATOR`, `OPERATOR CLASS`, `OPERATOR FAMILY`,
`PUBLICATION`, `RULE`, `SCHEMA`, `SERVER`, `STATISTICS`, `SUBSCRIPTION`, `TABLESPACE`, `TEXT SEARCH`
and `TRANSFORM` become `CreateObject`, with each clause a keyword tuple in `operands`, in grammar
order, and a definition list in `properties`. `DEFAULT` (conversions, operator classes) and `TRUSTED`
(languages) are the `modifier`. gram.y's rewrites are kept: `CREATE [OR REPLACE] LANGUAGE l` without
`HANDLER` is `CREATE EXTENSION [IF NOT EXISTS] l`; `PROCEDURAL`, `NO VALIDATOR`, `FOR SEARCH`,
`DO ALSO`, `CREATE SERVER ... VERSION NULL` and empty rule actions add nothing; a domain's `COLLATE`
comes before its constraints; a transform lists `FROM SQL` before `TO SQL`; and `COLLATION ... FROM`
is its `from` element. Publication objects take their kind as in `ALTER PUBLICATION`. A rule action is
a query, `INSERT`, `UPDATE`, `DELETE` or `(NOTIFY, channel [, payload])`. A schema element is
`CREATE TABLE`, `INDEX`, `SEQUENCE`, `TRIGGER` or `VIEW`; a `GRANT` element is a `grant` gap until
grants parse. `CREATE EXTENSION ... FROM` and schema elements after `IF NOT EXISTS` are refused
(0A000), as are `OR REPLACE` and a persistence where gram.y has none (42601).

## CREATE ROLE, USER, GROUP, POLICY and USER MAPPING

`CREATE ROLE`, `USER` and `GROUP` become `CreateObject` of their own kind, each option a
`(name, value)` tuple in `operands`, in order, named as gram.y's `DefElem` names it: `NOLOGIN` is
`(canlogin, false)`, `ENCRYPTED PASSWORD` is `password`, `IN GROUP` is `addroleto`, `USER` and `ROLE`
are `rolemembers`. An unknown option (42601), `UNENCRYPTED PASSWORD` (0A000) and a reserved role name
(42939) are refused. `CREATE POLICY` writes gram.y's defaults out: `(AS, PERMISSIVE)`, `(FOR, ALL)`
and `(TO, (PUBLIC))`. `CREATE USER MAPPING FOR USER` is `FOR CURRENT_USER`.

## ALTER

`ALTER TABLE`, `FOREIGN TABLE`, `INDEX`, `SEQUENCE`, `VIEW` and `MATERIALIZED VIEW` become
`AlterObject`, with the relation kind in `kind` and the relation in `name` (`ONLY` and `*` as
`TableRef` keeps them). Each `alter_table_cmd` is a keyword tuple in `operands`, in source order,
such as `(ADD COLUMN, columnDef)` or `(ALTER COLUMN, c, (SET STATISTICS, 16))`, with an empty
`action`. A rename, `SET SCHEMA`, `[NO] DEPENDS ON EXTENSION`, `ATTACH` or `DETACH PARTITION`,
`ALL IN TABLESPACE` and a sequence's `SeqOptList` name themselves in `action`, their values in
`operands`. Numbers are kept as values, so `SET STATISTICS 0x10` equals `SET STATISTICS 16`.
`ALTER FUNCTION`, `PROCEDURE`, `ROUTINE` and `AGGREGATE` also become `AlterObject`. The signature,
as `COMMENT ON` builds it, is the first operand and its name parts are `name`, so `a.b.c.d` keeps
`a.b` as the catalog. Options are tuples, as in `CREATE FUNCTION`, with an empty `action`; a rename, a
new owner, `SET SCHEMA` and `[NO] DEPENDS ON EXTENSION` name themselves in `action`. `RESTRICT` changes
nothing, as in PostgreSQL. In every routine, the special `SET` and `RESET` forms are the settings
PostgreSQL's raw tree makes of them: `SET TIME ZONE 'x'` is `SET timezone = 'x'`, `LOCAL` and
`DEFAULT` set the default, and `SCHEMA`, `NAMES`, `ROLE`, `SESSION AUTHORIZATION` and `XML OPTION`
set `search_path`, `client_encoding`, `role`, `session_authorization` and `xmloption`.
`SET CATALOG` is refused (0A000), as is a time zone interval with fields beyond `HOUR` and `MINUTE`
(42601). The other object kinds also become `AlterObject`, named as their grammar names them: one name, an
`any_name`, `name ON table` (`TRIGGER`, `RULE`), or an operator signature or large object number in
`operands` with an empty `name`. `OPERATOR CLASS` and `FAMILY` keep `(USING, method)`. Each kind's
own action names itself in `action`: enum values, attribute commands, domain constraints, mappings,
publication objects, subscription publications, foreign-data options, operator family members,
`SET STATISTICS`, `REFRESH VERSION` and settings. Publication objects take the kind before them, as
`preprocess_pubobj_list` gives it, so `TABLE a, b` equals `TABLE a, TABLE b`, and its errors are kept
(42601). In `operator_def_list`, `NONE` and a bare name both have no argument. `DROP VALUE` (0A000)
and missing kind words, such as `ALTER TEXT SEARCH t`, are refused. `ALTER` followed by a word no statement kind allows is refused (42601); other `ALTER` statements
stay opaque gaps. The grammar's own errors are kept, including `NOT VALID` on `ALTER CONSTRAINT`
(0A000), a column number out of range, and identity options `SET` cannot take.

## ALTER ROLE, USER, GROUP, POLICY, SYSTEM and USER MAPPING

`ALTER ROLE`, `USER` and `GROUP` make one `AlterRoleStmt` or `RenameStmt`, so all three are
`AlterObject` of kind `ROLE`. A role name is `name`; `ALL`, `PUBLIC` and the `CURRENT_*` words are
the first operand with an empty `name`. Options are the `(name, value)` tuples of `CREATE ROLE`, with
an empty `action`, so `ALTER GROUP g ADD USER a` equals `ALTER ROLE g USER a`; `DROP USER` has
`action` `DROP`. `[IN DATABASE d] SET` and `RESET` are `(IN DATABASE, d)` and the setting tuple of
`ALTER DATABASE`. A rename names itself in `action`, and a `RoleSpec` that is not a role name is
refused there (42939). `ALTER POLICY` keeps `(ON, table)`, then `TO`, `USING` and `WITH CHECK` as
written, with no defaults; only a rename takes `IF EXISTS`. `ALTER SYSTEM` takes `generic_set` and
`generic_reset` only, so `FROM CURRENT` and the special forms are refused (42601). `ALTER USER
MAPPING FOR USER` is `FOR CURRENT_USER`, and its `OPTIONS` are a `(OPTIONS, ...)` tuple.
