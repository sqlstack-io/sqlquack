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
- Nesting deeper than 1,000 levels fails with "the statement nests too deeply" (`syntax: false`),
  never a stack overflow. SQLite's own limit is the same expression depth.

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
