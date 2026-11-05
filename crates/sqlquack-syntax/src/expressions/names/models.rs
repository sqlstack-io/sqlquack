//! Data models of expressions names.

use serde::Deserialize;
use serde::Serialize;
use std::fmt;

use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::Changes;
use crate::expressions::queries::models::Sample;
use crate::expressions::statements::models::HistoricalData;
use crate::expressions::statements::models::Version;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::tokens::models::Span;

/// A literal, keeping numbers as written so precision, trailing zeros and hex survive.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "literal_type", content = "value", rename_all = "snake_case")]
pub enum Literal {
    /// Single-quoted string literal: `'hello'`
    String(String),
    /// Numeric literal, stored as the original text: `42`, `3.14`, `1e10`
    Number(String),
    /// Hex string literal: `X'FF'`
    HexString(String),
    /// Hex number: 0xA, 0xFF (BigQuery, SQLite style) - represents an integer in hex notation
    HexNumber(String),
    BitString(String),
    /// Byte string: b"..." (BigQuery style)
    ByteString(String),
    /// National string: N'abc'
    NationalString(String),
    /// DATE literal: DATE '2024-01-15'
    Date(String),
    /// TIME literal: TIME '10:30:00'
    Time(String),
    /// TIMESTAMP literal: TIMESTAMP '2024-01-15 10:30:00'
    Timestamp(String),
    /// DATETIME literal: DATETIME '2024-01-15 10:30:00' (BigQuery)
    Datetime(String),
    /// Triple-quoted string (`"""..."""` or `'''...'''`): content and quote character.
    TripleQuotedString(String, char),
    /// Escape string: E'...' (PostgreSQL)
    EscapeString(String),
    /// Dollar-quoted string: $$...$$  (PostgreSQL)
    DollarString(String),
    /// Raw string r"..." (BigQuery, Spark, Databricks): backslashes are literal, not escapes.
    RawString(String),
}

impl Literal {
    /// Returns the inner value as a string slice, regardless of literal type.
    pub fn value_str(&self) -> &str {
        match self {
            Literal::String(s)
            | Literal::Number(s)
            | Literal::HexString(s)
            | Literal::HexNumber(s)
            | Literal::BitString(s)
            | Literal::ByteString(s)
            | Literal::NationalString(s)
            | Literal::Date(s)
            | Literal::Time(s)
            | Literal::Timestamp(s)
            | Literal::Datetime(s)
            | Literal::EscapeString(s)
            | Literal::DollarString(s)
            | Literal::RawString(s) => s.as_str(),
            Literal::TripleQuotedString(s, _) => s.as_str(),
        }
    }

    /// Returns `true` if this is a string-type literal.
    pub fn is_string(&self) -> bool {
        matches!(
            self,
            Literal::String(_)
                | Literal::NationalString(_)
                | Literal::EscapeString(_)
                | Literal::DollarString(_)
                | Literal::RawString(_)
                | Literal::TripleQuotedString(_, _)
        )
    }

    /// Returns `true` if this is a numeric literal.
    pub fn is_number(&self) -> bool {
        matches!(self, Literal::Number(_) | Literal::HexNumber(_))
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::String(s) => write!(f, "'{}'", s),
            Literal::Number(n) => write!(f, "{}", n),
            Literal::HexString(h) => write!(f, "X'{}'", h),
            Literal::HexNumber(h) => write!(f, "0x{}", h),
            Literal::BitString(b) => write!(f, "B'{}'", b),
            Literal::ByteString(b) => write!(f, "b'{}'", b),
            Literal::NationalString(s) => write!(f, "N'{}'", s),
            Literal::Date(d) => write!(f, "DATE '{}'", d),
            Literal::Time(t) => write!(f, "TIME '{}'", t),
            Literal::Timestamp(ts) => write!(f, "TIMESTAMP '{}'", ts),
            Literal::Datetime(dt) => write!(f, "DATETIME '{}'", dt),
            Literal::TripleQuotedString(s, q) => {
                write!(f, "{0}{0}{0}{1}{0}{0}{0}", q, s)
            }
            Literal::EscapeString(s) => write!(f, "E'{}'", s),
            Literal::DollarString(s) => write!(f, "$${}$$", s),
            Literal::RawString(s) => write!(f, "r'{}'", s),
        }
    }
}

/// Boolean literal
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BooleanLiteral {
    pub value: bool,
}

/// NULL literal
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Null;

/// The DEFAULT keyword where a value is expected, such as a VALUES item: the column's default.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultKeyword;

/// An identifier (table, column, alias, ...); `quoted` records whether it was delimited.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    /// The raw text of the identifier, without any quoting characters.
    pub name: String,
    /// Whether the identifier was quoted in the source SQL.
    pub quoted: bool,
    #[serde(default)]
    pub trailing_comments: Vec<String>,
    /// Source range including quote delimiters; absent when synthetic, ignored by equality.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.quoted == other.quoted
            && self.trailing_comments == other.trailing_comments
    }
}

impl Identifier {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            quoted: false,
            trailing_comments: Vec::new(),
            span: None,
        }
    }

    pub fn quoted(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            quoted: true,
            trailing_comments: Vec::new(),
            span: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            name: String::new(),
            quoted: false,
            trailing_comments: Vec::new(),
            span: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    /// The legacy string form of a named type field, quoted with SQL-standard delimiters.
    pub(crate) fn to_type_field_name(&self) -> String {
        if self.quoted {
            format!("\"{}\"", self.name.replace('"', "\"\""))
        } else {
            self.name.clone()
        }
    }

    /// Set the source span on this identifier
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.quoted {
            write!(f, "\"{}\"", self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// A column reference, optionally qualified by a table: `name` or `table.name`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    /// The column name.
    pub name: Identifier,
    /// Optional table qualifier (e.g. `t` in `t.col`).
    pub table: Option<Identifier>,
    /// Oracle-style join marker (+) for outer joins
    #[serde(default)]
    pub join_mark: bool,
    /// Trailing comments that appeared after this column reference
    #[serde(default)]
    pub trailing_comments: Vec<String>,
    /// Source range including any qualifier; absent when synthetic, ignored by equality.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.table == other.table
            && self.join_mark == other.join_mark
            && self.trailing_comments == other.trailing_comments
            && self.inferred_type == other.inferred_type
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(table) = &self.table {
            write!(f, "{}.{}", table, self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// A table reference with optional schema and catalog, alias, time travel and hints.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRef {
    /// The unqualified table name.
    pub name: Identifier,
    /// Optional schema qualifier (e.g. `public` in `public.users`).
    pub schema: Option<Identifier>,
    /// Optional catalog qualifier (e.g. `mydb` in `mydb.public.users`).
    pub catalog: Option<Identifier>,
    /// Optional table alias (e.g. `t` in `FROM users AS t`).
    pub alias: Option<Identifier>,
    /// Whether AS keyword was explicitly used for the alias
    #[serde(default)]
    pub alias_explicit_as: bool,
    /// Column aliases for table alias: AS t(c1, c2)
    #[serde(default)]
    pub column_aliases: Vec<Identifier>,
    /// Leading comments that appeared before this table reference in a FROM clause
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leading_comments: Vec<String>,
    /// Trailing comments that appeared after this table reference
    #[serde(default)]
    pub trailing_comments: Vec<String>,
    /// Snowflake time travel: BEFORE (STATEMENT => ...) or AT (TIMESTAMP => ...)
    #[serde(default)]
    pub when: Option<Box<HistoricalData>>,
    /// PostgreSQL ONLY modifier: prevents scanning child tables in inheritance hierarchy
    #[serde(default)]
    pub only: bool,
    /// ClickHouse FINAL modifier: forces final aggregation for MergeTree tables
    #[serde(default)]
    pub final_: bool,
    /// TABLESAMPLE clause attached to this table reference (DuckDB, BigQuery)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_sample: Option<Box<Sample>>,
    /// TSQL table hints: WITH (TABLOCK, INDEX(myindex), ...)
    #[serde(default)]
    pub hints: Vec<Expression>,
    /// TSQL FOR SYSTEM_TIME clause text, such as "FOR SYSTEM_TIME BETWEEN c AND d".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_time: Option<String>,
    /// MySQL: PARTITION(p0, p1, ...) hint for reading from specific partitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub partitions: Vec<Identifier>,
    /// Snowflake IDENTIFIER(): a table name from a string or variable, used instead of `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier_func: Option<Box<Expression>>,
    /// Snowflake CHANGES clause: CHANGES (INFORMATION => ...) AT (...) END (...)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changes: Option<Box<Changes>>,
    /// Time travel version clause: FOR VERSION AS OF / FOR TIMESTAMP AS OF (Presto/Trino, BigQuery, Databricks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<Box<Version>>,
    /// Source position span
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

impl TableRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            schema: None,
            catalog: None,
            alias: None,
            alias_explicit_as: false,
            column_aliases: Vec::new(),
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
            when: None,
            only: false,
            final_: false,
            table_sample: None,
            hints: Vec::new(),
            system_time: None,
            partitions: Vec::new(),
            identifier_func: None,
            changes: None,
            version: None,
            span: None,
        }
    }

    /// Create with a schema qualifier.
    pub fn new_with_schema(name: impl Into<String>, schema: impl Into<String>) -> Self {
        let mut t = Self::new(name);
        t.schema = Some(Identifier::new(schema));
        t
    }

    /// Create with catalog and schema qualifiers.
    pub fn new_with_catalog(
        name: impl Into<String>,
        schema: impl Into<String>,
        catalog: impl Into<String>,
    ) -> Self {
        let mut t = Self::new(name);
        t.schema = Some(Identifier::new(schema));
        t.catalog = Some(Identifier::new(catalog));
        t
    }

    /// Create from an Identifier, preserving the quoted flag
    pub fn from_identifier(name: Identifier) -> Self {
        Self {
            name,
            schema: None,
            catalog: None,
            alias: None,
            alias_explicit_as: false,
            column_aliases: Vec::new(),
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
            when: None,
            only: false,
            final_: false,
            table_sample: None,
            hints: Vec::new(),
            system_time: None,
            partitions: Vec::new(),
            identifier_func: None,
            changes: None,
            version: None,
            span: None,
        }
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(Identifier::new(alias));
        self
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(Identifier::new(schema));
        self
    }
}

/// A star (`*`, `table.*`) with optional ILIKE or EXCEPT/EXCLUDE, then REPLACE and RENAME modifiers.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Star {
    /// Optional table qualifier (e.g. `t` in `t.*`).
    pub table: Option<Identifier>,
    /// Snowflake's ILIKE pattern (`* ILIKE '%_ID'`), the string literal's value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ilike: Option<String>,
    /// EXCLUDE / EXCEPT columns (DuckDB, BigQuery, Snowflake)
    pub except: Option<Vec<Identifier>>,
    /// REPLACE expressions (BigQuery, Snowflake)
    pub replace: Option<Vec<Alias>>,
    /// RENAME columns (Snowflake)
    pub rename: Option<Vec<(Identifier, Identifier)>>,
    /// Trailing comments that appeared after the star
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trailing_comments: Vec<String>,
    /// Source position span
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

/// ByteString
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ByteString {
    pub this: Box<Expression>,
    #[serde(default)]
    pub is_bytes: Option<Box<Expression>>,
}

/// Hex string expression (not a literal); BigQuery FROM_HEX.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HexStringExpr {
    pub this: Box<Expression>,
    #[serde(default)]
    pub is_integer: Option<bool>,
}

/// UnicodeString
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnicodeString {
    pub this: Box<Expression>,
    #[serde(default)]
    pub escape: Option<Box<Expression>>,
}
