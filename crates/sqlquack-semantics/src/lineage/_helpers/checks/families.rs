//! Type families, polyglot's comparison classes, from SQLQuack's types of expressions.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::types::models::DataType;

use crate::lineage::_helpers::checks::relational::query_families;
use crate::lineage::_helpers::tracing::expressions::expression_lineage;
use crate::lineage::models::{LineageContext, LineageScope};

/// The scale of a fixed-point type that holds only integers.
const INTEGER_SCALE: &str = "0";

/// A type's family, the class the type checks compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Family {
    Unknown,
    Boolean,
    Integer,
    Numeric,
    String,
    Binary,
    Date,
    Time,
    Timestamp,
    Interval,
    Json,
    Uuid,
    Array,
    Map,
    Struct,
}

impl Family {
    pub(crate) fn is_numeric(self) -> bool {
        matches!(self, Self::Integer | Self::Numeric)
    }

    pub(crate) fn is_temporal(self) -> bool {
        matches!(
            self,
            Self::Date | Self::Time | Self::Timestamp | Self::Interval
        )
    }

    /// Whether the checks model the family's conversions fully.
    pub(crate) fn fully_modelled(self) -> bool {
        !matches!(
            self,
            Self::Unknown | Self::Json | Self::Uuid | Self::Array | Self::Map | Self::Struct
        )
    }

    /// The family's name in a finding's words.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Numeric => "numeric",
            Self::String => "string",
            Self::Binary => "binary",
            Self::Date => "date",
            Self::Time => "time",
            Self::Timestamp => "timestamp",
            Self::Interval => "interval",
            Self::Json => "json",
            Self::Uuid => "uuid",
            Self::Array => "array",
            Self::Map => "map",
            Self::Struct => "struct",
        }
    }
}

/// The family of a type as spelled, `NUMBER(38,0)` an integer and `VARCHAR(10)` a string.
pub(crate) fn type_family(spelling: &str) -> Family {
    let trimmed: &str = spelling
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\'' | '`'));
    let normalized: String = trimmed
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_lowercase();
    if normalized.is_empty() {
        return Family::Unknown;
    }
    if let Some((base, inner)) = type_arguments(&normalized) {
        match base {
            "nullable" | "lowcardinality" => return type_family(inner),
            "array" | "list" => return Family::Array,
            "map" => return Family::Map,
            "struct" | "row" | "record" => return Family::Struct,
            "number" | "numeric" | "decimal" | "dec"
                if inner
                    .split(',')
                    .nth(1)
                    .is_some_and(|scale| scale.trim() == INTEGER_SCALE) =>
            {
                return Family::Integer;
            }
            _ => {}
        }
    }
    if ["array<", "list<"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
        || normalized.ends_with("[]")
    {
        return Family::Array;
    }
    if normalized.starts_with("map<") || normalized.starts_with("map(") {
        return Family::Map;
    }
    if ["struct<", "struct(", "row<", "record<", "object<"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return Family::Struct;
    }
    let base: &str = normalized.split('(').next().unwrap_or_default().trim();
    let base: &str = base.strip_prefix("unsigned ").unwrap_or(base);
    base_family(base.strip_suffix(" unsigned").unwrap_or(base))
}

fn type_arguments(normalized: &str) -> Option<(&str, &str)> {
    let open: usize = normalized.find('(')?;
    if !normalized.ends_with(')') || open + 1 >= normalized.len() {
        return None;
    }
    Some((
        normalized[..open].trim(),
        normalized[open + 1..normalized.len() - 1].trim(),
    ))
}

fn base_family(base: &str) -> Family {
    match base {
        "bool" | "boolean" => Family::Boolean,
        "tinyint" | "smallint" | "int2" | "int" | "integer" | "int4" | "int8" | "bigint"
        | "serial" | "smallserial" | "bigserial" | "utinyint" | "usmallint" | "uinteger"
        | "ubigint" | "uint8" | "uint16" | "uint32" | "uint64" | "int16" | "int32" | "int64"
        | "hugeint" | "int128" | "largeint" | "uhugeint" | "uint128" | "varint" => Family::Integer,
        "numeric" | "bignumeric" | "decimal" | "dec" | "number" | "float" | "float4" | "float8"
        | "real" | "double" | "double precision" | "bfloat16" | "float16" | "float32"
        | "float64" => Family::Numeric,
        "char" | "character" | "varchar" | "character varying" | "nchar" | "nvarchar" | "text"
        | "string" | "clob" => Family::String,
        "binary" | "varbinary" | "blob" | "bytea" | "bytes" => Family::Binary,
        "date" => Family::Date,
        "time" => Family::Time,
        "timestamp"
        | "timestamp_ntz"
        | "timestamp_ltz"
        | "timestamp_tz"
        | "timestampntz"
        | "timestampltz"
        | "timestamptz"
        | "datetime"
        | "datetime2"
        | "smalldatetime"
        | "timestamp with time zone"
        | "timestamp without time zone" => Family::Timestamp,
        "interval" => Family::Interval,
        "json" | "jsonb" | "variant" => Family::Json,
        "uuid" | "uniqueidentifier" => Family::Uuid,
        "array" | "list" => Family::Array,
        "map" => Family::Map,
        "struct" | "row" | "record" | "object" => Family::Struct,
        _ => Family::Unknown,
    }
}

/// The family of a written type, a parsed DECIMAL numeric whatever its scale.
pub(crate) fn data_type_family(data_type: &DataType) -> Family {
    match data_type {
        DataType::Boolean => Family::Boolean,
        DataType::TinyInt { .. }
        | DataType::SmallInt { .. }
        | DataType::Int { .. }
        | DataType::BigInt { .. }
        | DataType::Int128
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64
        | DataType::UInt128 => Family::Integer,
        DataType::Float { .. } | DataType::Double { .. } | DataType::Decimal { .. } => {
            Family::Numeric
        }
        DataType::Char { .. }
        | DataType::VarChar { .. }
        | DataType::String { .. }
        | DataType::Text
        | DataType::TextWithLength { .. }
        | DataType::CharacterSet { .. }
        | DataType::Enum { .. }
        | DataType::Set { .. } => Family::String,
        DataType::Binary { .. }
        | DataType::VarBinary { .. }
        | DataType::Blob
        | DataType::Bit { .. }
        | DataType::VarBit { .. } => Family::Binary,
        DataType::Date => Family::Date,
        DataType::Time { .. } => Family::Time,
        DataType::Timestamp { .. } => Family::Timestamp,
        DataType::Interval { .. } => Family::Interval,
        DataType::Json | DataType::JsonB => Family::Json,
        DataType::Uuid => Family::Uuid,
        DataType::Array { .. } | DataType::List { .. } | DataType::Vector { .. } => Family::Array,
        DataType::Map { .. } => Family::Map,
        DataType::Struct { .. }
        | DataType::Object { .. }
        | DataType::Union { .. }
        | DataType::Geometry { .. }
        | DataType::Geography { .. } => Family::Struct,
        DataType::Nullable { inner } => data_type_family(inner),
        DataType::Custom { name } => type_family(name),
        _ => Family::Unknown,
    }
}

/// The family of `expression` read in `scope`: its SQLQuack type, else what its form proves.
pub(crate) fn expression_family(
    expression: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Family {
    match expression {
        Expression::Null(_) => return Family::Unknown,
        Expression::Function(_) | Expression::AggregateFunction(_) => {
            return expression_lineage(expression, scope, context)
                .data_type
                .as_deref()
                .map_or(Family::Unknown, type_family);
        }
        Expression::Cast(cast) | Expression::TryCast(cast) | Expression::SafeCast(cast) => {
            return data_type_family(&cast.to);
        }
        _ => {}
    }
    match expression_lineage(expression, scope, context)
        .data_type
        .as_deref()
        .map(type_family)
    {
        Some(family) if family != Family::Unknown => family,
        _ => written_family(expression, scope, context),
    }
}

/// What an expression's form proves of its family when its type is unknown.
fn written_family(
    expression: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Family {
    let family = |expression: &Expression| expression_family(expression, scope, context);
    match expression {
        Expression::Literal(literal) => literal_family(literal),
        Expression::Boolean(_) => Family::Boolean,
        Expression::Alias(alias) => family(&alias.this),
        Expression::Neg(unary) => family(&unary.this),
        Expression::Add(operation) | Expression::Sub(operation) | Expression::Mul(operation) => {
            arithmetic_family(family(&operation.left), family(&operation.right))
        }
        Expression::Div(_) | Expression::Mod(_) => Family::Numeric,
        Expression::Concat(_) => Family::String,
        Expression::Eq(_)
        | Expression::Neq(_)
        | Expression::Lt(_)
        | Expression::Lte(_)
        | Expression::Gt(_)
        | Expression::Gte(_)
        | Expression::Like(_)
        | Expression::ILike(_)
        | Expression::And(_)
        | Expression::Or(_)
        | Expression::Not(_)
        | Expression::Between(_)
        | Expression::In(_)
        | Expression::IsNull(_)
        | Expression::IsTrue(_)
        | Expression::IsFalse(_)
        | Expression::Is(_) => Family::Boolean,
        Expression::Length(_) | Expression::DateDiff(_) | Expression::Extract(_) => Family::Integer,
        Expression::Upper(_)
        | Expression::Lower(_)
        | Expression::Trim(_)
        | Expression::LTrim(_)
        | Expression::RTrim(_)
        | Expression::Replace(_)
        | Expression::Substring(_)
        | Expression::Left(_)
        | Expression::Right(_)
        | Expression::Repeat(_)
        | Expression::Lpad(_)
        | Expression::Rpad(_)
        | Expression::ConcatWs(_) => Family::String,
        Expression::Abs(_)
        | Expression::Round(_)
        | Expression::Floor(_)
        | Expression::Ceil(_)
        | Expression::Power(_)
        | Expression::Sqrt(_)
        | Expression::Cbrt(_)
        | Expression::Ln(_)
        | Expression::Log(_)
        | Expression::Exp(_) => Family::Numeric,
        Expression::DateAdd(_)
        | Expression::DateSub(_)
        | Expression::ToDate(_)
        | Expression::CurrentDate(_) => Family::Date,
        Expression::ToTimestamp(_)
        | Expression::CurrentTimestamp(_)
        | Expression::CurrentTimestampLTZ(_) => Family::Timestamp,
        Expression::CurrentTime(_) => Family::Time,
        Expression::Interval(_) => Family::Interval,
        Expression::Subquery(subquery) => match query_families(&subquery.this, scope, context) {
            Some(families) if families.len() == 1 => families[0],
            _ => Family::Unknown,
        },
        _ => Family::Unknown,
    }
}

fn literal_family(literal: &Literal) -> Family {
    match literal {
        Literal::Number(value) if value.contains(['.', 'e', 'E']) => Family::Numeric,
        Literal::Number(_) | Literal::HexNumber(_) => Family::Integer,
        Literal::Date(_) => Family::Date,
        Literal::Time(_) => Family::Time,
        Literal::Timestamp(_) | Literal::Datetime(_) => Family::Timestamp,
        Literal::HexString(_) | Literal::BitString(_) | Literal::ByteString(_) => Family::Binary,
        _ => Family::String,
    }
}

fn arithmetic_family(left: Family, right: Family) -> Family {
    if left == Family::Unknown || right == Family::Unknown {
        Family::Unknown
    } else if left == Family::Integer && right == Family::Integer {
        Family::Integer
    } else if left.is_numeric() && right.is_numeric() {
        Family::Numeric
    } else if left.is_temporal() || right.is_temporal() {
        left
    } else {
        Family::Unknown
    }
}
