# sqlquack-syntax notes

Explanations kept from multi-line doc comments that became one-line doc comments.

## `ast_children.rs`: (module)

Canonical child traversal for the typed AST.

Implementations are generated beside the AST definitions by the private
`AstNode` derive. New AST payload fields are traversed automatically when
their type contains [`Expression`]. Use `#[ast(skip)]` only for derived
metadata that must not participate in syntax traversal or transformation.

## `dialects/mod.rs`: (module)

SQL Dialect System

This module implements the dialect abstraction layer that enables SQL transpilation
between more than 30 SQL dialects. Each dialect encapsulates three concerns:

- **Tokenization**: Dialect-specific lexing rules (e.g., BigQuery uses backtick quoting,
  MySQL uses backtick for identifiers, TSQL uses square brackets).
- **Generation**: How AST nodes are rendered back to SQL text, including identifier quoting
  style, function name casing, and syntax variations.
- **Transformation**: AST-level rewrites that convert dialect-specific constructs to/from
  a normalized form (e.g., Snowflake `SQUARE(x)` becomes `POWER(x, 2)`).

The primary entry point is [`Dialect::get`], which returns a configured [`Dialect`] instance
for a given [`DialectType`]. From there, callers can [`parse`](Dialect::parse),
[`generate`](Dialect::generate), [`transform`](Dialect::transform), or
[`transpile`](Dialect::transpile) to another dialect in a single call.

Each concrete dialect (e.g., `PostgresDialect`, `BigQueryDialect`) implements the
[`DialectImpl`] trait, which provides configuration hooks and expression-level transforms.
Dialect modules live in submodules of this module and are re-exported here.

## `dialects/mod.rs`: pub(crate) struct Dialect {

Main entry point for dialect-specific SQL operations.

A `Dialect` bundles together a tokenizer, generator configuration, and expression
transformer for a specific SQL database engine. It is the high-level API through
which callers parse, generate, transform, and transpile SQL.

# Usage

```rust,ignore
use polyglot_sql::dialects::{Dialect, DialectType};

// Parse PostgreSQL SQL into an AST
let pg = Dialect::get(DialectType::PostgreSQL);
let exprs = pg.parse("SELECT id, name FROM users WHERE active")?;

// Transpile from PostgreSQL to BigQuery
let results = pg.transpile("SELECT NOW()", DialectType::BigQuery)?;
assert_eq!(results[0], "SELECT CURRENT_TIMESTAMP()");
```

Obtain an instance via [`Dialect::get`] or [`Dialect::get_by_name`].
The struct is `Send + Sync` safe so it can be shared across threads.

## `expressions.rs`: (module)

SQL Expression AST (Abstract Syntax Tree).

This module defines all the AST node types used to represent parsed SQL
statements and expressions. The design follows Python sqlglot's expression
hierarchy, ported to a Rust enum-based AST.

# Architecture

The central type is [`Expression`], a large tagged enum with one variant per
SQL construct. Inner structs carry the fields for each variant. Most
heap-allocated variants are wrapped in `Box` to keep the enum size small.

# Variant Groups

| Group | Examples | Purpose |
|---|---|---|
| **Queries** | `Select`, `Union`, `Intersect`, `Except`, `Subquery` | Top-level query structures |
| **DML** | `Insert`, `Update`, `Delete`, `Merge`, `Copy` | Data manipulation |
| **DDL** | `CreateTable`, `AlterTable`, `DropView`, `CreateIndex` | Schema definition |
| **Clauses** | `From`, `Join`, `Where`, `GroupBy`, `OrderBy`, `With` | Query clauses |
| **Operators** | `And`, `Or`, `Add`, `Eq`, `Like`, `Not` | Binary and unary operations |
| **Functions** | `Function`, `AggregateFunction`, `WindowFunction`, `Count`, `Sum` | Scalar, aggregate, and window functions |
| **Literals** | `Literal`, `Boolean`, `Null`, `Interval` | Constant values |
| **Types** | `DataType`, `Cast`, `TryCast`, `SafeCast` | Data types and casts |
| **Identifiers** | `Identifier`, `Column`, `Table`, `Star` | Name references |

# SQL Generation

Every `Expression` can be rendered back to SQL via [`Expression::sql()`]
(generic dialect) or [`Expression::sql_for()`] (specific dialect). The
actual generation logic lives in the `generator` module.

## `expressions.rs`: #[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Ser

Represent any SQL expression or statement as a single, recursive AST node.

`Expression` is the root type of the polyglot AST. Every parsed SQL
construct -- from a simple integer literal to a multi-CTE query with
window functions -- is represented as a variant of this enum.

Variants are organized into logical groups (see the module-level docs).
Most non-trivial variants box their payload so that `size_of::<Expression>()`
stays small (currently two words: tag + pointer).

# Constructing Expressions

Use the convenience constructors on `impl Expression` for common cases:

```rust,ignore
use polyglot_sql::expressions::Expression;

let col  = Expression::column("id");
let lit  = Expression::number(42);
let star = Expression::star();
```

# Generating SQL

```rust,ignore
let expr = Expression::column("name");
assert_eq!(expr.sql(), "name");
```

## `expressions.rs`: #[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Ser

Represent a SQL literal value.

Numeric values are stored as their original text representation (not parsed
to `i64`/`f64`) so that precision, trailing zeros, and hex notation are
preserved across round-trips.

Dialect-specific literal forms (triple-quoted strings, dollar-quoted
strings, raw strings, etc.) each have a dedicated variant so that the
generator can emit them with the correct syntax.

## `expressions.rs`: #[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Ser

Oracle-specific data types whose semantics cannot be represented losslessly by the
generic [`DataType`] variants.

Keeping these types structured allows the Oracle parser to retain details such as a
negative `NUMBER` scale, `BYTE`/`CHAR` length semantics, interval precisions, and
`TIMESTAMP WITH LOCAL TIME ZONE` until the target dialect is known.

## `expressions.rs`: pub const TYPED_EXPRESSION_FUNCTION: &str = "__polyglot_typed";

Name of the analysis-only type assertion function.

The parser lowers `__polyglot_typed(expr, 'TYPE')` into an [`Annotated`] node whose
`type_assertion` is `TYPE`. The node is transparent for every analysis except typing:
it takes the asserted type only where the type of `expr` is unknown.

## `expressions.rs`: pub const TYPED_LIKE_EXPRESSION_FUNCTION: &str = "__polyglot_typed_lik

Name of the analysis-only type assertion function that asserts another expression's type.

The parser lowers `__polyglot_typed_like(expr, like)` into an [`Annotated`] node whose
`type_like` is `like`. Type annotation resolves the type of `like` into the node's
`type_assertion`, which then applies as for [`TYPED_EXPRESSION_FUNCTION`]; an unknown
type of `like` asserts nothing. Like `type_assertion`, `like` is not a child of the node.

## `expressions.rs`: pub fn type_assertion_result<'a>(

Type of an expression carrying a type assertion.

The asserted type applies where the inferred type is absent or unknown, and where a
partly unknown inferred type has the asserted type's shape (see
[`DataType::is_refined_by`]). A known inferred type, or a partly unknown one of
another shape, stands, including when it disagrees with the assertion, so callers
can report the disagreement.

## `function_registry.rs`: (module)

Centralized parser-facing function metadata.

This module is intentionally small and focused on parseability flags that were
previously scattered across parser-local lists. Unknown `name(...)` functions
still parse via generic function fallback; this registry only captures
parser-special cases like no-paren and aggregate behavior.

## `function_registry.rs`: pub(crate) const DUCKDB_AGGREGATE_FUNCTION_NAME_LIST: &[&str] = &[

DuckDB aggregate names that are not already covered by the shared registry.

This list follows DuckDB's 1.4 LTS aggregate-function catalog. Macro aliases
such as `GEOMEAN` and `WAVG` are included because calls to those macros have
aggregate cardinality in DuckDB. `HISTOGRAM_VALUES` is intentionally absent:
DuckDB exposes it as a table macro used in `FROM`, not as an aggregate
expression.

## `lib.rs`: (module)

SQLQuack's tokenizer, parser and syntax tree, for Snowflake and DuckDB.

The modules below the crate root are derived from polyglot (see `THIRD_PARTY_NOTICES`) and kept
close to it, so a dev-only check can prove that parse results are identical. Only Snowflake and
DuckDB are reachable through this API.

Every module is linted as the rest of the workspace.

## `lossless.rs`: (module)

Lossless tokens: every byte of the input belongs to exactly one token.

The copied tokenizer produces the significant tokens the parser reads and attaches comments to
them. This layer keeps those tokens unchanged and adds a trivia token for every whitespace run
and comment between them, with byte ranges, so the tokens joined back equal the input.

## `modes.rs`: (module)

Strict and tolerant parsing.

Strict parsing is for conversion: any error refuses the whole input. Tolerant parsing is for
formatting and linting unfinished SQL: it always returns the lossless tokens, parses each
statement on its own, keeps the ranges it cannot parse, and never panics.

## `parser.rs`: (module)

SQL Parser -- recursive-descent parser that converts a token stream into an AST.

The central type is [`Parser`], which consumes tokens produced by the
[`Tokenizer`](crate::tokens::Tokenizer) and builds a tree of [`Expression`]
nodes covering the full SQL grammar: queries, DML, DDL, set operations,
window functions, CTEs, and dialect-specific extensions for more than 30 SQL dialects.

The simplest entry point is [`Parser::parse_sql`], which tokenizes and
parses a SQL string in one call.

# Static configuration maps

This module also exports several `LazyLock<HashSet<TokenType>>` constants
(ported from Python sqlglot's `parser.py`) that classify token types:

- [`TYPE_TOKENS`] -- all tokens that represent SQL data types
- [`NESTED_TYPE_TOKENS`] -- parametric types like `ARRAY`, `MAP`, `STRUCT`
- [`RESERVED_TOKENS`] -- tokens that cannot be used as unquoted identifiers
- [`NO_PAREN_FUNCTIONS`] / [`NO_PAREN_FUNCTION_NAMES`] -- zero-argument
  functions that may be written without parentheses (e.g. `CURRENT_DATE`)
- [`DB_CREATABLES`] -- object kinds valid after `CREATE` (TABLE, VIEW, etc.)
- [`SUBQUERY_PREDICATES`] -- tokens introducing subquery predicates (ANY, ALL, EXISTS)

## `parser.rs`: #[cfg(test)]

Termination and time-budget oracle for the parser.

Every truncated prefix of a valid statement must reach a decision — parsed or
rejected — inside a fixed budget. Termination alone is too weak an assertion: a
parser can terminate and still spend seconds on tens of bytes, so the budget is
what is asserted. A normal prefix decides in microseconds, so the budget leaves six
orders of magnitude of headroom; it is a statement about the parser's complexity,
not about how fast the machine is.

## `parser.rs`: const BUDGET: Duration = Duration::from_secs(5);

Per-input budget. Enforced: going over it kills the worker. A prefix in this
corpus decides in microseconds and the defects guarded here cost seconds or never
finish at all, so there is a lot of room for a loaded machine to be slower.

## `parser.rs`: fn decide_all(inputs: &[&str]) -> Vec<Decision> {

Decide `inputs` in order, under an enforced per-input budget.

Returns one decision per input, stopping at the first input the worker did not
survive: the result is shorter than `inputs` exactly when that happened, and its
last element says what went wrong. Stopping there keeps a red run to one budget
rather than one per remaining input.

## `parser.rs`: #[test]

The oracle proper. Every truncated prefix of a valid statement must reach a
decision — parsed or rejected — inside the budget. It needs no knowledge of which
loops exist, so it also covers the ones nobody enumerated.

## `tokens.rs`: (module)

Token types and tokenization for SQL parsing

This module defines all SQL token types and the tokenizer that converts
SQL strings into token streams.

## `traversal.rs`: (module)

Tree traversal utilities for SQL expression ASTs.

This module provides read-only traversal, search, and transformation utilities
for the [`Expression`] tree produced by the parser. Because Rust's ownership
model does not allow parent pointers inside the AST, parent information is
tracked externally via [`TreeContext`] (built on demand).

# Traversal

Two iterator types are provided:
- [`DfsIter`] -- depth-first (pre-order) traversal using a stack. Visits a node
  before its children. Good for top-down analysis and early termination.
- [`BfsIter`] -- breadth-first (level-order) traversal using a queue. Visits all
  nodes at depth N before any node at depth N+1. Good for level-aware analysis.

Both are available through the [`ExpressionWalk`] trait methods [`dfs`](ExpressionWalk::dfs)
and [`bfs`](ExpressionWalk::bfs).

# Searching

The [`ExpressionWalk`] trait also provides convenience methods for finding expressions:
[`find`](ExpressionWalk::find), [`find_all`](ExpressionWalk::find_all),
[`contains`](ExpressionWalk::contains), and [`count`](ExpressionWalk::count).
Common predicates are available as free functions: [`is_column`], [`is_literal`],
[`is_function`], [`is_aggregate`], [`is_window_function`], [`is_subquery`], and
[`is_select`].

# Transformation

[`transform_all`] rewrites every expression visited by the read-only walkers,
bottom-up, using the generated child visitor and an explicit stack.
[`transform`] and [`transform_map`] retain the selective child traversal of
[`transform_recursive`](crate::dialects::transform_recursive) for compatibility.
The [`ExpressionWalk::transform_owned`] method provides the same capability as
an owned method on `Expression`.

Based on traversal patterns from `sqlglot/expressions.py`.

## `traversal.rs`: #[derive(Debug, Default)]

External parent-tracking context for an expression tree.

Since Rust's ownership model does not allow intrusive parent pointers in the AST,
`TreeContext` provides an on-demand side-table that maps each node (identified by
a [`NodeId`]) to its [`ParentInfo`] (parent node, field name, and list index).

Build a context from any expression root with [`TreeContext::build`], then query
parent relationships with [`get`](TreeContext::get), ancestry chains with
[`ancestors_of`](TreeContext::ancestors_of), or tree depth with
[`depth_of`](TreeContext::depth_of).

This is useful when analysis requires upward navigation (e.g., determining whether
a column reference appears inside a WHERE clause or a JOIN condition).
