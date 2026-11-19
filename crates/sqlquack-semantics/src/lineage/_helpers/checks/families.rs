//! Type families, polyglot's comparison classes, from SQLQuack's types of expressions.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::types::models::DataType;

use crate::lineage::_helpers::checks::relational::query_families;
use crate::lineage::_helpers::tracing::expressions::expression_lineage;
use crate::lineage::main::type_family::type_family;
use crate::lineage::models::Engine;
use crate::lineage::models::Family;
use crate::lineage::models::{LineageContext, LineageScope};

/// The scale of a fixed-point type that holds only integers.
const INTEGER_SCALE: &str = "0";

/// The longest bare type word `word_family` lowers on the stack.
const WORD_CAPACITY: usize = 32;

pub(crate) fn type_arguments(normalized: &str) -> Option<(&str, &str)> {
    let open: usize = normalized.find('(')?;
    if !normalized.ends_with(')') || open + 1 >= normalized.len() {
        return None;
    }
    Some((
        normalized[..open].trim(),
        normalized[open + 1..normalized.len() - 1].trim(),
    ))
}

/// The family of a bare type word such as `BIGINT`, lowered without allocating, else `None`.
pub(crate) fn word_family(spelling: &str) -> Option<Family> {
    let bytes: &[u8] = spelling.as_bytes();
    if bytes.is_empty()
        || bytes.len() > WORD_CAPACITY
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        return None;
    }
    let mut lowered: [u8; WORD_CAPACITY] = [0; WORD_CAPACITY];
    for (slot, byte) in lowered.iter_mut().zip(bytes) {
        *slot = byte.to_ascii_lowercase();
    }
    Some(base_family(&String::from_utf8_lossy(
        &lowered[..bytes.len()],
    )))
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
        DataType::Nullable { inner } | DataType::NotNull { inner } => data_type_family(inner),
        DataType::Custom { name } => type_family(name),
        DataType::Named { .. } => data_type
            .unnamed()
            .map_or(Family::Unknown, |unnamed| data_type_family(&unnamed)),
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
            let family: Family = expression_lineage(expression, scope, context)
                .data_type
                .as_deref()
                .map_or(Family::Unknown, type_family);
            return match expression {
                Expression::Function(function)
                    if family == Family::Unknown
                        && Engine::of(context) == Engine::BigQuery
                        && function.name.eq_ignore_ascii_case("ARRAY") =>
                {
                    Family::Array
                }
                Expression::Function(function)
                    if family == Family::Unknown
                        && Engine::of(context) == Engine::BigQuery
                        && function.name.eq_ignore_ascii_case("STRUCT") =>
                {
                    Family::Struct
                }
                _ => family,
            };
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
        Expression::Neg(unary) | Expression::Pos(unary) => family(&unary.this),
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
        Literal::HexString(_)
        | Literal::BitString(_)
        | Literal::ByteString(_)
        | Literal::RawByteString(_)
        | Literal::JoinedBytes(..) => Family::Binary,
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

/// A type spelling without quotes, its words single-spaced and in lower case.
pub(crate) fn normalized_spelling(spelling: &str) -> String {
    let trimmed: &str = spelling
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\'' | '`'));
    if is_single_spaced_ascii(trimmed) {
        let mut lowered: String = trimmed.to_owned();
        lowered.make_ascii_lowercase();
        return lowered;
    }
    trimmed
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_lowercase()
}

/// Whether `text` is printable ASCII with single spaces between words and none around them.
fn is_single_spaced_ascii(text: &str) -> bool {
    let bytes: &[u8] = text.as_bytes();
    bytes.first() != Some(&b' ')
        && bytes.last() != Some(&b' ')
        && !text.contains("  ")
        && bytes
            .iter()
            .all(|byte| *byte == b' ' || byte.is_ascii_graphic())
}

/// The family of an array, map or struct spelling, or of a decimal that holds only integers.
pub(crate) fn composite_family(normalized: &str) -> Option<Family> {
    if let Some((base, inner)) = type_arguments(normalized) {
        match base {
            "array" | "list" => return Some(Family::Array),
            "map" => return Some(Family::Map),
            "struct" | "row" | "record" => return Some(Family::Struct),
            "number" | "numeric" | "decimal" | "dec"
                if inner
                    .split(',')
                    .nth(1)
                    .is_some_and(|scale| scale.trim() == INTEGER_SCALE) =>
            {
                return Some(Family::Integer);
            }
            _ => {}
        }
    }
    if ["array<", "list<"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
        || normalized.ends_with("[]")
    {
        return Some(Family::Array);
    }
    if normalized.starts_with("map<") || normalized.starts_with("map(") {
        return Some(Family::Map);
    }
    if ["struct<", "struct(", "row<", "record<", "object<"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return Some(Family::Struct);
    }
    None
}

/// The family of a scalar spelling by its base name, its parameters and signedness dropped.
pub(crate) fn scalar_family(normalized: &str) -> Family {
    let base: &str = normalized.split('(').next().unwrap_or_default().trim();
    let base: &str = base.strip_prefix("unsigned ").unwrap_or(base);
    base_family(base.strip_suffix(" unsigned").unwrap_or(base))
}
