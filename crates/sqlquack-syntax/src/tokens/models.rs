//! Data models of tokens.

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::ops::Range;
use std::sync::Arc;

use crate::guard::models::TokenGuardStats;
use crate::tokens::_helpers::defaults::DEFAULT_COMMENTS;
use crate::tokens::_helpers::defaults::DEFAULT_IDENTIFIERS;
use crate::tokens::_helpers::defaults::DEFAULT_KEYWORDS;
use crate::tokens::_helpers::defaults::DEFAULT_QUOTES;
use crate::tokens::_helpers::defaults::DEFAULT_SINGLE_TOKENS;
use crate::tokens::errors::ValidationError;
use crate::tokens::types::Result;
use crate::tokens::types::SpanField;
use crate::tokens::types::TokenOutput;
use crate::tokens::types::TokenizerCursor;

/// A dialect SQLQuack parses: one per parser crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    /// Snowflake SQL.
    Snowflake,
    /// DuckDB SQL.
    DuckDB,
    /// PostgreSQL.
    PostgreSQL,
    /// SQLite.
    SQLite,
    /// BigQuery's GoogleSQL.
    BigQuery,
    /// ClickHouse SQL.
    ClickHouse,
    /// Databricks SQL.
    Databricks,
    /// SQL Server's Transact-SQL.
    SqlServer,
}

/// How the parser treats function calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallMode {
    /// Every Snowflake call keeps its name and arguments as written; SQLQuack's mode.
    Faithful,
    /// As [`CallMode::Faithful`] with call spans, for the exact check; skips statement-context rules.
    FaithfulWithSpans,
    /// polyglot's call lowering, for the exact check only; skips statement-context rules.
    Lowered,
    /// As [`CallMode::Faithful`], recording each node's source range (for refusal locations).
    FaithfulLocated,
}

/// Every supported SQL dialect; `Generic` is standard SQL and the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DialectType {
    /// Standard SQL with no dialect-specific behavior (default).
    #[default]
    Generic,
    /// Snowflake -- cloud data platform with QUALIFY clause, FLATTEN, and variant types.
    Snowflake,
    /// DuckDB -- in-process analytical database with modern SQL extensions.
    DuckDB,
    /// PostgreSQL -- `$$` strings, nested comments and its own operators.
    PostgreSQL,
    /// SQLite -- several identifier quotes, hex numbers, its own operator tiers.
    SQLite,
}

impl std::fmt::Display for DialectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialectType::Generic => write!(f, "generic"),

            DialectType::Snowflake => write!(f, "snowflake"),
            DialectType::DuckDB => write!(f, "duckdb"),
            DialectType::PostgreSQL => write!(f, "postgresql"),
            DialectType::SQLite => write!(f, "sqlite"),
        }
    }
}

impl std::str::FromStr for DialectType {
    type Err = crate::tokens::errors::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "generic" | "" => Ok(DialectType::Generic),

            "snowflake" => Ok(DialectType::Snowflake),
            "duckdb" => Ok(DialectType::DuckDB),
            "postgresql" | "postgres" => Ok(DialectType::PostgreSQL),
            "sqlite" => Ok(DialectType::SQLite),

            _ => Err(crate::tokens::errors::Error::parse(
                format!("Unknown dialect: {}", s),
                Span::default(),
            )),
        }
    }
}

/// The output-column selector that failed to resolve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ColumnResolutionTarget {
    /// Resolve an output column by name.
    Name { name: String },
    /// Resolve an output column by zero-based ordinal.
    Ordinal { ordinal: usize },
}

impl fmt::Display for ColumnResolutionTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name { name } => write!(f, "column '{name}'"),
            Self::Ordinal { ordinal } => write!(f, "output ordinal {ordinal}"),
        }
    }
}

/// Why an output-column selector could not be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnResolutionReason {
    /// The output shape is known and does not contain the requested column.
    NotFound,
    /// An unresolved wildcard prevents the output position from being known.
    Indeterminate,
    /// More than one output position matches the requested name.
    Ambiguous,
}

impl fmt::Display for ColumnResolutionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("not found"),
            Self::Indeterminate => {
                f.write_str("indeterminate because an output wildcard could not be expanded")
            }
            Self::Ambiguous => f.write_str("ambiguous"),
        }
    }
}

/// Severity level for validation errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    /// An error that prevents the query from being valid
    Error,
    /// A warning about potential issues
    Warning,
}

/// Result of validating SQL
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the SQL is valid (no errors, warnings are allowed)
    pub valid: bool,
    /// List of validation errors and warnings
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a validation result with errors
    pub fn with_errors(errors: Vec<ValidationError>) -> Self {
        let has_errors = errors
            .iter()
            .any(|e| e.severity == ValidationSeverity::Error);
        Self {
            valid: !has_errors,
            errors,
        }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, error: ValidationError) {
        if error.severity == ValidationSeverity::Error {
            self.valid = false;
        }
        self.errors.push(error);
    }
}

/// The SQL a token's text comes from, with the shared copy parser tokens point into.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TokenSource<'a> {
    pub(crate) text: &'a str,
    pub(crate) shared: Option<&'a Arc<str>>,
}

/// Represents a position in the source SQL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    /// Starting Unicode character offset (not a UTF-8 byte or UTF-16 code-unit offset).
    pub start: usize,
    /// Ending Unicode character offset (exclusive).
    pub end: usize,
    /// Cursor line after consuming the token (1-based).
    pub line: usize,
    /// Cursor column after consuming the token (1-based, in Unicode characters).
    pub column: usize,
}

impl Span {
    /// Cover two source ranges, retaining the ending range's cursor location.
    pub fn through(self, end: Self) -> Self {
        Self {
            start: self.start,
            ..end
        }
    }

    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

/// SQLQuack: a node's source range, recorded only by a located parse and ignored by equality.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceSpan(pub Option<Span>);

impl SourceSpan {
    /// Whether no range was recorded, so the field is left out of serialised trees.
    pub fn is_absent(&self) -> bool {
        self.0.is_none()
    }
}

impl PartialEq for SourceSpan {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl SpanField for Option<Span> {
    fn recorded(&self) -> Option<Span> {
        *self
    }

    fn fill(&mut self, _span: Span) {}
}

impl SpanField for SourceSpan {
    fn recorded(&self) -> Option<Span> {
        self.0
    }

    fn fill(&mut self, span: Span) {
        self.0.get_or_insert(span);
    }
}

/// A token in the SQL token stream
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    /// The type of token
    pub token_type: TokenType,
    /// The raw text of the token
    pub text: String,
    /// Position information
    pub span: Span,
    /// Leading comments (comments that appeared before this token)
    #[serde(default)]
    pub comments: Vec<String>,
    /// Trailing comments (comments that appeared after this token, before the next one)
    #[serde(default)]
    pub trailing_comments: Vec<String>,
}

impl Token {
    /// Create a new token
    pub fn new(token_type: TokenType, text: impl Into<String>, span: Span) -> Self {
        Self {
            token_type,
            text: text.into(),
            span,
            comments: Vec::new(),
            trailing_comments: Vec::new(),
        }
    }

    /// Create a NUMBER token
    pub fn number(n: i64) -> Self {
        Self::new(TokenType::Number, n.to_string(), Span::default())
    }

    /// Create a STRING token
    pub fn string(s: impl Into<String>) -> Self {
        Self::new(TokenType::String, s, Span::default())
    }

    /// Create an IDENTIFIER token
    pub fn identifier(s: impl Into<String>) -> Self {
        Self::new(TokenType::Identifier, s, Span::default())
    }

    /// Create a VAR token
    pub fn var(s: impl Into<String>) -> Self {
        Self::new(TokenType::Var, s, Span::default())
    }

    /// Add a comment to this token
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comments.push(comment.into());
        self
    }
}

#[derive(Debug, Clone)]
pub enum ParserTokenText {
    Source {
        source: Arc<str>,
        start: usize,
        end: usize,
    },
    Owned(String),
}

impl Deref for ParserTokenText {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Source { source, start, end } => &source[*start..*end],
            Self::Owned(text) => text,
        }
    }
}

impl fmt::Display for ParserTokenText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self)
    }
}

impl PartialEq<str> for ParserTokenText {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.deref() == other
    }
}

impl PartialEq<&str> for ParserTokenText {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.deref() == *other
    }
}

#[derive(Debug, Clone)]
pub struct ParserToken {
    pub token_type: TokenType,
    pub span: Span,
    pub comments: Vec<String>,
    pub trailing_comments: Vec<String>,
    pub text: ParserTokenText,
}

impl ParserToken {
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[inline]
    pub fn text_owned(&self) -> String {
        self.text.to_string()
    }
}

impl From<Token> for ParserToken {
    fn from(token: Token) -> Self {
        Self {
            token_type: token.token_type,
            span: token.span,
            comments: token.comments,
            trailing_comments: token.trailing_comments,
            text: ParserTokenText::Owned(token.text),
        }
    }
}

impl TokenOutput for Token {
    fn from_source(
        token_type: TokenType,
        source: TokenSource<'_>,
        text: Range<usize>,
        span: Span,
    ) -> Self {
        Self::new(token_type, &source.text[text], span)
    }

    fn from_owned(token_type: TokenType, text: String, span: Span) -> Self {
        Self::new(token_type, text, span)
    }

    fn token_type(&self) -> TokenType {
        self.token_type
    }

    fn text<'a>(&'a self, _source: &'a str) -> &'a str {
        &self.text
    }

    fn comments_mut(&mut self) -> &mut Vec<String> {
        &mut self.comments
    }

    fn trailing_comments_mut(&mut self) -> &mut Vec<String> {
        &mut self.trailing_comments
    }
}

impl TokenOutput for ParserToken {
    fn from_source(
        token_type: TokenType,
        source: TokenSource<'_>,
        text: Range<usize>,
        span: Span,
    ) -> Self {
        let text = match source.shared {
            Some(shared) => ParserTokenText::Source {
                source: Arc::clone(shared),
                start: text.start,
                end: text.end,
            },
            None => ParserTokenText::Owned(source.text[text].to_string()),
        };
        Self {
            token_type,
            span,
            comments: Vec::new(),
            trailing_comments: Vec::new(),
            text,
        }
    }

    fn from_owned(token_type: TokenType, text: String, span: Span) -> Self {
        Self {
            token_type,
            span,
            comments: Vec::new(),
            trailing_comments: Vec::new(),
            text: ParserTokenText::Owned(text),
        }
    }

    fn token_type(&self) -> TokenType {
        self.token_type
    }

    fn text<'a>(&'a self, _source: &'a str) -> &'a str {
        self.text()
    }

    fn comments_mut(&mut self) -> &mut Vec<String> {
        &mut self.comments
    }

    fn trailing_comments_mut(&mut self) -> &mut Vec<String> {
        &mut self.trailing_comments
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}({})", self.token_type, self.text)
    }
}

/// All possible token types in SQL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u16)]
pub enum TokenType {
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Dot,
    Dash,
    Plus,
    Colon,
    DotColon,
    DColon,
    DColonDollar,
    DColonPercent,
    DColonQMark,
    DQMark,
    Semicolon,
    Star,
    Backslash,
    Slash,
    Lt,
    Lte,
    Gt,
    Gte,
    Not,
    Eq,
    Neq,
    NullsafeEq,
    ColonEq,
    ColonGt,
    NColonGt,
    And,
    Or,
    Amp,
    DPipe,
    PipeGt,
    Pipe,
    PipeSlash,
    DPipeSlash,
    Caret,
    CaretAt,
    LtLt,
    GtGt,
    Tilde,
    Arrow,
    DArrow,
    FArrow,
    Hash,
    HashArrow,
    DHashArrow,
    LrArrow,
    DAt,
    AtAt,
    AtQMark,
    LtAt,
    AtGt,
    Dollar,
    Parameter,
    Session,
    SessionParameter,
    SessionUser,
    DAmp,
    AmpLt,
    AmpGt,
    Adjacent,
    Xor,
    DStar,
    QMarkAmp,
    QMarkPipe,
    HashDash,
    Exclamation,

    UriStart,
    BlockStart,
    BlockEnd,
    Space,
    Break,

    BlockComment,
    LineComment,

    String,
    DollarString,
    TripleDoubleQuotedString,
    TripleSingleQuotedString,
    Number,
    Identifier,
    QuotedIdentifier,
    Database,
    Column,
    ColumnDef,
    Schema,
    Table,
    Warehouse,
    Stage,
    Streamlit,
    Var,
    BitString,
    HexString,
    /// Hex number: 0xA, 0xFF (BigQuery, SQLite style) - represents an integer in hex notation
    HexNumber,
    ByteString,
    NationalString,
    EscapeString,
    RawString,
    HeredocString,
    HeredocStringAlternative,
    UnicodeString,

    Bit,
    Boolean,
    TinyInt,
    UTinyInt,
    SmallInt,
    USmallInt,
    MediumInt,
    UMediumInt,
    Int,
    UInt,
    BigInt,
    UBigInt,
    BigNum,
    Int128,
    UInt128,
    Int256,
    UInt256,
    Float,
    Double,
    UDouble,
    Decimal,
    Decimal32,
    Decimal64,
    Decimal128,
    Decimal256,
    DecFloat,
    UDecimal,
    BigDecimal,
    Char,
    NChar,
    VarChar,
    NVarChar,
    BpChar,
    Text,
    MediumText,
    LongText,
    Blob,
    MediumBlob,
    LongBlob,
    TinyBlob,
    TinyText,
    Name,
    Binary,
    VarBinary,
    Json,
    JsonB,
    Time,
    TimeTz,
    TimeNs,
    Timestamp,
    TimestampTz,
    TimestampLtz,
    TimestampNtz,
    TimestampS,
    TimestampMs,
    TimestampNs,
    DateTime,
    DateTime2,
    DateTime64,
    SmallDateTime,
    Date,
    Date32,
    Int4Range,
    Int4MultiRange,
    Int8Range,
    Int8MultiRange,
    NumRange,
    NumMultiRange,
    TsRange,
    TsMultiRange,
    TsTzRange,
    TsTzMultiRange,
    DateRange,
    DateMultiRange,
    Uuid,
    Geography,
    GeographyPoint,
    Nullable,
    Geometry,
    Point,
    Ring,
    LineString,
    LocalTime,
    LocalTimestamp,
    SysTimestamp,
    MultiLineString,
    Polygon,
    MultiPolygon,
    HllSketch,
    HStore,
    Super,
    Serial,
    SmallSerial,
    BigSerial,
    Xml,
    Year,
    UserDefined,
    Money,
    SmallMoney,
    RowVersion,
    Image,
    Variant,
    Object,
    Inet,
    IpAddress,
    IpPrefix,
    Ipv4,
    Ipv6,
    Enum,
    Enum8,
    Enum16,
    FixedString,
    LowCardinality,
    Nested,
    AggregateFunction,
    SimpleAggregateFunction,
    TDigest,
    Unknown,
    Vector,
    Dynamic,
    Void,

    Add,
    Alias,
    Alter,
    All,
    Anti,
    Any,
    Apply,
    Array,
    Asc,
    AsOf,
    Attach,
    AutoIncrement,
    Begin,
    Between,
    BulkCollectInto,
    Cache,
    Cascade,
    Case,
    CharacterSet,
    Cluster,
    ClusterBy,
    Collate,
    Command,
    Comment,
    Commit,
    Prepare,
    Preserve,
    Connect,
    ConnectBy,
    Constraint,
    Copy,
    Create,
    Cross,
    Cube,
    CurrentDate,
    CurrentDateTime,
    CurrentSchema,
    CurrentTime,
    CurrentTimestamp,
    CurrentUser,
    CurrentRole,
    CurrentCatalog,
    Declare,
    Default,
    Delete,
    Desc,
    Describe,
    Detach,
    Dictionary,
    Distinct,
    Distribute,
    DistributeBy,
    Div,
    Drop,
    Else,
    End,
    Escape,
    Except,
    Execute,
    Exists,
    False,
    Fetch,
    File,
    FileFormat,
    Filter,
    Final,
    First,
    For,
    Force,
    ForeignKey,
    Format,
    From,
    Full,
    Function,
    Get,
    Glob,
    Global,
    Grant,
    GroupBy,
    GroupingSets,
    Having,
    Hint,
    Ignore,
    ILike,
    In,
    Index,
    IndexedBy,
    Inner,
    Input,
    Insert,
    Install,
    Intersect,
    Interval,
    Into,
    Inpath,
    InputFormat,
    Introducer,
    IRLike,
    Is,
    IsNull,
    Join,
    JoinMarker,
    Keep,
    Key,
    Kill,
    Lambda,
    Language,
    Lateral,
    Left,
    Like,
    NotLike,
    NotILike,
    NotRLike,
    NotIRLike,
    Limit,
    List,
    Load,
    Local,
    Lock,
    Map,
    Match,
    MatchCondition,
    MatchRecognize,
    MemberOf,
    Materialized,
    Merge,
    Mod,
    Model,
    Natural,
    Next,
    NoAction,
    Nothing,
    NotNull,
    Null,
    ObjectIdentifier,
    Offset,
    On,
    Only,
    Operator,
    OrderBy,
    OrderSiblingsBy,
    Ordered,
    Ordinality,
    Out,
    Outer,
    Output,
    Over,
    Overlaps,
    Overwrite,
    Partition,
    PartitionBy,
    Percent,
    Pivot,
    Placeholder,
    Positional,
    Pragma,
    Prewhere,
    PrimaryKey,
    Procedure,
    Properties,
    PseudoType,
    Put,
    Qualify,
    Quote,
    QDColon,
    Range,
    Recursive,
    Refresh,
    Rename,
    Replace,
    Returning,
    Revoke,
    References,
    Restrict,
    Right,
    RLike,
    Rollback,
    Rollup,
    Row,
    Rows,
    Select,
    Semi,
    Savepoint,
    Separator,
    Sequence,
    Serde,
    SerdeProperties,
    Set,
    Settings,
    Show,
    Siblings,
    SimilarTo,
    Some,
    Sort,
    SortBy,
    SoundsLike,
    StartWith,
    StorageIntegration,
    StraightJoin,
    Struct,
    Summarize,
    TableSample,
    Sample,
    Bernoulli,
    System,
    Block,
    Seed,
    Repeatable,
    Tag,
    Temporary,
    Transaction,
    To,
    Top,
    Then,
    True,
    Truncate,
    Uncache,
    Union,
    Unnest,
    Unpivot,
    Update,
    Use,
    Using,
    Values,
    View,
    SemanticView,
    Volatile,
    When,
    Where,
    Window,
    With,
    Ties,
    Exclude,
    No,
    Others,
    Unique,
    UtcDate,
    UtcTime,
    UtcTimestamp,
    VersionSnapshot,
    TimestampSnapshot,
    Option,
    Sink,
    Source,
    Analyze,
    Namespace,
    Export,
    As,
    By,
    Nulls,
    Respect,
    Last,
    If,
    Cast,
    TryCast,
    SafeCast,
    Count,
    Extract,
    Substring,
    Trim,
    Leading,
    Trailing,
    Both,
    Position,
    Overlaying,
    Placing,
    Treat,
    Within,
    Group,
    Order,

    Unbounded,
    Preceding,
    Following,
    Current,
    Groups,

    Trigger,
    Type,
    Domain,
    Returns,
    Body,
    Increment,
    Minvalue,
    Maxvalue,
    Start,
    Cycle,
    NoCycle,
    Prior,
    Generated,
    Identity,
    Always,
    Measures,
    Pattern,
    Define,
    Running,
    Owned,
    After,
    Before,
    Instead,
    Each,
    Statement,
    Referencing,
    Old,
    New,
    Of,
    Check,
    Authorization,
    Restart,

    /// Never produced by [`Tokenizer`]; a caller-supplied one marks the end of input.
    Eof,
}

impl TokenType {
    /// Check if this token type is a keyword that can be used as an identifier in certain contexts
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenType::Select
                | TokenType::From
                | TokenType::Where
                | TokenType::And
                | TokenType::Or
                | TokenType::Not
                | TokenType::In
                | TokenType::Is
                | TokenType::Null
                | TokenType::True
                | TokenType::False
                | TokenType::As
                | TokenType::On
                | TokenType::Join
                | TokenType::Left
                | TokenType::Right
                | TokenType::Inner
                | TokenType::Outer
                | TokenType::Full
                | TokenType::Cross
                | TokenType::Semi
                | TokenType::Anti
                | TokenType::Union
                | TokenType::Except
                | TokenType::Intersect
                | TokenType::GroupBy
                | TokenType::OrderBy
                | TokenType::Having
                | TokenType::Limit
                | TokenType::Offset
                | TokenType::Case
                | TokenType::When
                | TokenType::Then
                | TokenType::Else
                | TokenType::End
                | TokenType::Create
                | TokenType::Drop
                | TokenType::Alter
                | TokenType::Insert
                | TokenType::Update
                | TokenType::Delete
                | TokenType::Into
                | TokenType::Values
                | TokenType::Set
                | TokenType::With
                | TokenType::Distinct
                | TokenType::All
                | TokenType::Exists
                | TokenType::Between
                | TokenType::Like
                | TokenType::ILike
                | TokenType::Filter
                | TokenType::Date
                | TokenType::Timestamp
                | TokenType::TimestampTz
                | TokenType::Interval
                | TokenType::Time
                | TokenType::Table
                | TokenType::Index
                | TokenType::Column
                | TokenType::Database
                | TokenType::Schema
                | TokenType::View
                | TokenType::Function
                | TokenType::Procedure
                | TokenType::Trigger
                | TokenType::Sequence
                | TokenType::Over
                | TokenType::Partition
                | TokenType::Window
                | TokenType::Rows
                | TokenType::Range
                | TokenType::First
                | TokenType::Last
                | TokenType::Preceding
                | TokenType::Following
                | TokenType::Current
                | TokenType::Row
                | TokenType::Unbounded
                | TokenType::Array
                | TokenType::Struct
                | TokenType::Map
                | TokenType::PrimaryKey
                | TokenType::Key
                | TokenType::ForeignKey
                | TokenType::References
                | TokenType::Unique
                | TokenType::Check
                | TokenType::Default
                | TokenType::Constraint
                | TokenType::Comment
                | TokenType::Rollup
                | TokenType::Cube
                | TokenType::Grant
                | TokenType::Revoke
                | TokenType::Type
                | TokenType::Use
                | TokenType::Cache
                | TokenType::Uncache
                | TokenType::Load
                | TokenType::Any
                | TokenType::Some
                | TokenType::Asc
                | TokenType::Desc
                | TokenType::Nulls
                | TokenType::Lateral
                | TokenType::Natural
                | TokenType::Escape
                | TokenType::Glob
                | TokenType::Match
                | TokenType::Recursive
                | TokenType::Replace
                | TokenType::Returns
                | TokenType::If
                | TokenType::Pivot
                | TokenType::Unpivot
                | TokenType::Json
                | TokenType::Blob
                | TokenType::Text
                | TokenType::Int
                | TokenType::BigInt
                | TokenType::SmallInt
                | TokenType::TinyInt
                | TokenType::Int128
                | TokenType::UInt128
                | TokenType::Int256
                | TokenType::UInt256
                | TokenType::UInt
                | TokenType::UBigInt
                | TokenType::Float
                | TokenType::Double
                | TokenType::Decimal
                | TokenType::Boolean
                | TokenType::VarChar
                | TokenType::Char
                | TokenType::Binary
                | TokenType::VarBinary
                | TokenType::No
                | TokenType::DateTime
                | TokenType::Truncate
                | TokenType::Execute
                | TokenType::Merge
                | TokenType::Top
                | TokenType::Begin
                | TokenType::Generated
                | TokenType::Identity
                | TokenType::Always
                | TokenType::Extract
                | TokenType::AsOf
                | TokenType::Prior
                | TokenType::After
                | TokenType::Restrict
                | TokenType::Cascade
                | TokenType::Local
                | TokenType::Rename
                | TokenType::Enum
                | TokenType::Within
                | TokenType::Format
                | TokenType::Final
                | TokenType::FileFormat
                | TokenType::Input
                | TokenType::InputFormat
                | TokenType::Copy
                | TokenType::Put
                | TokenType::Get
                | TokenType::Show
                | TokenType::Serde
                | TokenType::Sample
                | TokenType::Sort
                | TokenType::Collate
                | TokenType::Ties
                | TokenType::IsNull
                | TokenType::NotNull
                | TokenType::Exclude
                | TokenType::Temporary
                | TokenType::Add
                | TokenType::Ordinality
                | TokenType::Overlaps
                | TokenType::Block
                | TokenType::Pattern
                | TokenType::Group
                | TokenType::Cluster
                | TokenType::Repeatable
                | TokenType::Groups
                | TokenType::Commit
                | TokenType::Warehouse
                | TokenType::System
                | TokenType::By
                | TokenType::To
                | TokenType::Fetch
                | TokenType::For
                | TokenType::Only
                | TokenType::Next
                | TokenType::Lock
                | TokenType::Refresh
                | TokenType::Settings
                | TokenType::Operator
                | TokenType::Overwrite
                | TokenType::StraightJoin
                | TokenType::Start
                | TokenType::Ignore
                | TokenType::Domain
                | TokenType::Apply
                | TokenType::Respect
                | TokenType::Materialized
                | TokenType::Prewhere
                | TokenType::Old
                | TokenType::New
                | TokenType::Cast
                | TokenType::TryCast
                | TokenType::SafeCast
                | TokenType::Transaction
                | TokenType::Describe
                | TokenType::Kill
                | TokenType::Lambda
                | TokenType::Declare
                | TokenType::Keep
                | TokenType::Output
                | TokenType::Percent
                | TokenType::Qualify
                | TokenType::Returning
                | TokenType::Language
                | TokenType::Prepare
                | TokenType::Preserve
                | TokenType::Savepoint
                | TokenType::Rollback
                | TokenType::Body
                | TokenType::Increment
                | TokenType::Minvalue
                | TokenType::Maxvalue
                | TokenType::Cycle
                | TokenType::NoCycle
                | TokenType::Seed
                | TokenType::Namespace
                | TokenType::Authorization
                | TokenType::Order
                | TokenType::Restart
                | TokenType::Before
                | TokenType::Instead
                | TokenType::Each
                | TokenType::Statement
                | TokenType::Referencing
                | TokenType::Of
                | TokenType::Separator
                | TokenType::Others
                | TokenType::Placing
                | TokenType::Owned
                | TokenType::Running
                | TokenType::Define
                | TokenType::Measures
                | TokenType::MatchRecognize
                | TokenType::AutoIncrement
                | TokenType::Connect
                | TokenType::Distribute
                | TokenType::Bernoulli
                | TokenType::TableSample
                | TokenType::Inpath
                | TokenType::Pragma
                | TokenType::Siblings
                | TokenType::SerdeProperties
                | TokenType::RLike
        )
    }

    /// Check if this token type is a comparison operator
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            TokenType::Eq
                | TokenType::Neq
                | TokenType::Lt
                | TokenType::Lte
                | TokenType::Gt
                | TokenType::Gte
                | TokenType::NullsafeEq
        )
    }

    /// Check if this token type is an arithmetic operator
    pub fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            TokenType::Plus
                | TokenType::Dash
                | TokenType::Star
                | TokenType::Slash
                | TokenType::Percent
                | TokenType::Mod
                | TokenType::Div
        )
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Tokenizer configuration for a dialect
#[derive(Debug, Clone)]
pub struct TokenizerConfig {
    /// Keywords mapping (uppercase keyword -> token type)
    pub keywords: HashMap<String, TokenType>,
    /// Single character tokens
    pub single_tokens: HashMap<char, TokenType>,
    /// Quote characters (start -> end)
    pub quotes: HashMap<String, String>,
    /// Identifier quote characters (start -> end)
    pub identifiers: HashMap<char, char>,
    /// Whether quoted identifiers use string-style backslash escapes (BigQuery/ClickHouse).
    pub identifier_backslash_escapes: bool,
    /// Comment definitions (start -> optional end)
    pub comments: HashMap<String, Option<String>>,
    /// String escape characters
    pub string_escapes: Vec<char>,
    /// Whether to support nested comments
    pub nested_comments: bool,
    /// Characters a backslash escapes (MySQL); otherwise the backslash is dropped.
    pub escape_follow_chars: Vec<char>,
    /// Whether b'...' is a byte string (BigQuery) rather than a bit string (the default).
    pub b_prefix_is_byte_string: bool,
    /// Numeric literal suffixes and their types, such as `L` for BIGINT (Hive, Spark).
    pub numeric_literals: HashMap<String, String>,
    /// Whether unquoted identifiers may start with a digit, as `1a` (Hive, Spark, MySQL).
    pub identifiers_can_start_with_digit: bool,
    /// Whether a 0x prefix starts a hex literal (BigQuery, SQLite, Teradata).
    pub hex_number_strings: bool,
    /// Whether 0x literals are integers (BigQuery) rather than binary strings.
    pub hex_string_is_integer_type: bool,
    /// Whether an empty `0x` is an empty binary string (TSQL).
    pub allow_empty_hex_string: bool,
    /// Whether escapes such as \' work in raw strings (BigQuery) or not (Spark).
    pub string_escapes_allowed_in_raw_strings: bool,
    /// Whether # starts a single-line comment (ClickHouse, MySQL)
    pub hash_comments: bool,
    /// Whether `$` can start or continue an identifier (ClickHouse).
    pub dollar_sign_is_identifier: bool,
    /// Whether data after INSERT ... FORMAT name is raw up to a blank line (ClickHouse).
    pub insert_format_raw_data: bool,
    /// Whether numbers may use `_` digit separators, as `1_000` (ClickHouse, DuckDB).
    pub numbers_can_be_underscore_separated: bool,
    /// Recovers `'a\' or 1=1` by closing at the escaped quote when no later quote exists.
    pub recover_terminal_backslash_quote: bool,
    /// Recovers an unterminated final single-quoted string at end of input (ClickHouse).
    pub recover_unterminated_string: bool,
    /// Whether `//` is the integer-division operator (Vertica).
    pub double_slash_int_div: bool,
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            keywords: DEFAULT_KEYWORDS.clone(),
            single_tokens: DEFAULT_SINGLE_TOKENS.clone(),
            quotes: DEFAULT_QUOTES.clone(),
            identifiers: DEFAULT_IDENTIFIERS.clone(),
            identifier_backslash_escapes: false,
            comments: DEFAULT_COMMENTS.clone(),
            string_escapes: vec!['\''],
            nested_comments: true,
            escape_follow_chars: vec![],
            b_prefix_is_byte_string: false,
            numeric_literals: HashMap::new(),
            identifiers_can_start_with_digit: false,
            hex_number_strings: false,
            hex_string_is_integer_type: false,
            allow_empty_hex_string: false,
            string_escapes_allowed_in_raw_strings: true,
            hash_comments: false,
            dollar_sign_is_identifier: false,
            insert_format_raw_data: false,
            numbers_can_be_underscore_separated: false,
            recover_terminal_backslash_quote: false,
            recover_unterminated_string: false,
            double_slash_int_div: false,
        }
    }
}

/// SQL Tokenizer
#[derive(Debug)]
pub struct Tokenizer {
    pub(crate) config: Arc<TokenizerConfig>,
}

impl Tokenizer {
    /// Whether this tokenizer's block comments nest, as lossless trivia must read them.
    pub fn nested_comments(&self) -> bool {
        self.config.nested_comments
    }

    /// Create a new tokenizer with the given configuration
    pub fn new(config: TokenizerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub fn from_shared_config(config: Arc<TokenizerConfig>) -> Self {
        Self { config }
    }

    /// Create a tokenizer with default configuration
    pub fn default_config() -> Self {
        Self::new(TokenizerConfig::default())
    }

    /// Tokenize a SQL string
    pub fn tokenize(&self, sql: &str) -> Result<Vec<Token>> {
        if sql.is_ascii() {
            TokenizerState::<_, Token>::new(sql, &self.config, AsciiCursor(sql.as_bytes()))
                .tokenize()
        } else {
            TokenizerState::<_, Token>::new(sql, &self.config, UnicodeCursor::new(sql)).tokenize()
        }
    }

    pub fn tokenize_for_parser(
        &self,
        sql: &Arc<str>,
    ) -> Result<(Vec<ParserToken>, TokenGuardStats)> {
        if sql.is_ascii() {
            let mut state = TokenizerState::<_, ParserToken>::new_shared(
                sql,
                Arc::clone(sql),
                &self.config,
                AsciiCursor(sql.as_bytes()),
            );
            let tokens = state.tokenize()?;
            Ok((tokens, state.guard_stats.take().unwrap_or_default()))
        } else {
            let mut state = TokenizerState::<_, ParserToken>::new_shared(
                sql,
                Arc::clone(sql),
                &self.config,
                UnicodeCursor::new(sql),
            );
            let tokens = state.tokenize()?;
            Ok((tokens, state.guard_stats.take().unwrap_or_default()))
        }
    }

    #[cfg(test)]
    pub(crate) fn tokenize_without_ascii_fast_path(&self, sql: &str) -> Result<Vec<Token>> {
        TokenizerState::new(sql, &self.config, UnicodeCursor::new(sql)).tokenize()
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::default_config()
    }
}

#[derive(Debug)]
pub(crate) struct AsciiCursor<'a>(pub(crate) &'a [u8]);

impl TokenizerCursor for AsciiCursor<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn char_at(&self, index: usize) -> char {
        self.0[index] as char
    }

    #[inline]
    fn text_from_range(&self, source: &str, start: usize, end: usize) -> String {
        source[start..end].to_string()
    }

    #[inline]
    fn source_range<'a>(&self, source: &'a str, start: usize, end: usize) -> Option<&'a str> {
        Some(&source[start..end])
    }
}

#[derive(Debug)]
pub(crate) struct UnicodeCursor(pub(crate) Vec<char>);

impl UnicodeCursor {
    pub(crate) fn new(source: &str) -> Self {
        Self(source.chars().collect())
    }
}

impl TokenizerCursor for UnicodeCursor {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn char_at(&self, index: usize) -> char {
        self.0[index]
    }

    #[inline]
    fn text_from_range(&self, _source: &str, start: usize, end: usize) -> String {
        self.0[start..end].iter().collect()
    }
}

/// Internal state for tokenization
#[derive(Debug)]
pub(crate) struct TokenizerState<'a, C, T> {
    pub(crate) source: &'a str,
    pub(crate) shared_source: Option<Arc<str>>,
    pub(crate) cursor: C,
    pub(crate) size: usize,
    pub(crate) tokens: Vec<T>,
    pub(crate) start: usize,
    pub(crate) current: usize,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) comments: Vec<String>,
    pub(crate) guard_stats: Option<TokenGuardStats>,
    pub(crate) config: &'a TokenizerConfig,
}

/// A position in source SQL, in Unicode characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct SourceLocation {
    /// Character offset of the first character.
    pub start: usize,
    /// Character offset one past the last character.
    pub end: usize,
    /// One-based line.
    pub line: usize,
    /// One-based column.
    pub column: usize,
}

/// Where each line of source SQL starts, to turn character ranges into locations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLines {
    line_starts: Vec<usize>,
    length: usize,
    shift: usize,
    source: String,
}

impl SourceLines {
    /// The lines of `source`.
    pub fn new(source: &str) -> Self {
        Self::shifted(source, 0)
    }

    /// The lines of `source`, for ranges in a parse of it behind a prefix of `shift` characters.
    pub fn shifted(source: &str, shift: usize) -> Self {
        let mut line_starts = vec![0];
        let mut length = 0;
        for (offset, character) in source.chars().enumerate() {
            if character == '\n' {
                line_starts.push(offset + 1);
            }
            length = offset + 1;
        }
        Self {
            line_starts,
            length,
            shift,
            source: source.to_owned(),
        }
    }

    /// The source text at `location`, a location these lines gave.
    pub fn text(&self, location: &SourceLocation) -> String {
        self.source
            .chars()
            .skip(location.start)
            .take(location.end.saturating_sub(location.start))
            .collect()
    }

    /// The location of the characters `start..end` of the parsed text.
    pub fn location(&self, start: usize, end: usize) -> SourceLocation {
        let start = start.saturating_sub(self.shift).min(self.length);
        let end = end.saturating_sub(self.shift).clamp(start, self.length);
        let line_index = self
            .line_starts
            .partition_point(|line_start| *line_start <= start)
            .saturating_sub(1);
        SourceLocation {
            start,
            end,
            line: line_index + 1,
            column: start - self.line_starts[line_index] + 1,
        }
    }

    /// The location of the characters `start..end` of the source itself, not of the parsed text.
    pub fn source_location(&self, start: usize, end: usize) -> SourceLocation {
        self.location(start + self.shift, end + self.shift)
    }

    /// The location of the whole source.
    pub fn whole(&self) -> SourceLocation {
        self.location(self.shift, self.shift + self.length)
    }
}
