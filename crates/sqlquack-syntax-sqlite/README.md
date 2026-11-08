# sqlquack-syntax-sqlite

**Work in progress.** A SQLite 3.53.4 scanner and parser that builds the shared
`sqlquack-syntax` tree. It follows SQLite's `tokenize.c`, `parse.y` and the checks the grammar's
actions make, and is checked against the 3.53.4 engine's `sqlite3_prepare_v2`. The crate's layout
follows `sqlquack-syntax-duckdb`.

## Done

- `scan`: `sqlite3GetToken` rule for rule. It covers the four identifier quotes (`"`, `` ` ``,
  `[ ]`, and a string where a name may stand), strings, `X'..'` blobs, decimal, hex and
  digit-separated numbers, the five parameter forms, `||`, `->` and `->>`. `WINDOW`, `OVER` and
  `FILTER` are keywords only where SQLite's lookahead makes them so. The scanner stops at a NUL
  byte, as SQLite does. `uncovered` reports a byte that no token, blank or comment covers.
- `keywords`: SQLite's 147 keywords and the grammar's `%fallback ID` list.
- `parse`: `SELECT`, `VALUES` and `WITH` queries by recursive descent over `parse.y`, with
  SQLite's precedence tiers:
  - Compounds and the result modifiers: `ORDER BY`, and `LIMIT` with `OFFSET` or a comma.
  - CTEs, with `[NOT] MATERIALIZED`, and named windows.
  - Joins, with the join words read as `sqlite3JoinType` reads them, nested table lists and
    table functions.
  - `CAST`, `CASE`, `EXISTS`, `IN` with a list or a subquery, `BETWEEN`, `LIKE`/`GLOB`/`REGEXP`/
    `MATCH` with `ESCAPE`, `IS [NOT] [DISTINCT FROM]`, `COLLATE`.
  - Calls with `DISTINCT`, an argument `ORDER BY`, `FILTER`, and `OVER` with a frame.
- The checks of the grammar's actions keep SQLite's messages. They are:
  - a duplicate CTE name;
  - `ORDER BY` or `LIMIT` before a compound operator;
  - more than 500 compound terms;
  - an unknown join type;
  - `ON` or `USING` without a join;
  - an unsupported frame;
  - `DISTINCT` in a window function;
  - an argument `ORDER BY` in a window function;
  - too many arguments;
  - a CTE column with `COLLATE` or a sort order;
  - a window chained to a missing or misused base;
  - parameter numbers out of range.

  As in SQLite, these checks report their error only once the parser takes the next token. So a
  later syntax error wins over them when it falls on that token.

## Local parser limits

Past these limits, parsing fails with "the statement nests too deeply" (`syntax: false`) instead of
overflowing the stack. Some inputs that SQLite 3.53.4 accepts fall past them.

| Shape, nested or chained | Parser accepts up to | sqlite3 3.53.4 accepts up to |
|---|---|---|
| Derived tables, `(SELECT * FROM (...))` | 999 | 415 |
| Scalar subqueries, `(SELECT (SELECT ...))` | 499 | 498 |
| `EXISTS` subqueries | 499 | 415 |
| `IN` subqueries | 499 | 355 |
| Subqueries holding a `UNION` | 499 | 355 |
| `IN` lists, `1 IN (1 IN (...))` | 998 | 499 |
| `CASE WHEN 1 THEN (CASE ...) END` | 998 | 498 |
| Nested `WITH`, `WITH a AS (WITH a AS (...) ...)` | 999 | 498 |
| `FILTER (WHERE count(*) FILTER (...))` | 998 | 356 |
| Window `ORDER BY`, `count(*) OVER (ORDER BY count(*) OVER (...))` | 998 | 311 |
| Flat chains of `AND`, `+` and similar | 998 operators | 999 operators |
| Parentheses around one value | 998 | 2,493 |
| Parentheses around one table in `FROM` | 999 | 1,245 |
| Terms of a compound `SELECT` | 500 (SQLite's own check) | 500 |

The parser's figures come from two limits: 1,000 levels of recursion and chained operators, as
sqlite3's 1,000-level expression depth (`SQLITE_MAX_EXPR_DEPTH`), and a tree depth of 8,000, the
depth the shared tree's clone, comparison, serialization and drop are tested to on a 2 MiB stack.
Within the recursion limit no shape builds a tree deeper than about 2,000 levels, so the tree-depth
limit is a backstop.

sqlite3's lower figures for subqueries, `IN` lists, `CASE`, `WITH`, `FILTER` and window `ORDER BY`
come from its resource limits: its parser's recursion limit ("Recursion limit", the depth of its
LALR stack) and "Expression tree is too large". The parser does not model them, so it accepts these
shapes past sqlite3's figures. These are known false accepts at depth. Its higher figures for
parentheses come from the same LALR stack, which does not count a parenthesis as expression depth;
there the parser refuses earlier. Flat chains stop one operator short of sqlite3.

A depth probe checks the limits: 31 shapes at depths from 50 to 100,000, on 8 MiB and 2 MiB stacks,
in debug and release builds. Each case is parsed and serialized in its own process. It found no
crashes and no case slower than 1.6 s.

## Gaps

Each gap turns the whole statement into a `Command` node and records a named gap. Nothing is
accepted silently.

- Every statement that is not a query: DML, upsert, DDL, `PRAGMA`, transactions and the rest. The
  gap is named after the statement's first keyword in lower case, such as `insert` or `pragma`.
  The parser does not check these statements past that keyword.
- `IN table` and `IN table-function(...)` (`in_table`).
- `INDEXED BY` and `NOT INDEXED` (`indexed_by`).
- Tcl-style `$a::b(c)` and `#name` parameters (`parameter`).
- `RAISE` (`raise`), which only appears in trigger bodies.

## Lossy mappings

- Calls are generic `Function` and `AggregateFunction` nodes, named as written. Later stages
  classify them by name, as they do for the DuckDB parser.
- An unquoted, one-part `true` or `false` is a `Boolean`. SQLite resolves such a name to a boolean
  only when no column has that name.
- A type name in `CAST` maps to a `DataType` when SQLite's affinity words name one. Otherwise it is
  `Custom` with its modifiers written out.
- `0x` numbers are `HexNumber` and blobs are `HexString`.
