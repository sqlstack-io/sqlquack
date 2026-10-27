//! Data models of expressions queries.

use crate::tokens::models::SourceSpan;
use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Null;
use crate::expressions::operators::models::ResultSerialization;
use crate::expressions::operators::models::WithFill;
use crate::expressions::statements::models::Lock;
use crate::expressions::statements::models::VerticaSelectExtensions;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Tuple;

/// A complete SELECT statement; absent optional parts are omitted from generated SQL.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Select {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertica: Option<Box<VerticaSelectExtensions>>,
    /// The select-list: columns, expressions, aliases, and wildcards.
    pub expressions: Vec<Expression>,
    /// The FROM clause, containing one or more table sources.
    pub from: Option<From>,
    /// JOIN clauses applied after the FROM source.
    pub joins: Vec<Join>,
    pub lateral_views: Vec<LateralView>,
    /// ClickHouse PREWHERE clause
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prewhere: Option<Expression>,
    pub where_clause: Option<Where>,
    pub group_by: Option<GroupBy>,
    pub having: Option<Having>,
    pub qualify: Option<Qualify>,
    pub order_by: Option<OrderBy>,
    pub distribute_by: Option<DistributeBy>,
    pub cluster_by: Option<ClusterBy>,
    pub sort_by: Option<SortBy>,
    pub limit: Option<Limit>,
    pub offset: Option<Offset>,
    /// ClickHouse LIMIT BY clause expressions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_by: Option<Vec<Expression>>,
    pub fetch: Option<Fetch>,
    pub distinct: bool,
    pub distinct_on: Option<Vec<Expression>>,
    pub top: Option<Top>,
    pub with: Option<With>,
    pub sample: Option<Sample>,
    /// ClickHouse SETTINGS clause (e.g., SETTINGS max_threads = 4)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<Vec<Expression>>,
    /// ClickHouse FORMAT clause (e.g., FORMAT PrettyCompact)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Expression>,
    pub windows: Option<Vec<NamedWindow>>,
    pub hint: Option<Hint>,
    /// Oracle CONNECT BY clause for hierarchical queries
    pub connect: Option<Connect>,
    /// SELECT ... INTO table_name for creating tables
    pub into: Option<SelectInto>,
    /// FOR UPDATE/SHARE locking clauses
    #[serde(default)]
    pub locks: Vec<Lock>,
    /// T-SQL FOR XML clause options (PATH, RAW, AUTO, EXPLICIT, BINARY BASE64, ELEMENTS XSINIL, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub for_xml: Vec<Expression>,
    /// T-SQL FOR JSON clause options (PATH, AUTO, ROOT, INCLUDE_NULL_VALUES, WITHOUT_ARRAY_WRAPPER)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub for_json: Vec<Expression>,
    /// Structured result serialization, including a bare FOR JSON/XML clause.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_serialization: Option<ResultSerialization>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_collation: Option<Identifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query_hints: Vec<Expression>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub total_rowcount: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    /// Leading comments before the statement
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// Comments after the SELECT keyword and before the first projection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_select_comments: Vec<String>,
    /// BigQuery SELECT AS STRUCT / SELECT AS VALUE kind
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// MySQL operation modifiers (HIGH_PRIORITY, STRAIGHT_JOIN, SQL_CALC_FOUND_ROWS, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operation_modifiers: Vec<String>,
    /// Whether QUALIFY appears after WINDOW (DuckDB) vs before (Snowflake/BigQuery default)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub qualify_after_window: bool,
    /// TSQL OPTION clause (e.g., OPTION(LABEL = 'foo'))
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option: Option<String>,
    /// Redshift-style EXCLUDE at the end of the projection list: `SELECT *, 4 AS c EXCLUDE (a)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<Expression>>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

impl Select {
    pub fn new() -> Self {
        Self {
            vertica: None,
            expressions: Vec::new(),
            from: None,
            joins: Vec::new(),
            lateral_views: Vec::new(),
            prewhere: None,
            where_clause: None,
            group_by: None,
            having: None,
            qualify: None,
            order_by: None,
            distribute_by: None,
            cluster_by: None,
            sort_by: None,
            limit: None,
            offset: None,
            limit_by: None,
            fetch: None,
            distinct: false,
            distinct_on: None,
            top: None,
            with: None,
            sample: None,
            settings: None,
            format: None,
            windows: None,
            hint: None,
            connect: None,
            into: None,
            locks: Vec::new(),
            for_xml: Vec::new(),
            for_json: Vec::new(),
            result_serialization: None,
            query_collation: None,
            query_hints: Vec::new(),
            total_rowcount: false,
            source_dialect: None,
            leading_comments: Vec::new(),
            post_select_comments: Vec::new(),
            kind: None,
            operation_modifiers: Vec::new(),
            qualify_after_window: false,
            option: None,
            exclude: None,
            span: SourceSpan::default(),
        }
    }

    /// Add a column to select
    pub fn column(mut self, expr: Expression) -> Self {
        self.expressions.push(expr);
        self
    }

    /// Set the FROM clause
    pub fn from(mut self, table: Expression) -> Self {
        self.from = Some(From {
            expressions: vec![table],
        });
        self
    }

    /// Add a WHERE clause
    pub fn where_(mut self, condition: Expression) -> Self {
        self.where_clause = Some(Where { this: condition });
        self
    }

    /// Set DISTINCT
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Add a JOIN
    pub fn join(mut self, join: Join) -> Self {
        self.joins.push(join);
        self
    }

    /// Set ORDER BY
    pub fn order_by(mut self, expressions: Vec<Ordered>) -> Self {
        self.order_by = Some(OrderBy {
            expressions,
            siblings: false,
            comments: Vec::new(),
        });
        self
    }

    /// Set LIMIT
    pub fn limit(mut self, n: Expression) -> Self {
        self.limit = Some(Limit {
            this: n,
            percent: false,
            comments: Vec::new(),
        });
        self
    }

    /// Set OFFSET
    pub fn offset(mut self, n: Expression) -> Self {
        self.offset = Some(Offset {
            this: n,
            rows: None,
        });
        self
    }
}

impl Default for Select {
    fn default() -> Self {
        Self::new()
    }
}

/// UNION [ALL] of two queries, with optional ORDER BY, LIMIT, OFFSET and BY NAME.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Union {
    /// The left-hand query operand.
    pub left: Expression,
    /// The right-hand query operand.
    pub right: Expression,
    /// Whether UNION ALL (true) or UNION (false, which deduplicates).
    pub all: bool,
    /// Whether DISTINCT was explicitly specified
    #[serde(default)]
    pub distinct: bool,
    /// Optional WITH clause
    pub with: Option<With>,
    /// ORDER BY applied to entire UNION result
    pub order_by: Option<OrderBy>,
    /// Row limit on the whole UNION: a count expression or a full Limit/Fetch clause.
    pub limit: Option<Box<Expression>>,
    /// OFFSET applied to entire UNION result
    pub offset: Option<Box<Expression>>,
    /// DISTRIBUTE BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribute_by: Option<DistributeBy>,
    /// SORT BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<SortBy>,
    /// CLUSTER BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_by: Option<ClusterBy>,
    /// DuckDB, Snowflake, and BigQuery BY NAME modifier
    #[serde(default)]
    pub by_name: bool,
    /// BigQuery: Set operation side (LEFT, RIGHT, FULL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    /// BigQuery: Set operation kind (INNER)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// BigQuery: CORRESPONDING modifier
    #[serde(default)]
    pub corresponding: bool,
    /// BigQuery: STRICT modifier (before CORRESPONDING)
    #[serde(default)]
    pub strict: bool,
    /// BigQuery: BY (columns) after CORRESPONDING
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_columns: Vec<Expression>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// Flattens the left-recursive chain iteratively so dropping deep set operations cannot overflow.
impl Drop for Union {
    fn drop(&mut self) {
        while let Expression::Union(ref mut inner) = self.left {
            let next_left = std::mem::replace(&mut inner.left, Expression::Null(Null));
            let old_left = std::mem::replace(&mut self.left, next_left);
            drop(old_left);
        }
    }
}

/// INTERSECT [ALL] of two queries: rows in both, duplicates kept by multiplicity under ALL.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Intersect {
    /// The left-hand query operand.
    pub left: Expression,
    /// The right-hand query operand.
    pub right: Expression,
    /// Whether INTERSECT ALL (true) or INTERSECT (false, which deduplicates).
    pub all: bool,
    /// Whether DISTINCT was explicitly specified
    #[serde(default)]
    pub distinct: bool,
    /// Optional WITH clause
    pub with: Option<With>,
    /// ORDER BY applied to entire INTERSECT result
    pub order_by: Option<OrderBy>,
    /// Row limit on the whole INTERSECT: a count expression or a full Limit/Fetch clause.
    pub limit: Option<Box<Expression>>,
    /// OFFSET applied to entire INTERSECT result
    pub offset: Option<Box<Expression>>,
    /// DISTRIBUTE BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribute_by: Option<DistributeBy>,
    /// SORT BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<SortBy>,
    /// CLUSTER BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_by: Option<ClusterBy>,
    /// DuckDB, Snowflake, and BigQuery BY NAME modifier
    #[serde(default)]
    pub by_name: bool,
    /// BigQuery: Set operation side (LEFT, RIGHT, FULL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    /// BigQuery: Set operation kind (INNER)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// BigQuery: CORRESPONDING modifier
    #[serde(default)]
    pub corresponding: bool,
    /// BigQuery: STRICT modifier (before CORRESPONDING)
    #[serde(default)]
    pub strict: bool,
    /// BigQuery: BY (columns) after CORRESPONDING
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_columns: Vec<Expression>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

impl Drop for Intersect {
    fn drop(&mut self) {
        while let Expression::Intersect(ref mut inner) = self.left {
            let next_left = std::mem::replace(&mut inner.left, Expression::Null(Null));
            let old_left = std::mem::replace(&mut self.left, next_left);
            drop(old_left);
        }
    }
}

/// EXCEPT (MINUS) [ALL]: left rows not in the right, subtracted by multiplicity under ALL.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Except {
    /// The left-hand query operand.
    pub left: Expression,
    /// The right-hand query operand (rows to subtract).
    pub right: Expression,
    /// Whether EXCEPT ALL (true) or EXCEPT (false, which deduplicates).
    pub all: bool,
    /// Whether DISTINCT was explicitly specified
    #[serde(default)]
    pub distinct: bool,
    /// Optional WITH clause
    pub with: Option<With>,
    /// ORDER BY applied to entire EXCEPT result
    pub order_by: Option<OrderBy>,
    /// Row limit on the whole EXCEPT: a count expression or a full Limit/Fetch clause.
    pub limit: Option<Box<Expression>>,
    /// OFFSET applied to entire EXCEPT result
    pub offset: Option<Box<Expression>>,
    /// DISTRIBUTE BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribute_by: Option<DistributeBy>,
    /// SORT BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<SortBy>,
    /// CLUSTER BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_by: Option<ClusterBy>,
    /// DuckDB, Snowflake, and BigQuery BY NAME modifier
    #[serde(default)]
    pub by_name: bool,
    /// BigQuery: Set operation side (LEFT, RIGHT, FULL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    /// BigQuery: Set operation kind (INNER)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// BigQuery: CORRESPONDING modifier
    #[serde(default)]
    pub corresponding: bool,
    /// BigQuery: STRICT modifier (before CORRESPONDING)
    #[serde(default)]
    pub strict: bool,
    /// BigQuery: BY (columns) after CORRESPONDING
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_columns: Vec<Expression>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

impl Drop for Except {
    fn drop(&mut self) {
        while let Expression::Except(ref mut inner) = self.left {
            let next_left = std::mem::replace(&mut inner.left, Expression::Null(Null));
            let old_left = std::mem::replace(&mut self.left, next_left);
            drop(old_left);
        }
    }
}

/// INTO clause for SELECT INTO statements
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectInto {
    /// Target table or variable (used when single target)
    pub this: Expression,
    /// Whether TEMPORARY keyword was used
    #[serde(default)]
    pub temporary: bool,
    /// Whether UNLOGGED keyword was used (PostgreSQL)
    #[serde(default)]
    pub unlogged: bool,
    /// Whether BULK COLLECT INTO was used (Oracle PL/SQL)
    #[serde(default)]
    pub bulk_collect: bool,
    /// Multiple target variables (Oracle PL/SQL: BULK COLLECT INTO v1, v2)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expressions: Vec<Expression>,
}

/// A parenthesized subquery with optional alias, column aliases, ORDER BY, LIMIT and OFFSET.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subquery {
    /// The inner query expression.
    pub this: Expression,
    /// Optional alias for the derived table.
    pub alias: Option<Identifier>,
    /// Optional column aliases: AS t(c1, c2)
    pub column_aliases: Vec<Identifier>,
    /// Whether AS keyword was explicitly used for the alias.
    #[serde(default)]
    pub alias_explicit_as: bool,
    /// Original alias keyword spelling, e.g. `AS` vs `as`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias_keyword: Option<String>,
    /// ORDER BY clause (for parenthesized queries)
    pub order_by: Option<OrderBy>,
    /// LIMIT clause
    pub limit: Option<Limit>,
    /// OFFSET clause
    pub offset: Option<Offset>,
    /// DISTRIBUTE BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribute_by: Option<DistributeBy>,
    /// SORT BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<SortBy>,
    /// CLUSTER BY clause (Hive/Spark)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_by: Option<ClusterBy>,
    /// Whether this is a LATERAL subquery (can reference earlier tables in FROM)
    #[serde(default)]
    pub lateral: bool,
    /// Whether ORDER BY/LIMIT/OFFSET render inside the parentheses (`(SELECT 1 LIMIT 1)`).
    #[serde(default)]
    pub modifiers_inside: bool,
    /// Trailing comments after the closing paren
    #[serde(default)]
    pub trailing_comments: Vec<String>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// Pipe syntax step `query |> transform` (BigQuery, DataFusion).
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipeOperator {
    /// The input query/expression (left side of |>)
    pub this: Expression,
    /// The piped operation (right side of |>)
    pub expression: Expression,
}

/// VALUES table constructor: VALUES (1, 'a'), (2, 'b')
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Values {
    /// The rows of values
    pub expressions: Vec<Tuple>,
    /// Optional alias for the table
    pub alias: Option<Identifier>,
    /// Optional column aliases: AS t(c1, c2)
    pub column_aliases: Vec<Identifier>,
}

/// PIVOT/UNPIVOT, in the standard FROM-clause form or DuckDB's statement form.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pivot {
    /// Source table/expression
    pub this: Expression,
    /// Standard PIVOT's aggregate functions (the first is primary); unused by DuckDB's form.
    #[serde(default)]
    pub expressions: Vec<Expression>,
    /// For standard PIVOT: the FOR...IN clause(s) as In expressions
    #[serde(default)]
    pub fields: Vec<Expression>,
    /// For standard: unused. For DuckDB simplified: the USING aggregation functions
    #[serde(default)]
    pub using: Vec<Expression>,
    /// GROUP BY clause (used in both standard inside-parens and DuckDB simplified)
    #[serde(default)]
    pub group: Option<Box<Expression>>,
    /// Whether this is an UNPIVOT (vs PIVOT)
    #[serde(default)]
    pub unpivot: bool,
    /// For DuckDB UNPIVOT: INTO NAME col VALUE col
    #[serde(default)]
    pub into: Option<Box<Expression>>,
    /// Optional alias
    #[serde(default)]
    pub alias: Option<Identifier>,
    /// Optional output column aliases from `PIVOT(...) AS alias(col1, col2, ...)`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alias_columns: Vec<Identifier>,
    /// Include/exclude nulls (for UNPIVOT)
    #[serde(default)]
    pub include_nulls: Option<bool>,
    /// Default on null value (Snowflake)
    #[serde(default)]
    pub default_on_null: Option<Box<Expression>>,
    /// WITH clause (CTEs)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with: Option<With>,
}

/// UNPIVOT operation
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unpivot {
    pub this: Expression,
    pub value_column: Identifier,
    pub name_column: Identifier,
    pub columns: Vec<Expression>,
    pub alias: Option<Identifier>,
    /// Optional output column aliases from `UNPIVOT(...) AS alias(col1, col2, ...)`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alias_columns: Vec<Identifier>,
    /// Whether the value_column was parenthesized in the original SQL
    #[serde(default)]
    pub value_column_parenthesized: bool,
    /// INCLUDE NULLS (true), EXCLUDE NULLS (false), or not specified (None)
    #[serde(default)]
    pub include_nulls: Option<bool>,
    /// Additional value columns when parenthesized (e.g., (first_half_sales, second_half_sales))
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_value_columns: Vec<Identifier>,
}

/// A pivot alias: an identifier or an expression (Oracle/BigQuery concatenated aliases).
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotAlias {
    pub this: Expression,
    pub alias: Expression,
}

/// PREWHERE clause (ClickHouse) - early filtering before WHERE
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreWhere {
    pub this: Expression,
}

/// STREAM definition (Snowflake) - for change data capture
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stream {
    pub this: Expression,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<Expression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_initial_rows: Option<bool>,
}

/// USING DATA clause for data import statements
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsingData {
    pub this: Expression,
}

/// XML Namespace declaration
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XmlNamespace {
    pub this: Expression,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<Identifier>,
}

/// ROW FORMAT clause for Hive/Spark
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowFormat {
    pub delimited: bool,
    pub fields_terminated_by: Option<String>,
    pub collection_items_terminated_by: Option<String>,
    pub map_keys_terminated_by: Option<String>,
    pub lines_terminated_by: Option<String>,
    pub null_defined_as: Option<String>,
}

/// A FROM clause; several sources are comma-separated implicit joins.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct From {
    /// The table source expressions.
    pub expressions: Vec<Expression>,
}

/// A JOIN between two sources, with an ON predicate or a USING list.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Join {
    /// The right-hand table expression being joined.
    pub this: Expression,
    /// The ON condition (mutually exclusive with `using`).
    pub on: Option<Expression>,
    /// The USING column list (mutually exclusive with `on`).
    pub using: Vec<Identifier>,
    /// The join type (INNER, LEFT, RIGHT, FULL, CROSS, etc.).
    pub kind: JoinKind,
    /// Whether INNER keyword was explicitly used (INNER JOIN vs JOIN)
    pub use_inner_keyword: bool,
    /// Whether OUTER keyword was explicitly used (LEFT OUTER JOIN vs LEFT JOIN)
    pub use_outer_keyword: bool,
    /// Whether the ON/USING condition was deferred (assigned right-to-left for chained JOINs)
    pub deferred_condition: bool,
    /// TSQL join hint: LOOP, HASH, MERGE (e.g., INNER LOOP JOIN)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_hint: Option<String>,
    /// Snowflake ASOF JOIN match condition (MATCH_CONDITION clause)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_condition: Option<Expression>,
    /// PIVOT/UNPIVOT operations that follow this join (Oracle/TSQL syntax)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pivots: Vec<Expression>,
    /// Comments collected between join-kind keywords (for example `INNER <comment> JOIN`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    /// Nesting group of a join for pretty-printing; joins parsed together share a group.
    #[serde(default)]
    pub nesting_group: usize,
    /// Snowflake: DIRECTED keyword in JOIN (e.g., CROSS DIRECTED JOIN)
    #[serde(default)]
    pub directed: bool,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// Every join type, standard and dialect-specific (SEMI/ANTI, APPLY, ASOF, ARRAY, ...).
#[derive(
    sqlquack_syntax_derive::AstNode,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
)]
pub enum JoinKind {
    #[default]
    Inner,
    Left,
    Right,
    Full,
    Outer,
    Cross,
    Natural,
    NaturalLeft,
    NaturalRight,
    NaturalFull,
    Semi,
    Anti,
    LeftSemi,
    LeftAnti,
    RightSemi,
    RightAnti,
    CrossApply,
    OuterApply,
    AsOf,
    AsOfLeft,
    AsOfRight,
    Lateral,
    LeftLateral,
    Straight,
    Implicit,
    Array,
    LeftArray,
    Paste,
    Positional,
}

/// Parenthesized table expression with joins: `(t1 CROSS JOIN t2)`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinedTable {
    /// The left-hand side table expression
    pub left: Expression,
    /// The joins applied to the left table
    pub joins: Vec<Join>,
    /// LATERAL VIEW clauses (Hive/Spark)
    pub lateral_views: Vec<LateralView>,
    /// Optional alias for the joined table expression
    pub alias: Option<Identifier>,
}

/// Represent a WHERE clause containing a boolean filter predicate.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Where {
    /// The filter predicate expression.
    pub this: Expression,
}

/// GROUP BY with optional ALL/DISTINCT and WITH TOTALS; items include ROLLUP, CUBE and `()`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupBy {
    /// The grouping expressions.
    pub expressions: Vec<Expression>,
    /// GROUP BY modifier: Some(true) = ALL, Some(false) = DISTINCT, None = no modifier
    #[serde(default)]
    pub all: Option<bool>,
    /// ClickHouse: WITH TOTALS modifier
    #[serde(default)]
    pub totals: bool,
    /// Leading comments that appeared before the GROUP BY keyword
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

/// Represent a HAVING clause containing a predicate over aggregate results.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Having {
    /// The filter predicate, typically involving aggregate functions.
    pub this: Expression,
    /// Leading comments that appeared before the HAVING keyword
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

/// Represent an ORDER BY clause containing one or more sort specifications.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBy {
    /// The sort specifications, each with direction and null ordering.
    pub expressions: Vec<Ordered>,
    /// Whether this is ORDER SIBLINGS BY (Oracle hierarchical queries)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub siblings: bool,
    /// Leading comments that appeared before the ORDER BY keyword
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

/// A sort key: ascending unless `desc`; `nulls_first` is `None` when unspecified.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ordered {
    /// Vertica lets the engine select null placement for aggregate/window ordering.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub nulls_auto: bool,
    /// The expression to sort by.
    pub this: Expression,
    /// Whether the sort direction is descending (true) or ascending (false).
    pub desc: bool,
    /// `Some(true)` = NULLS FIRST, `Some(false)` = NULLS LAST, `None` = unspecified.
    pub nulls_first: Option<bool>,
    /// Whether ASC was explicitly written (not just implied)
    #[serde(default)]
    pub explicit_asc: bool,
    /// ClickHouse WITH FILL clause
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_fill: Option<Box<WithFill>>,
}

impl Ordered {
    pub fn asc(expr: Expression) -> Self {
        Self {
            nulls_auto: false,
            this: expr,
            desc: false,
            nulls_first: None,
            explicit_asc: false,
            with_fill: None,
        }
    }

    pub fn desc(expr: Expression) -> Self {
        Self {
            nulls_auto: false,
            this: expr,
            desc: true,
            nulls_first: None,
            explicit_asc: false,
            with_fill: None,
        }
    }
}

/// DISTRIBUTE BY (Hive/Spark): how rows are distributed across reducers.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistributeBy {
    pub expressions: Vec<Expression>,
}

/// CLUSTER BY (Hive/Spark): DISTRIBUTE BY and SORT BY on the same columns.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterBy {
    pub expressions: Vec<Ordered>,
}

/// SORT BY (Hive/Spark): sorts within each reducer, not globally.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortBy {
    pub expressions: Vec<Ordered>,
}

/// LATERAL VIEW (Hive/Spark): unnests arrays and maps with EXPLODE, POSEXPLODE and the like.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LateralView {
    /// The table-generating function (EXPLODE, POSEXPLODE, etc.)
    pub this: Expression,
    /// Table alias for the generated table
    pub table_alias: Option<Identifier>,
    /// Column aliases for the generated columns
    pub column_aliases: Vec<Identifier>,
    /// OUTER keyword - preserve nulls when input is empty/null
    pub outer: bool,
}

/// Query hint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    pub expressions: Vec<HintExpression>,
}

/// Individual hint expression
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HintExpression {
    /// A structured hint directive with optional execution scope modifiers.
    Directive {
        name: Identifier,
        arguments: Option<Vec<Expression>>,
        remote: bool,
        cascade: bool,
    },
    /// Function-style hint: USE_HASH(table)
    Function { name: String, args: Vec<Expression> },
    /// Simple identifier hint: PARALLEL
    Identifier(String),
    /// Raw hint text (unparsed)
    Raw(String),
}

/// Pseudocolumn type
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum PseudocolumnType {
    Rownum,
    Rowid,
    Level,
    Sysdate,
    ObjectId,
    ObjectValue,
}

impl PseudocolumnType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PseudocolumnType::Rownum => "ROWNUM",
            PseudocolumnType::Rowid => "ROWID",
            PseudocolumnType::Level => "LEVEL",
            PseudocolumnType::Sysdate => "SYSDATE",
            PseudocolumnType::ObjectId => "OBJECT_ID",
            PseudocolumnType::ObjectValue => "OBJECT_VALUE",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "ROWNUM" => Some(PseudocolumnType::Rownum),
            "ROWID" => Some(PseudocolumnType::Rowid),
            "LEVEL" => Some(PseudocolumnType::Level),
            "SYSDATE" => Some(PseudocolumnType::Sysdate),
            "OBJECT_ID" => Some(PseudocolumnType::ObjectId),
            "OBJECT_VALUE" => Some(PseudocolumnType::ObjectValue),
            _ => None,
        }
    }
}

/// Pseudocolumn (Oracle ROWNUM, ROWID, LEVEL, ...), never quoted.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pseudocolumn {
    pub kind: PseudocolumnType,
}

impl Pseudocolumn {
    pub fn rownum() -> Self {
        Self {
            kind: PseudocolumnType::Rownum,
        }
    }

    pub fn rowid() -> Self {
        Self {
            kind: PseudocolumnType::Rowid,
        }
    }

    pub fn level() -> Self {
        Self {
            kind: PseudocolumnType::Level,
        }
    }
}

/// Oracle CONNECT BY clause for hierarchical queries
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connect {
    /// START WITH condition (optional, can come before or after CONNECT BY)
    pub start: Option<Expression>,
    /// CONNECT BY condition (required, contains PRIOR references)
    pub connect: Expression,
    /// NOCYCLE keyword to prevent infinite loops
    pub nocycle: bool,
}

/// Oracle PRIOR expression - references parent row's value in CONNECT BY
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prior {
    pub this: Expression,
}

/// Oracle CONNECT_BY_ROOT function - returns root row's column value
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectByRoot {
    pub this: Expression,
}

/// MATCH_RECOGNIZE clause for row pattern matching (Oracle/Snowflake/Presto/Trino)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchRecognize {
    /// Source table/expression
    pub this: Option<Box<Expression>>,
    /// PARTITION BY expressions
    pub partition_by: Option<Vec<Expression>>,
    /// ORDER BY expressions
    pub order_by: Option<Vec<Ordered>>,
    /// MEASURES definitions
    pub measures: Option<Vec<MatchRecognizeMeasure>>,
    /// Row semantics (ONE ROW PER MATCH, ALL ROWS PER MATCH, etc.)
    pub rows: Option<MatchRecognizeRows>,
    /// AFTER MATCH SKIP behavior
    pub after: Option<MatchRecognizeAfter>,
    /// PATTERN definition (stored as raw string for complex regex patterns)
    pub pattern: Option<String>,
    /// DEFINE clauses (pattern variable definitions)
    pub define: Option<Vec<(Identifier, Expression)>>,
    /// Optional alias for the result
    pub alias: Option<Identifier>,
    /// Whether AS keyword was explicitly present before alias
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub alias_explicit_as: bool,
}

/// MEASURES expression with optional RUNNING/FINAL semantics
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchRecognizeMeasure {
    /// The measure expression
    pub this: Expression,
    /// RUNNING or FINAL semantics (Snowflake-specific)
    pub window_frame: Option<MatchRecognizeSemantics>,
}

/// Semantics for MEASURES in MATCH_RECOGNIZE
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum MatchRecognizeSemantics {
    Running,
    Final,
}

/// Row output semantics for MATCH_RECOGNIZE
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchRecognizeRows {
    OneRowPerMatch,
    AllRowsPerMatch,
    AllRowsPerMatchShowEmptyMatches,
    AllRowsPerMatchOmitEmptyMatches,
    AllRowsPerMatchWithUnmatchedRows,
}

/// AFTER MATCH SKIP behavior
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchRecognizeAfter {
    PastLastRow,
    ToNextRow,
    ToFirst(Identifier),
    ToLast(Identifier),
}

/// Represent a LIMIT clause that restricts the number of returned rows.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Limit {
    /// The limit count expression.
    pub this: Expression,
    /// Whether PERCENT modifier is present (DuckDB: LIMIT 10 PERCENT)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub percent: bool,
    /// Comments from before the LIMIT keyword (emitted after the limit value)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

impl Limit {
    /// Preserve the legacy count-only set-operation AST when no metadata is needed.
    pub(crate) fn into_set_limit(self) -> Box<Expression> {
        if self.percent || !self.comments.is_empty() {
            Box::new(Expression::Limit(Box::new(self)))
        } else {
            Box::new(self.this)
        }
    }
}

/// OFFSET clause
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Offset {
    pub this: Expression,
    /// Whether ROW/ROWS keyword was used (SQL standard syntax)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rows: Option<bool>,
}

/// TOP clause (SQL Server)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Top {
    pub this: Expression,
    pub percent: bool,
    pub with_ties: bool,
    /// Whether the expression was parenthesized: TOP (10) vs TOP 10
    #[serde(default)]
    pub parenthesized: bool,
}

/// FETCH FIRST/NEXT clause (SQL standard)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fetch {
    /// FIRST or NEXT
    pub direction: String,
    /// Count expression (optional)
    pub count: Option<Expression>,
    /// PERCENT modifier
    pub percent: bool,
    /// ROWS or ROW keyword present
    pub rows: bool,
    /// WITH TIES modifier
    pub with_ties: bool,
}

/// QUALIFY: a filter on window function results.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Qualify {
    /// The filter predicate over window function results.
    pub this: Expression,
}

/// SAMPLE / TABLESAMPLE clause
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub method: SampleMethod,
    pub size: Expression,
    pub seed: Option<Expression>,
    /// ClickHouse OFFSET expression after SAMPLE size
    #[serde(default)]
    pub offset: Option<Expression>,
    /// Whether the unit comes after the size (e.g., "100 ROWS" vs "ROW 100")
    pub unit_after_size: bool,
    /// Whether the keyword was SAMPLE (true) or TABLESAMPLE (false)
    #[serde(default)]
    pub use_sample_keyword: bool,
    /// Whether the method was explicitly specified (BERNOULLI, SYSTEM, etc.)
    #[serde(default)]
    pub explicit_method: bool,
    /// Whether the method keyword appeared before the size (TABLESAMPLE BERNOULLI (10))
    #[serde(default)]
    pub method_before_size: bool,
    /// Whether SEED keyword was used (true) or REPEATABLE (false)
    #[serde(default)]
    pub use_seed_keyword: bool,
    /// BUCKET numerator for Hive bucket sampling (BUCKET 1 OUT OF 5)
    pub bucket_numerator: Option<Box<Expression>>,
    /// BUCKET denominator (the 5 in BUCKET 1 OUT OF 5)
    pub bucket_denominator: Option<Box<Expression>>,
    /// BUCKET field for ON clause (BUCKET 1 OUT OF 5 ON x)
    pub bucket_field: Option<Box<Expression>>,
    /// Whether this is a DuckDB USING SAMPLE clause (vs SAMPLE/TABLESAMPLE)
    #[serde(default)]
    pub is_using_sample: bool,
    /// Whether the unit was explicitly PERCENT (vs ROWS)
    #[serde(default)]
    pub is_percent: bool,
    /// Whether to suppress method output (for cross-dialect transpilation)
    #[serde(default)]
    pub suppress_method_output: bool,
}

/// Sample method
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum SampleMethod {
    Bernoulli,
    System,
    Block,
    Row,
    Percent,
    /// Hive bucket sampling
    Bucket,
    /// DuckDB reservoir sampling
    Reservoir,
}

/// Named window definition (WINDOW w AS (...))
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedWindow {
    pub name: Identifier,
    pub spec: Over,
}

/// A WITH clause of one or more CTEs; `recursive` makes it WITH RECURSIVE.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct With {
    /// The list of CTE definitions, in order.
    pub ctes: Vec<Cte>,
    /// Whether the WITH RECURSIVE keyword was used.
    pub recursive: bool,
    /// Leading comments before the statement
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// SEARCH/CYCLE clause for recursive CTEs (PostgreSQL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<Box<Expression>>,
}

/// One CTE: name, optional column list, body, and optional [NOT] MATERIALIZED hint.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cte {
    /// The CTE name.
    pub alias: Identifier,
    /// The CTE body (typically a SELECT, UNION, etc.).
    pub this: Expression,
    /// Optional column alias list: `cte_name(c1, c2) AS (...)`.
    pub columns: Vec<Identifier>,
    /// `Some(true)` = MATERIALIZED, `Some(false)` = NOT MATERIALIZED, `None` = unspecified.
    pub materialized: Option<bool>,
    /// USING KEY (columns) for DuckDB recursive CTEs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_expressions: Vec<Identifier>,
    /// ClickHouse supports expression-first WITH items: WITH <expr> AS <alias>
    #[serde(default)]
    pub alias_first: bool,
    /// Comments associated with this CTE (placed after alias name, before AS)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// Window specification
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowSpec {
    pub partition_by: Vec<Expression>,
    pub order_by: Vec<Ordered>,
    pub frame: Option<WindowFrame>,
}

/// OVER clause
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Over {
    /// Named window reference (e.g., OVER w or OVER (w ORDER BY x))
    pub window_name: Option<Identifier>,
    pub partition_by: Vec<Expression>,
    pub order_by: Vec<Ordered>,
    pub frame: Option<WindowFrame>,
    pub alias: Option<Identifier>,
}

/// Window frame
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowFrame {
    pub kind: WindowFrameKind,
    pub start: WindowFrameBound,
    pub end: Option<WindowFrameBound>,
    pub exclude: Option<WindowFrameExclude>,
    /// Original text of the frame kind keyword (preserves input case, e.g. "range")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_text: Option<String>,
    /// Original text of the start bound side keyword (e.g. "preceding")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_side_text: Option<String>,
    /// Original text of the end bound side keyword
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_side_text: Option<String>,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum WindowFrameKind {
    Rows,
    Range,
    Groups,
}

/// EXCLUDE clause for window frames
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum WindowFrameExclude {
    CurrentRow,
    Group,
    Ties,
    NoOthers,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowFrameBound {
    CurrentRow,
    UnboundedPreceding,
    UnboundedFollowing,
    Preceding(Box<Expression>),
    Following(Box<Expression>),
    /// Bare PRECEDING without value (inverted syntax: just "PRECEDING")
    BarePreceding,
    /// Bare FOLLOWING without value (inverted syntax: just "FOLLOWING")
    BareFollowing,
    /// Bare numeric bound without PRECEDING/FOLLOWING (e.g., RANGE BETWEEN 1 AND 3)
    Value(Box<Expression>),
}
