# sqlquack-syntax-duckdb

**Work in progress.** A DuckDB 2.0.0 scanner and parser that builds the shared
`sqlquack-syntax` tree. It follows DuckDB's PEG parser at `v2.0.0`
(`d54cbbe3df17e8e5ce68312f483b4155836112aa`): `src/parser/peg/tokenizer` and
`src/parser/peg/grammar`. The crate's layout follows `sqlquack-syntax-databricks`.

## Done

- `scan`: the PEG tokenizer (`base_tokenizer.cpp` with `parser_tokenizer.cpp`'s refusals):
  words, `"` and `` ` `` quoted identifiers, plain, `E`, `N`, `X` and `B` strings, `$tag$`
  strings, numbers with `_` separators and exponents (`0xFF` is the number `0` and the word
  `xFF`, as in DuckDB), operator runs with PostgreSQL's trailing `+`/`-` rule, `::` and `:=`,
  the single-character tokens, nested block comments and line comments. `uncovered` reports a
  byte no token, blank or comment covers.
- `keywords`: the six keyword lists of the grammar (`grammar/keywords/*.list`).

- `parse`: queries and expressions by recursive descent over `grammar/statements/*.gram`, with
  the checks DuckDB's transformer makes while parsing (argument counts of the calls it rewrites,
  an empty selection list, chained comparisons, `ORDER BY ALL` in a window, an empty
  `CUBE`/`ROLLUP`, constant type modifiers, mixed named and positional parameters). It covers `SELECT` and `FROM`-first
  queries, CTEs (`USING KEY`, with entries other than bare columns in `key_items`,
  `MATERIALIZED`), set operations with `BY NAME`, joins (`ASOF`,
  `POSITIONAL`, `NATURAL`, `LATERAL`, with `NATURAL SEMI|ANTI` and `ASOF FULL|SEMI|ANTI` as their
  own kinds), table functions (`WITH ORDINALITY`), samples after any
  `FROM` item, nested schema paths (each catalog part kept with its quoting in `catalog_parts`),
  a table's time travel `AT (VERSION|TIMESTAMP => value)` in its `when` (`HistoricalData`, as
  Snowflake's `AT` is kept),
  `PIVOT`/`UNPIVOT` in `FROM` and as statements (`PIVOT_WIDER`, `PIVOT_LONGER`,
  `INTO NAME ... VALUE`; an enum target is marked `is_field`, and an `ON` entry that is an `IN`
  test is parenthesized so it is not read as a value list), set-operation `LIMIT n%` and `FETCH`
  without a count (one row), lists with a trailing
  comma (`List` in `common.gram`), `DESCRIBE` and `SUMMARIZE` of a query or a table (with the
  refusals of a lone describe with a CTE or result modifiers), `SHOW` and the table listings
  (a `Show` node: `ALL TABLES` for a bare `SHOW` or `DESCRIBE`, the special listing's name for
  `TABLES`, `DATABASES`, `SCHEMAS` or `VARIABLES`, `TABLES` with `from` for `TABLES FROM`, and
  `SETTING`, `TABLE` or `QUERY` with the `target` for a one-part name or string, a qualified
  name and a query), star `EXCLUDE`/`REPLACE`/`RENAME` (qualified names kept in `qualified`, with the refusals of a duplicate entry, a column two
  lists name and more than a table before the star), `COLUMNS`, lambdas, list comprehensions,
  struct and map literals, slices (refusing `[]`), `OPERATOR(...)` as an infix and a prefix
  operator, `IGNORE NULLS` only in a window, the regular-expression operators (`~` and `!~` as
  `regexp_matches`, with an `ESCAPE` as its third argument), `POSITION(a IN b)` as
  `position(b, a)`, `OVERLAY` with `PLACING` or a plain argument list, `COLLATE` with a
  constant or a dotted name (its parts joined by `.`, as DuckDB names it; any other expression,
  which DuckDB refuses as not implemented, has an empty name), sample row counts
  rounded half to even and compared exactly, named arguments and macro parameters named by
  any word but a reserved one, a qualified aggregate's name parts and quoting
  (`qualified_name`, `quoted`), `DISTINCT` and an argument `ORDER BY` in a method call,
  `EXPORT_STATE` (`export_state`), user type names with their parts quoted where needed and
  their modifiers written out in `Custom.name` (as the Postgres parser keeps them), `ENUM('a',
  ...)` as an `Enum` type, and `CREATE MACRO` as a function, its further overloads in
  `CreateFunction.overloads` (refusing table and scalar overloads together). The transformer's refusals are
  kept: `DISTINCT` or `ALL` with no argument, `EXPORT_STATE` with `OVER`, a window function in
  a window definition, `WITHIN GROUP` with `DISTINCT` or more than one ordering, a named
  argument to a call it rewrites (`IFNULL`, `TRY`, `UNPACK`, `construct_array`), a `unit TO
  unit` interval, and `ANY`/`ALL` after an operator that is no comparison.
- DML: `INSERT`, `UPDATE`, `DELETE` and `MERGE` as trees. `MERGE` takes every clause kind
  (`MATCHED`, `NOT MATCHED [BY TARGET]`, `NOT MATCHED BY SOURCE`) and action (`UPDATE` with
  `SET`, `SET *` or `BY NAME|POSITION`, `INSERT` with values, `DEFAULT VALUES` or `*`,
  `DELETE`, `DO NOTHING`, `ERROR`), with the refusal of a clause an unconditional one of its kind
  makes unreachable. `INSERT ... ON CONFLICT` is an `OnConflict` node (the conflict columns,
  their `WHERE`, `ON CONSTRAINT`, and `DO NOTHING` or `DO UPDATE SET` with its `WHERE`), refusing
  it after `OR REPLACE|IGNORE`; `OVERRIDING USER|SYSTEM VALUE` is kept in `overriding`. A
  `DELETE ... USING` list of plain tables is kept in `using`, and any other list (subqueries,
  table functions, joins) in `using_items`. A CTE body is a query, `INSERT`, `UPDATE`, `DELETE`,
  `TRUNCATE` or `COPY`, which may start with its own `WITH`; others are refused as the
  transformer refuses them.
- DDL: `CREATE TABLE` (columns, constraints, generated columns, `WITH` options, `AS` query),
  `CREATE VIEW`, a column `DEFAULT` read as `ColumnDefaultExpr` (`OR` and `AND` only, so
  `DEFAULT 1 NOT NULL` keeps its constraint),
  `CREATE SEQUENCE`, `CREATE TYPE`, `CREATE SCHEMA`, `CREATE INDEX` (its keys as `columns` when
  every key is a bare column, otherwise every key as an expression in `key_parts`) and
  `CREATE TRIGGER` as trees, and `DROP` of tables, views, macros, functions, types, schemas, indexes, sequences and
  triggers (`DROP INDEX`, and `DROP VIEW` with `CASCADE` or `RESTRICT`, as a `DropObject`
  keeping them; `CREATE TEMP
  TYPE` keeps `temporary`). A recursive CTE refuses a DML or `COPY` body, a column
  `CONSTRAINT name` needs its constraint, and `START WITH` refuses a subtraction. A table option keeps its name and its value as written; a string or number value
  stands alone, as `DefArg` takes it. `CREATE SECRET`, extension repositories, external
  resources, `DROP SECRET`, table macros, table layouts (`PARTITIONED|SORTED BY`), view and index
  options and a trigger's statement body are gaps. The transformer's refusals are kept with its
  messages: sequence bounds and folded option values, index and view options, `REFERENCING` aliases, and the column checks.
- `ALTER`: `ALTER TABLE` (add, drop, alter and rename columns, add constraints, rename the table),
  `ALTER TABLE ... SET|RESET (options)`, `ALTER VIEW ... RENAME` and
  `ALTER SEQUENCE ... OWNED BY` as trees, with a struct field dropped or renamed as `DropField`
  or `RenameField` by its names; adding or altering a struct field, table layouts
  (`SET PARTITIONED|SORTED BY`), `ALTER DATABASE` and schema options are gaps. `ALTER TABLE` takes
  one action, and the refusals of the forms DuckDB does not implement are kept.
- `COPY` of a table (with a column list, `FROM` or `TO`) or a query (`TO`), the file as a string,
  a parameter, an expression or a dotted name, `USING DELIMITERS`, and generic or specialized
  options, with the transformer's refusals of a repeated option, a list option without its list
  and a `FORMAT` without a value. Any option value expression is kept: one the transformer cannot
  keep, such as `CASE`, a comparison or `coalesce(...)` outside `PARTITION_PATH` and `ORDER_BY`,
  is refused by DuckDB only after parsing, as not implemented. Also `COPY FROM DATABASE`, and
  `COPY FROM` refused as a CTE body.
- `PRAGMA` as a call or an assignment, `TRUNCATE` (with an alias and a drop behaviour) and `USE`
  as trees, with the transformer's refusals: one value in an assignment, a named parameter
  without a column on its left, `TRUNCATE ... CASCADE`, and more than `database.schema`.
  `RESET` is a `ResetStatement` (`VARIABLE` or the setting scope as its scope), refusing
  `RESET LOCAL`.
- `INSTALL` (forced or not, from a repository name or URL) as a tree; `INSTALL AND LOAD`, a
  `VERSION`, `LOAD` and `UPDATE EXTENSIONS` are parsed strictly and kept as gaps, as the tree
  has no node for them. `[FORCE] CHECKPOINT [catalog]` is the `Call` DuckDB makes it, of
  `checkpoint` or `force_checkpoint` with the catalog as a string.
- Utility statements: `ATTACH` (path, alias and options; `OR REPLACE` is a gap), `DETACH`, `SET`
  (`SCHEMA`, a variable or setting with its scope, `TIME ZONE`; an interval time zone is a gap),
  `BEGIN`/`START`, `COMMIT`/`END` and `ROLLBACK`/`ABORT` with their modes, and `COMMENT ON` of
  every kind. The transformer's refusals are kept: `SET LOCAL`, more than one `SET` value,
  comments on databases and schemas, and over-qualified or unqualified column references.
- `PREPARE` (a `PrepareStatement`, refusing a statement DuckDB cannot prepare with its type, as
  DuckDB names it), `EXECUTE` (an `ExecuteStatement` with its positional and named arguments),
  `DEALLOCATE` (a `DropObject` of kind `PREPARED STATEMENT`, as DuckDB reads it) and `CALL` (a
  `Call` of a qualified table function).
- `EXPLAIN` as a `Describe` styled `EXPLAIN` or `EXPLAIN ANALYZE`, its options in `properties`
  as written, of the statements DuckDB explains; `EXPLAIN (SQL)`, a view and a macro refuse a
  `PIVOT` whose values come from the data. `VACUUM` and `ANALYZE` are `Analyze` nodes (kind
  `VACUUM` for a vacuum) with their options, table and columns, refusing an unknown vacuum option.
  `EXPORT DATABASE` is an `Export` of its path, its source database in `connection` and its
  options, refusing a `FORMAT` without a name; `IMPORT DATABASE` is the `import_database`
  pragma it is to DuckDB.

Errors the catalogue records past the parser are left to later stages, which must refuse them
when they exist. These refusals are still to build:

- a duplicate column name in `CREATE TABLE` (a catalog error);
- `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` with `NOT NULL` or `UNIQUE`, which DuckDB refuses
  while rewriting the statement (not implemented).
- what DuckDB's transformer refuses as not implemented or invalid input rather than as a parser
  error: a `PREPARE` type list, `EXECUTE` arguments that are not scalar or mix names and
  positions, an `EXPLAIN` option or `FORMAT` value it does not take, `VACUUM FULL`, `FREEZE` and
  `VERBOSE`, and `ANALYZE VERBOSE`. They parse, so the tree keeps them.

Some shared-tree fields are set only by this parser. Before DuckDB becomes a conversion source,
classification must name each one so that converters refuse it, and typing must read or abstain
on it:

- `catalog_parts` on a table name, `qualified` on a star's `EXCLUDE`, `REPLACE` or `RENAME`, and
  a CTE's `key_items`; the older `catalog` and `except` fields lose parts;
- an aggregate's `qualified_name`, `quoted` and `export_state`, and a function's `export_state`;
- a method call's `distinct` and `order_by`;
- a macro's further definitions in `CreateFunction.overloads`, each a nameless `CreateFunction`
  that a walker collecting created functions skips, registering only the parent;
- a user type's `Custom` name, which holds the type as written, quoted parts and modifiers
  included (`"s1.mood"`, `GEOMETRY('OGC:CRS84')`), so typing abstains on it rather than
  reporting that text as a type.

Every other statement is kept as a named gap: a `Command` holding its text. `parse_strict` also
refuses an uncovered byte.

As in DuckDB, a tokenizer fault anywhere in the input comes before any parse, and a statement's
transformer refusal is raised only once the statement parses, so a later syntax error in it wins.

Nesting is bounded so that no input exhausts the caller's stack or takes exponential time:

- Each nested level (parentheses, subqueries, calls, types, `NOT` and similar prefixes) runs on
  a stack that grows on demand, and a statement nesting past 200 levels is refused with "the
  statement nests too deeply". Once one reading passes the limit, every other reading of the
  statement fails at once, and the statement is refused with that error. DuckDB allows deeper
  expressions (999 levels) and does not limit query nesting, so inputs between those depths are
  refused here and accepted by DuckDB; some constructs, such as scalar subqueries and lambdas,
  count two levels each. In practice 196 to 198 nested parentheses, `FROM` subqueries, joins,
  calls, `CASE`s, `CAST`s, lists, subscripts, `NOT`s, set operations or types parse, and 98
  nested scalar, `IN` or `EXISTS` subqueries.
- A type's constructors and array bounds count together: `STRUCT(a INTEGER)[]` is three levels,
  so a type taller than 200 levels is refused as nesting too deeply, where DuckDB takes types
  up to its expression depth.
- A flat chain of operators or indirections (`::`, `.`, `[]`, `!`) is refused
  with DuckDB's own message once it reaches 1,000 links, DuckDB's expression depth. DuckDB's
  own boundary differs from this by at most two links, depending on the context. DuckDB reads
  an `AND` or `OR` chain as one conjunction and does not limit set operations, so neither has a
  chain limit here; their trees are as deep as the chain, so only a chain past the tree depth
  below is refused.
- A statement whose finished tree is more than 8,000 levels deep, counted iteratively after it
  parses, is refused as nesting too deeply, where DuckDB accepts it: a chain of about 8,000
  `AND`, `OR` or set-operation links, or nested levels that each stay within the limits above
  but multiply, such as 60 parenthesised queries each with a 200-link `UNION ALL` chain.
- A typed literal is tried only when a string follows the type, and `TRIM`'s `characters FROM`
  only when `FROM` follows at the call's own level, so nested calls parse in linear time.

## Coverage

- `corpora/duckdb-core/duckshop.jsonl`: 875 of 877 statements parse strictly, with no gap. The
  two that do not are the excluded `CONNECT BY` model and its test, which DuckDB refuses too.
- The DuckDB catalogue's 5,663 recorded cases, judged by each case's expected verdict (the 25
  observe cases are skipped): 121 are gaps. Of the rest, none expected to be accepted is
  refused, and none expected to be refused is accepted.
- `corpora/duckdb-core/extras.jsonl`: the 4,133 distinct DuckDB inputs of sqlglot's and
  polyglot's tests, with DuckDB 2.0.0's verdict. None reaches a gap. Each is accepted
  where DuckDB accepts it, refused as a syntax error where DuckDB reports one, and refused with
  DuckDB's message otherwise.

## Lossy mappings

These parse without a gap, but a reader of the shared tree's older fields sees less than the
source said:

- A name with more than three parts keeps each catalog part, with its quoting, only in
  `TableRef.catalog_parts`; `catalog` holds them joined by `.`. Relation keys built from
  `catalog` alone (as `sqlquack-semantics` builds them) treat `m.a.b.c.t` and `m."a.b".c.t` as one
  relation until they read `catalog_parts`.
- A qualified star `EXCLUDE` or `RENAME` name is joined by `.` in `except` and `rename`; its parts
  are in `Star.qualified`.
- A `FETCH` without a count on a set operation becomes a limit of 1.
- A pivot `ON` entry that is an `IN` test is wrapped in a `Paren` node, so that it is not read
  as a pivot value list.
- A user type's name parts and modifiers are written out in `DataType::Custom.name`
  (`"s1.mood"`, `GEOMETRY('OGC:CRS84')`, `TUPLE(INTEGER, VARCHAR)`), with the modifiers as
  written; a reader of the type must parse the name to see them.
- `COLLATE` keeps DuckDB's collation name, the parts of a dotted name joined by `.`, so
  `COLLATE "a.b"` and `COLLATE a.b` share a name, as DuckDB gives them. A collation that is no
  constant or name, such as `COLLATE lower('x')`, which DuckDB refuses only after parsing (not
  implemented), keeps an empty name.
- `UPDATE ... SET (a, b) = value` and the same `ON CONFLICT DO UPDATE SET` become one `set` pair
  per column, as DuckDB's transformer splits them: a row's items in order, and any other value
  repeated for each column.
- `EXPLAIN (options)` keeps each option's value as its text in `properties`.
- A sample's size is kept as written, so a fractional row count is not turned into the whole
  count DuckDB uses.
- `SHOW TABLES FROM a` and `SHOW TABLES FROM a.b` keep the database or schema in `Show.from` as
  a `TableRef` whose last part is in `name`, so a schema reads as a table name and a database as
  that name's schema.

## To do

The remaining utility statements.
