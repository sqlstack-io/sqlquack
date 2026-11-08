//! Data models of expressions types.

use serde::Deserialize;
use serde::Serialize;

use crate::expressions::statements::models::VerticaDataType;
use crate::expressions::tree::models::Expression;

/// Struct field with optional OPTIONS clause (BigQuery) and COMMENT (Spark/Databricks)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructField {
    /// Raw field name or delimited identifier; empty for an anonymous field.
    pub name: String,
    pub data_type: DataType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Expression>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl StructField {
    /// Create a new struct field without options
    pub fn new(name: String, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            options: Vec::new(),
            comment: None,
        }
    }

    /// Create a new struct field with options
    pub fn with_options(name: String, data_type: DataType, options: Vec<Expression>) -> Self {
        Self {
            name,
            data_type,
            options,
            comment: None,
        }
    }

    /// Create a new struct field with options and comment
    pub fn with_options_and_comment(
        name: String,
        data_type: DataType,
        options: Vec<Expression>,
        comment: Option<String>,
    ) -> Self {
        Self {
            name,
            data_type,
            options,
            comment,
        }
    }
}

/// Oracle data types the generic [`DataType`] cannot hold losslessly, such as negative scales.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "oracle_data_type", rename_all = "snake_case")]
pub enum OracleDataType {
    Number {
        precision: Option<u32>,
        scale: Option<i32>,
    },
    BinaryFloat,
    BinaryDouble,
    Float {
        precision: Option<u32>,
    },
    Character {
        kind: OracleCharacterKind,
        length: Option<u32>,
        semantics: Option<OracleCharacterLengthSemantics>,
    },
    Date,
    Timestamp {
        precision: Option<u32>,
        timezone: OracleTimestampTimeZone,
    },
    IntervalYearToMonth {
        year_precision: Option<u32>,
    },
    IntervalDayToSecond {
        day_precision: Option<u32>,
        fractional_seconds_precision: Option<u32>,
    },
    Clob {
        national: bool,
    },
    Blob,
    Raw {
        length: Option<u32>,
    },
    Long {
        raw: bool,
    },
    RowId,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OracleCharacterKind {
    Char,
    VarChar,
    NChar,
    NVarChar,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OracleCharacterLengthSemantics {
    Byte,
    Char,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OracleTimestampTimeZone {
    None,
    WithTimeZone,
    WithLocalTimeZone,
}

/// Every data type the parser recognizes, standard and dialect-specific.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "data_type", rename_all = "snake_case")]
pub enum DataType {
    Vertica {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        vertica_type: Box<VerticaDataType>,
    },
    Boolean,
    TinyInt {
        length: Option<u32>,
    },
    SmallInt {
        length: Option<u32>,
    },
    /// INT with optional length; `integer_spelling` records an `INTEGER` spelling.
    Int {
        length: Option<u32>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        integer_spelling: bool,
    },
    BigInt {
        length: Option<u32>,
    },
    /// Signed 128-bit integer (DuckDB HUGEINT, ClickHouse Int128, StarRocks LARGEINT).
    Int128,
    /// Unsigned 8-bit integer (DuckDB UTINYINT, ClickHouse UInt8).
    #[serde(rename = "uint8")]
    UInt8,
    /// Unsigned 16-bit integer (DuckDB USMALLINT, ClickHouse UInt16).
    #[serde(rename = "uint16")]
    UInt16,
    /// Unsigned 32-bit integer (DuckDB UINTEGER, ClickHouse UInt32).
    #[serde(rename = "uint32")]
    UInt32,
    /// Unsigned 64-bit integer (DuckDB UBIGINT, ClickHouse UInt64).
    #[serde(rename = "uint64")]
    UInt64,
    /// Unsigned 128-bit integer (DuckDB UHUGEINT, ClickHouse UInt128).
    #[serde(rename = "uint128")]
    UInt128,
    /// FLOAT with optional precision and scale; `real_spelling` records a `REAL` spelling.
    Float {
        precision: Option<u32>,
        scale: Option<u32>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        real_spelling: bool,
    },
    Double {
        precision: Option<u32>,
        scale: Option<u32>,
    },
    Decimal {
        precision: Option<u32>,
        scale: Option<u32>,
    },
    /// Structured Oracle data type retained until the target dialect is known.
    Oracle {
        oracle_type: OracleDataType,
    },

    /// A HANA type whose domain or syntax is not represented by a common type.
    Hana {
        hana_type: HanaDataType,
    },

    Char {
        length: Option<u32>,
    },
    /// VARCHAR with optional length; `parenthesized_length` records Hive's `VARCHAR((50))`.
    VarChar {
        length: Option<u32>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        parenthesized_length: bool,
    },
    /// String type with optional max length (BigQuery STRING(n))
    String {
        length: Option<u32>,
    },
    Text,
    /// TEXT with optional length: TEXT(n) - used by MySQL, SQLite, DuckDB, etc.
    TextWithLength {
        length: u32,
    },

    Binary {
        length: Option<u32>,
    },
    VarBinary {
        length: Option<u32>,
    },
    Blob,

    Bit {
        length: Option<u32>,
    },
    VarBit {
        length: Option<u32>,
    },

    Date,
    Time {
        precision: Option<u32>,
        #[serde(default)]
        timezone: bool,
    },
    Timestamp {
        precision: Option<u32>,
        timezone: bool,
    },
    Interval {
        unit: Option<String>,
        /// For range intervals like INTERVAL DAY TO HOUR
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<String>,
    },

    Json,
    JsonB,

    Uuid,

    Array {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        element_type: Box<DataType>,
        /// Optional dimension size for PostgreSQL (e.g., [3] in INT[3])
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dimension: Option<u32>,
    },

    /// Materialize postfix LIST type, such as `INT LIST`.
    List {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        element_type: Box<DataType>,
    },

    Struct {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        fields: Vec<StructField>,
        nested: bool,
    },
    Map {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        key_type: Box<DataType>,
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        value_type: Box<DataType>,
    },

    Enum {
        values: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        assignments: Vec<Option<String>>,
    },

    Set {
        values: Vec<String>,
    },

    Union {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        fields: Vec<(String, DataType)>,
    },

    Vector {
        #[serde(default)]
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        element_type: Option<Box<DataType>>,
        dimension: Option<u32>,
    },

    Object {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        fields: Vec<(String, DataType, bool)>,
        modifier: Option<String>,
    },

    Nullable {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        inner: Box<DataType>,
    },

    /// An element or value type declared NOT NULL, as in Snowflake's `ARRAY ( INT NOT NULL )`.
    NotNull {
        #[serde(
            serialize_with = "crate::expressions::tree::main::_serialize_on_grown_stack::serialize_on_grown_stack"
        )]
        inner: Box<DataType>,
    },

    Custom {
        name: String,
    },

    Geometry {
        subtype: Option<String>,
        srid: Option<u32>,
    },
    Geography {
        subtype: Option<String>,
        srid: Option<u32>,
    },

    CharacterSet {
        name: String,
    },

    Unknown,
}

/// Source-specific HANA type, retained until a target is selected.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HanaDataType {
    pub name: String,
    pub parameters: Vec<u32>,
}

impl DataType {
    /// The longest chain of types nested inside this one: 0 for `INT`, 2 for `INT[][]`.
    pub(crate) fn nesting_depth(&self) -> usize {
        let mut deepest = 0;
        let mut pending: Vec<(&DataType, usize)> = vec![(self, 0)];
        while let Some((data_type, depth)) = pending.pop() {
            deepest = deepest.max(depth);
            let next = depth + 1;
            match data_type {
                DataType::Array { element_type, .. }
                | DataType::List { element_type }
                | DataType::Vector {
                    element_type: Some(element_type),
                    ..
                }
                | DataType::Nullable {
                    inner: element_type,
                }
                | DataType::NotNull {
                    inner: element_type,
                } => pending.push((element_type, next)),
                DataType::Map {
                    key_type,
                    value_type,
                } => pending.extend([(key_type.as_ref(), next), (value_type.as_ref(), next)]),
                DataType::Struct { fields, .. } => {
                    pending.extend(fields.iter().map(|field| (&field.data_type, next)));
                }
                DataType::Object { fields, .. } => {
                    pending.extend(fields.iter().map(|(_, field, _)| (field, next)));
                }
                DataType::Union { fields } => {
                    pending.extend(fields.iter().map(|(_, field)| (field, next)));
                }
                DataType::Vertica { vertica_type } => match vertica_type.as_ref() {
                    VerticaDataType::Array { element_type, .. }
                    | VerticaDataType::Set { element_type, .. } => {
                        pending.push((element_type, next));
                    }
                    VerticaDataType::Row { fields } => {
                        pending.extend(fields.iter().map(|field| (&field.data_type, next)));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        deepest
    }

    /// Whether this type is [`DataType::Unknown`] or has an unknown part, as `ARRAY<UNKNOWN>`.
    pub fn contains_unknown(&self) -> bool {
        match self {
            DataType::Unknown => true,
            DataType::Array { element_type, .. } | DataType::List { element_type } => {
                element_type.contains_unknown()
            }
            DataType::Nullable { inner } | DataType::NotNull { inner } => inner.contains_unknown(),
            DataType::Map {
                key_type,
                value_type,
            } => key_type.contains_unknown() || value_type.contains_unknown(),
            DataType::Struct { fields, .. } => fields
                .iter()
                .any(|field| field.data_type.contains_unknown()),
            DataType::Object { fields, .. } => fields.iter().any(|(_, t, _)| t.contains_unknown()),
            DataType::Union { fields } => fields.iter().any(|(_, t)| t.contains_unknown()),
            DataType::Vector { element_type, .. } => element_type
                .as_deref()
                .is_some_and(DataType::contains_unknown),
            _ => false,
        }
    }

    /// Whether `refined` fills this type's unknown parts without changing its shape.
    pub fn is_refined_by(&self, refined: &DataType) -> bool {
        self.contains_unknown() && self.matches_shape_of(refined)
    }

    pub(crate) fn matches_shape_of(&self, refined: &DataType) -> bool {
        if !self.contains_unknown() {
            return self == refined;
        }
        match (self, refined) {
            (DataType::Unknown, _) => true,
            (DataType::Nullable { inner } | DataType::NotNull { inner }, other) => {
                inner.matches_shape_of(other)
            }
            (this, DataType::Nullable { inner } | DataType::NotNull { inner }) => {
                this.matches_shape_of(inner)
            }
            (
                DataType::Array { element_type, .. } | DataType::List { element_type },
                DataType::Array {
                    element_type: other,
                    ..
                }
                | DataType::List {
                    element_type: other,
                },
            ) => element_type.matches_shape_of(other),
            (
                DataType::Map {
                    key_type,
                    value_type,
                },
                DataType::Map {
                    key_type: other_key,
                    value_type: other_value,
                },
            ) => key_type.matches_shape_of(other_key) && value_type.matches_shape_of(other_value),
            (DataType::Struct { fields, .. }, DataType::Struct { fields: other, .. }) => {
                fields.len() == other.len()
                    && fields.iter().zip(other).all(|(field, other)| {
                        field.name.eq_ignore_ascii_case(&other.name)
                            && field.data_type.matches_shape_of(&other.data_type)
                    })
            }
            _ => false,
        }
    }

    /// Canonical unsigned type names and their bit-width aliases.
    pub(crate) fn from_unsigned_name(name: &str) -> Option<Self> {
        match name.to_ascii_uppercase().as_str() {
            "UTINYINT" | "UINT8" => Some(Self::UInt8),
            "USMALLINT" | "UINT16" => Some(Self::UInt16),
            "UINTEGER" | "UINT32" => Some(Self::UInt32),
            "UBIGINT" | "UINT64" => Some(Self::UInt64),
            "UHUGEINT" | "UINT128" => Some(Self::UInt128),
            _ => None,
        }
    }
}

/// Array expression
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Array {
    pub expressions: Vec<Expression>,
    /// Inferred array data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// Struct expression
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Struct {
    pub fields: Vec<(Option<String>, Expression)>,
}

/// Tuple expression
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tuple {
    pub expressions: Vec<Expression>,
}

/// Interval expression
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    /// The value expression (e.g., '1', 5, column_ref)
    pub this: Option<Expression>,
    /// The unit specification (optional - can be None, a simple unit, a span, or an expression)
    pub unit: Option<IntervalUnitSpec>,
}

/// Specification for interval unit - can be a simple unit, a span (HOUR TO SECOND), or an expression
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IntervalUnitSpec {
    /// Simple interval unit (YEAR, MONTH, DAY, etc.)
    Simple {
        unit: IntervalUnit,
        /// Whether to use plural form (e.g., DAYS vs DAY)
        use_plural: bool,
    },
    /// Interval span (e.g., HOUR TO SECOND)
    Span(IntervalSpan),
    /// Oracle interval span over expressions, such as `DAY(9) TO SECOND(3)`.
    ExprSpan(IntervalSpanExpr),
    /// Expression as unit (e.g., CURRENT_DATE, CAST(GETDATE() AS DATE))
    Expr(Box<Expression>),
    /// Snowflake qualifier with a field precision, such as `YEAR(9) TO MONTH` or `DAY TO SECOND(3)`.
    Qualifier(IntervalQualifier),
}

/// Snowflake interval qualifier: fields with their leading and fractional seconds precisions.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalQualifier {
    /// Leading field (e.g., DAY)
    pub this: IntervalUnit,
    /// Leading field precision, as 9 in `DAY(9)`
    pub leading_precision: Option<u32>,
    /// Trailing field after TO (e.g., SECOND)
    pub expression: Option<IntervalUnit>,
    /// Fractional seconds precision, as 3 in `DAY TO SECOND(3)`
    pub fractional_precision: Option<u32>,
}

/// Interval span for ranges like HOUR TO SECOND
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalSpan {
    /// Start unit (e.g., HOUR)
    pub this: IntervalUnit,
    /// End unit (e.g., SECOND)
    pub expression: IntervalUnit,
}

/// Oracle interval span whose units are expressions with optional precision.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalSpanExpr {
    /// Start unit expression (e.g., Var("DAY") or Anonymous("DAY", [9]))
    pub this: Box<Expression>,
    /// End unit expression (e.g., Var("SECOND") or Anonymous("SECOND", [3]))
    pub expression: Box<Expression>,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum IntervalUnit {
    Year,
    Quarter,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
}
