//! Data models of expressions operators.

use crate::tokens::models::SourceSpan;
use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::queries::models::Ordered;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;

/// An aliased expression `expr AS name`, including column alias lists such as `AS t(c1, c2)`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alias {
    /// The expression being aliased.
    pub this: Expression,
    /// The alias name (required for simple aliases, optional when only column aliases provided)
    pub alias: Identifier,
    /// Optional column aliases for table-valued functions: AS t(col1, col2) or AS (col1, col2)
    #[serde(default)]
    pub column_aliases: Vec<Identifier>,
    /// Whether AS keyword was explicitly used for the alias.
    #[serde(default)]
    pub alias_explicit_as: bool,
    /// Original alias keyword spelling, e.g. `AS` vs `as`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias_keyword: Option<String>,
    /// Comments that appeared between the expression and AS keyword
    #[serde(default)]
    pub pre_alias_comments: Vec<String>,
    /// Trailing comments that appeared after the alias
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

impl Alias {
    /// Create a simple alias
    pub fn new(this: Expression, alias: Identifier) -> Self {
        Self {
            this,
            alias,
            column_aliases: Vec::new(),
            alias_explicit_as: false,
            alias_keyword: None,
            pre_alias_comments: Vec::new(),
            trailing_comments: Vec::new(),
            inferred_type: None,
            span: SourceSpan::default(),
        }
    }

    /// Create an alias with column aliases only (no table alias name)
    pub fn with_columns(this: Expression, column_aliases: Vec<Identifier>) -> Self {
        Self {
            this,
            alias: Identifier::empty(),
            column_aliases,
            alias_explicit_as: false,
            alias_keyword: None,
            pre_alias_comments: Vec::new(),
            trailing_comments: Vec::new(),
            inferred_type: None,
            span: SourceSpan::default(),
        }
    }
}

/// A cast: `CAST(expr AS type)` or `expr::type`; also the payload of TryCast and SafeCast.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cast {
    /// The expression being cast.
    pub this: Expression,
    /// The target data type.
    pub to: DataType,
    #[serde(default)]
    pub trailing_comments: Vec<String>,
    /// Whether PostgreSQL `::` syntax was used (true) vs CAST() function (false)
    #[serde(default)]
    pub double_colon_syntax: bool,
    /// SQLQuack: written as a typed literal, `TYPE '...'`; read by the Snowflake refusals only.
    #[serde(skip)]
    #[ast(skip)]
    pub typed_literal: bool,
    /// FORMAT clause for BigQuery: CAST(x AS STRING FORMAT 'format_string')
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<Box<Expression>>,
    /// DEFAULT value ON CONVERSION ERROR (Oracle): CAST(x AS type DEFAULT val ON CONVERSION ERROR)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub default: Option<Box<Expression>>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

impl Cast {
    /// A cast of `this` to `to`, written with `::` when `double_colon_syntax`, else as CAST.
    pub fn new(this: Expression, to: DataType, double_colon_syntax: bool) -> Self {
        Self {
            this,
            to,
            trailing_comments: Vec::new(),
            double_colon_syntax,
            typed_literal: false,
            format: None,
            default: None,
            inferred_type: None,
            span: SourceSpan::default(),
        }
    }
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollationExpr {
    pub this: Expression,
    pub collation: String,
    /// True if the collation was single-quoted in the original SQL (string literal)
    #[serde(default)]
    pub quoted: bool,
    /// True if the collation was double-quoted in the original SQL (identifier)
    #[serde(default)]
    pub double_quoted: bool,
}

/// A CASE expression: simple when `operand` is set, searched otherwise; `whens` pairs.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Case {
    /// The operand for simple CASE, or `None` for searched CASE.
    pub operand: Option<Expression>,
    /// Pairs of (WHEN condition, THEN result).
    pub whens: Vec<(Expression, Expression)>,
    /// Optional ELSE result.
    pub else_: Option<Expression>,
    /// Comments from the CASE keyword (emitted after END)
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// The payload of every binary operator variant: arithmetic, comparison, logical, bitwise.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryOp {
    pub left: Expression,
    pub right: Expression,
    /// Comments after the left operand (before the operator)
    #[serde(default)]
    pub left_comments: Vec<String>,
    /// Comments after the operator (before the right operand)
    #[serde(default)]
    pub operator_comments: Vec<String>,
    /// Comments after the right operand
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

impl BinaryOp {
    pub fn new(left: Expression, right: Expression) -> Self {
        Self {
            left,
            right,
            left_comments: Vec::new(),
            operator_comments: Vec::new(),
            trailing_comments: Vec::new(),
            inferred_type: None,
            span: SourceSpan::default(),
        }
    }
}

/// LIKE/ILIKE operation with optional ESCAPE clause and quantifier (ANY/ALL)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LikeOp {
    /// Infix NOT LIKE, negating each comparison before ANY/ALL reduction.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub negated: bool,
    pub left: Expression,
    pub right: Expression,
    /// ESCAPE character/expression
    #[serde(default)]
    pub escape: Option<Expression>,
    /// Quantifier: ANY, ALL, or SOME
    #[serde(default)]
    pub quantifier: Option<String>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

impl LikeOp {
    pub fn new(left: Expression, right: Expression) -> Self {
        Self {
            negated: false,
            left,
            right,
            escape: None,
            quantifier: None,
            inferred_type: None,
            span: SourceSpan::default(),
        }
    }

    pub fn with_escape(left: Expression, right: Expression, escape: Expression) -> Self {
        Self {
            negated: false,
            left,
            right,
            escape: Some(escape),
            quantifier: None,
            inferred_type: None,
            span: SourceSpan::default(),
        }
    }
}

/// The payload of the unary operator variants `Not`, `Neg` and `BitwiseNot`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnaryOp {
    /// The operand expression.
    pub this: Expression,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

impl UnaryOp {
    pub fn new(this: Expression) -> Self {
        Self {
            this,
            inferred_type: None,
            span: SourceSpan::default(),
        }
    }
}

/// An IN predicate over a value list or a subquery; `not` makes it NOT IN.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct In {
    /// The expression being tested.
    pub this: Expression,
    /// The value list (mutually exclusive with `query`).
    pub expressions: Vec<Expression>,
    /// A subquery (mutually exclusive with `expressions`).
    pub query: Option<Expression>,
    /// Whether this is NOT IN.
    pub not: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub global: bool,
    /// BigQuery: IN UNNEST(expr)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unnest: Option<Box<Expression>>,
    /// Whether the right side is a bare reference without parentheses: `a IN subquery1`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_field: bool,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// Represent a BETWEEN predicate (`x BETWEEN low AND high`).
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Between {
    /// The expression being tested.
    pub this: Expression,
    /// The lower bound.
    pub low: Expression,
    /// The upper bound.
    pub high: Expression,
    /// Whether this is NOT BETWEEN.
    pub not: bool,
    /// SYMMETRIC/ASYMMETRIC qualifier: None = regular, Some(true) = SYMMETRIC, Some(false) = ASYMMETRIC
    #[serde(default)]
    pub symmetric: Option<bool>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// IS NULL predicate
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsNull {
    pub this: Expression,
    pub not: bool,
    /// Whether this was the postfix form (ISNULL/NOTNULL) vs standard (IS NULL/IS NOT NULL)
    #[serde(default)]
    pub postfix_form: bool,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// IS TRUE / IS FALSE predicate
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsTrueFalse {
    pub this: Expression,
    pub not: bool,
}

/// IS JSON predicate (SQL standard): whether a value is valid JSON.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsJson {
    pub this: Expression,
    /// JSON type: VALUE, SCALAR, OBJECT, or ARRAY (None = just IS JSON)
    pub json_type: Option<String>,
    /// Key uniqueness constraint
    pub unique_keys: Option<JsonUniqueKeys>,
    /// Whether IS NOT JSON
    pub negated: bool,
}

/// PostgreSQL: a call written with keywords, e.g. `JSON_VALUE(j, '$' RETURNING int)` or `x IS DOCUMENT`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeywordCall {
    /// The call's keywords, e.g. "JSON_VALUE", "XMLELEMENT", "IS DOCUMENT", "FORMAT JSON"
    pub kind: String,
    /// The call's operands in order, e.g. the context item and the path
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operands: Vec<Expression>,
    /// The call's clauses in order, e.g. `(RETURNING, int)` or `(ON ERROR, NULL)`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Expression>,
}

/// JSON unique keys constraint variants
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsonUniqueKeys {
    /// WITH UNIQUE KEYS
    With,
    /// WITHOUT UNIQUE KEYS
    Without,
    /// UNIQUE KEYS (shorthand for WITH UNIQUE KEYS)
    Shorthand,
}

/// Represent an EXISTS predicate (`EXISTS (SELECT ...)` or `NOT EXISTS (...)`).
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exists {
    /// The subquery expression.
    pub this: Expression,
    /// Whether this is NOT EXISTS.
    pub not: bool,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Upsert {
    /// Retained source semantics; serialized independently of generator configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    pub table: TableRef,
    pub partition: Option<Expression>,
    pub columns: Vec<Identifier>,
    pub source: Expression,
    pub condition: Option<Expression>,
    pub primary_key: bool,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionSpec {
    /// Retained source semantics; serialized independently of generator configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    pub method: String,
    pub columns: Vec<Expression>,
    pub partitions: Option<Expression>,
    pub ranges: Vec<PartitionRangeSpec>,
    pub primary_key_check: Option<bool>,
    pub properties: Vec<Expression>,
    pub subpartition: Option<Box<PartitionSpec>>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionRangeSpec {
    pub name: Option<Identifier>,
    pub kind: String,
    pub values: Vec<Expression>,
    pub dynamic: bool,
    pub direction: Option<String>,
    pub threshold: Option<Expression>,
    pub interval: Option<(Expression, Option<String>)>,
    pub properties: Vec<Expression>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageProperty {
    /// Retained source semantics; serialized independently of generator configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    pub name: String,
    pub values: Vec<Expression>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hierarchy {
    /// Retained source semantics; serialized independently of generator configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    pub name: String,
    pub source: Expression,
    pub start: Option<Expression>,
    pub siblings: Vec<Ordered>,
    pub depth: Option<Expression>,
    pub multiparent: Option<String>,
    pub orphan: Option<String>,
    pub cycle: Option<String>,
    pub cache: Option<String>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewParameter {
    /// Retained source semantics; serialized independently of generator configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    pub name: Identifier,
    pub value: Expression,
}

#[derive(
    sqlquack_derive::AstNode, Debug, Clone, Default, PartialEq, Serialize, Deserialize,
)]
pub struct JsonOptions {
    pub wrapper: Option<String>,
    pub on_empty: Option<SqlBehavior>,
    pub on_error: Option<SqlBehavior>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlBehavior {
    /// ERROR, NULL, DEFAULT, EMPTY ARRAY, or EMPTY OBJECT.
    pub kind: String,
    pub value: Option<Expression>,
}

/// Procedure invocation, including library member calls and asynchronous execution.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    /// Retained source semantics; serialized independently of generator configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    pub name: Vec<Identifier>,
    pub member: Option<Identifier>,
    pub arguments: Vec<Expression>,
    pub asynchronous: bool,
    pub hints: Vec<Expression>,
    /// Snowflake `WITH ... CALL`: the CTEs and anonymous procedures the call can name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with: Option<crate::expressions::queries::models::With>,
    /// Snowflake scripting `INTO :variable` target of the call's result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub into: Option<Identifier>,
}

/// Regex search options shared by predicates, extraction, replacement, and position calls.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    /// Original spelling when a dialect has more than one native alias.
    pub source_name: String,
    #[serde(default)]
    pub negated: bool,
    pub start: Option<Expression>,
    pub occurrence: Option<Expression>,
    pub flags: Option<Expression>,
    pub position_after: Option<bool>,
}

/// Grouping-set selection and result-delivery options.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupingOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    pub best: Option<Expression>,
    pub limit: Option<Expression>,
    pub offset: Option<Expression>,
    pub subtotal: bool,
    pub balance: bool,
    pub total: bool,
    pub structured: bool,
    pub overview: bool,
    pub prefix: Option<Expression>,
    pub multiple_resultsets: bool,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultSerialization {
    /// JSON or XML, validated by the HANA parser.
    pub format: String,
    pub options: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returning: Option<DataType>,
}

/// Lambda expression
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LambdaExpr {
    pub parameters: Vec<Identifier>,
    pub body: Expression,
    /// True if using DuckDB's LAMBDA x : expr syntax (vs x -> expr)
    #[serde(default)]
    pub colon: bool,
    /// Parameter types by index, for typed lambdas (Snowflake `a int -> a + 1`).
    #[serde(default)]
    pub parameter_types: Vec<Option<DataType>>,
}

/// Parameter (parameterized queries)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: Option<String>,
    pub index: Option<u32>,
    pub style: ParameterStyle,
    /// Whether the name was quoted (e.g., @"x" vs @x)
    #[serde(default)]
    pub quoted: bool,
    /// Whether the name was string-quoted with single quotes (e.g., @'foo')
    #[serde(default)]
    pub string_quoted: bool,
    /// Optional secondary expression for ${kind:name} syntax (Hive hiveconf variables)
    #[serde(default)]
    pub expression: Option<String>,
}

/// Parameter placeholder styles
#[derive(
    sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum ParameterStyle {
    Question,
    Dollar,
    DollarBrace,
    Brace,
    Colon,
    At,
    DoubleAt,
    DoubleDollar,
    Percent,
}

/// Placeholder expression
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placeholder {
    pub index: Option<u32>,
}

/// Named argument in function call: name => value or name := value
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedArgument {
    pub name: Identifier,
    pub value: Expression,
    /// The separator used: `=>`, `:=`, or `=`
    pub separator: NamedArgSeparator,
}

/// Separator style for named arguments
#[derive(
    sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum NamedArgSeparator {
    /// `=>` (standard SQL, Snowflake, BigQuery)
    DArrow,
    /// `:=` (Oracle, MySQL)
    ColonEq,
    /// `=` (simple equals, some dialects)
    Eq,
}

/// TABLE or MODEL reference as a function argument (BigQuery).
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableArgument {
    /// The keyword prefix: "TABLE" or "MODEL"
    pub prefix: String,
    /// The table/model reference expression
    pub this: Expression,
}

/// SQL Comment preservation
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlComment {
    pub text: String,
    pub is_block: bool,
}

/// SIMILAR TO expression
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarToExpr {
    pub this: Expression,
    pub pattern: Expression,
    pub escape: Option<Expression>,
    pub not: bool,
}

/// ANY / ALL quantified expression
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantifiedExpr {
    pub this: Expression,
    pub subquery: Expression,
    pub op: Option<QuantifiedOp>,
}

/// Comparison operator for quantified expressions
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuantifiedOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

/// OVERLAPS: binary `a OVERLAPS b`, or ANSI `(a, b) OVERLAPS (c, d)`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlapsExpr {
    /// Left operand for simple binary form
    #[serde(skip_serializing_if = "Option::is_none")]
    pub this: Option<Expression>,
    /// Right operand for simple binary form
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<Expression>,
    /// Left range start for full ANSI form
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_start: Option<Expression>,
    /// Left range end for full ANSI form
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_end: Option<Expression>,
    /// Right range start for full ANSI form
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_start: Option<Expression>,
    /// Right range end for full ANSI form
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_end: Option<Expression>,
}

/// Subscript access (array[index] or map[key])
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subscript {
    pub this: Expression,
    pub index: Expression,
}

/// Dot access (struct.field)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DotAccess {
    pub this: Expression,
    pub field: Identifier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inferred_type: Option<DataType>,
}

/// Method call (expr.method(args))
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodCall {
    pub this: Expression,
    pub method: Identifier,
    pub args: Vec<Expression>,
    /// Whether DISTINCT was specified inside the call.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub distinct: bool,
    /// ORDER BY inside the call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_by: Vec<Ordered>,
}

impl MethodCall {
    /// A method call with no `DISTINCT` and no argument `ORDER BY`.
    pub fn new(this: Expression, method: Identifier, args: Vec<Expression>) -> Self {
        Self {
            this,
            method,
            args,
            distinct: false,
            order_by: Vec::new(),
        }
    }
}

/// Array slice (array[start:end])
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArraySlice {
    pub this: Expression,
    pub start: Option<Expression>,
    pub end: Option<Expression>,
}

/// Export
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Export {
    pub this: Box<Expression>,
    #[serde(default)]
    pub connection: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// Filter
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Changes
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Changes {
    #[serde(default)]
    pub information: Option<Box<Expression>>,
    #[serde(default)]
    pub at_before: Option<Box<Expression>>,
    #[serde(default)]
    pub end: Option<Box<Expression>>,
}

/// Directory
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Directory {
    pub this: Box<Expression>,
    #[serde(default)]
    pub local: Option<Box<Expression>>,
    #[serde(default)]
    pub row_format: Option<Box<Expression>>,
}

/// ForeignKey
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKey {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub reference: Option<Box<Expression>>,
    #[serde(default)]
    pub delete: Option<Box<Expression>>,
    #[serde(default)]
    pub update: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// ColumnPrefix
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnPrefix {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// PrimaryKey
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimaryKey {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub options: Vec<Expression>,
    #[serde(default)]
    pub include: Option<Box<Expression>>,
}

/// Into
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntoClause {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub temporary: bool,
    #[serde(default)]
    pub unlogged: Option<Box<Expression>>,
    #[serde(default)]
    pub bulk_collect: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// JoinHint
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinHint {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Opclass
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opclass {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Index
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Index {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub table: Option<Box<Expression>>,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub primary: Option<Box<Expression>>,
    #[serde(default)]
    pub amp: Option<Box<Expression>>,
    #[serde(default)]
    pub params: Vec<Expression>,
}

/// IndexParameters
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexParameters {
    #[serde(default)]
    pub using: Option<Box<Expression>>,
    #[serde(default)]
    pub include: Option<Box<Expression>>,
    #[serde(default)]
    pub columns: Vec<Expression>,
    #[serde(default)]
    pub with_storage: Option<Box<Expression>>,
    #[serde(default)]
    pub partition_by: Option<Box<Expression>>,
    #[serde(default)]
    pub tablespace: Option<Box<Expression>>,
    #[serde(default)]
    pub where_: Option<Box<Expression>>,
    #[serde(default)]
    pub on: Option<Box<Expression>>,
}

/// ConditionalInsert
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalInsert {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub else_: Option<Box<Expression>>,
}

/// MultitableInserts
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultitableInserts {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    pub kind: String,
    #[serde(default)]
    pub source: Option<Box<Expression>>,
    /// Leading comments before the statement
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// OVERWRITE modifier (Snowflake: INSERT OVERWRITE ALL)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub overwrite: bool,
}

/// OnConflict
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnConflict {
    #[serde(default)]
    pub duplicate: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub action: Option<Box<Expression>>,
    #[serde(default)]
    pub conflict_keys: Option<Box<Expression>>,
    #[serde(default)]
    pub index_predicate: Option<Box<Expression>>,
    #[serde(default)]
    pub constraint: Option<Box<Expression>>,
    #[serde(default)]
    pub where_: Option<Box<Expression>>,
}

/// OnCondition
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnCondition {
    #[serde(default)]
    pub error: Option<Box<Expression>>,
    #[serde(default)]
    pub empty: Option<Box<Expression>>,
    #[serde(default)]
    pub null: Option<Box<Expression>>,
}

/// Returning
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Returning {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub into: Option<Box<Expression>>,
}

/// Introducer
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Introducer {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// PartitionRange
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionRange {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Group
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub grouping_sets: Option<Box<Expression>>,
    #[serde(default)]
    pub cube: Option<Box<Expression>>,
    #[serde(default)]
    pub rollup: Option<Box<Expression>>,
    #[serde(default)]
    pub totals: Option<Box<Expression>>,
    /// GROUP BY modifier: Some(true) = ALL, Some(false) = DISTINCT, None = no modifier
    #[serde(default)]
    pub all: Option<bool>,
}

/// Cube
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cube {
    /// Selection, subtotal, and result-delivery options for this grouping operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<GroupingOptions>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Rollup
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rollup {
    /// Selection, subtotal, and result-delivery options for this grouping operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<GroupingOptions>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// GroupingSets
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupingSets {
    /// Selection, subtotal, and result-delivery options for this grouping operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<GroupingOptions>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// LimitOptions
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimitOptions {
    #[serde(default)]
    pub percent: Option<Box<Expression>>,
    #[serde(default)]
    pub rows: Option<Box<Expression>>,
    #[serde(default)]
    pub with_ties: Option<Box<Expression>>,
}

/// Lateral
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lateral {
    pub this: Box<Expression>,
    #[serde(default)]
    pub view: Option<Box<Expression>>,
    #[serde(default)]
    pub outer: Option<Box<Expression>>,
    #[serde(default)]
    pub alias: Option<String>,
    /// Whether the alias was originally quoted (backtick/double-quote)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub alias_quoted: bool,
    #[serde(default)]
    pub cross_apply: Option<Box<Expression>>,
    #[serde(default)]
    pub ordinality: Option<Box<Expression>>,
    /// Column aliases for the lateral expression (e.g., LATERAL func() AS alias(col1, col2))
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_aliases: Vec<String>,
    /// Quoting of each column alias, parallel to `column_aliases`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_aliases_quoted: Vec<bool>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// TableFromRows
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableFromRows {
    pub this: Box<Expression>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub joins: Vec<Expression>,
    #[serde(default)]
    pub pivots: Option<Box<Expression>>,
    #[serde(default)]
    pub sample: Option<Box<Expression>>,
}

/// PostgreSQL ROWS FROM (func(args) AS alias(...), ...) over set-returning functions.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowsFrom {
    /// List of function expressions, each potentially with an alias and typed columns
    pub expressions: Vec<Expression>,
    /// WITH ORDINALITY modifier
    #[serde(default)]
    pub ordinality: bool,
    /// Optional outer alias: ROWS FROM (...) AS alias(col1 type1, col2 type2)
    #[serde(default)]
    pub alias: Option<Box<Expression>>,
}

/// WithFill
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithFill {
    #[serde(default)]
    pub from_: Option<Box<Expression>>,
    #[serde(default)]
    pub to: Option<Box<Expression>>,
    #[serde(default)]
    pub step: Option<Box<Expression>>,
    #[serde(default)]
    pub staleness: Option<Box<Expression>>,
    #[serde(default)]
    pub interpolate: Option<Box<Expression>>,
}
