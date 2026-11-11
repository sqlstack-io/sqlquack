//! DuckDB types as the type rules read them: parsed from spellings, rendered, and combined.

use sqlquack_syntax::DataType;

use crate::duckdb::profile::constants::{
    DUCKDB_DECIMAL_LIMIT, DUCKDB_DEFAULT_DECIMAL, DUCKDB_INTEGER_ALIASES, DUCKDB_INTEGER_SPELLINGS,
    DUCKDB_JSON_TYPE, DUCKDB_WIDEST_INTEGER,
};

/// What a DuckDB type is, as far as the type rules distinguish it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DuckType {
    Boolean,
    /// An integer of `bits` bits, unsigned or not.
    Integer(u16, bool),
    /// DECIMAL(precision, scale).
    Decimal(u32, u32),
    /// FLOAT (REAL).
    Real,
    Double,
    Text,
    Binary,
    Date,
    /// TIME, with time zone or not.
    Time(bool),
    /// TIMESTAMP, with time zone or not.
    Timestamp(bool),
    Interval,
    Uuid,
    Array(Box<DuckType>),
}

/// A numeric operand's shape: integer width and signedness, decimal, REAL or DOUBLE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Number {
    Integer(u16, bool),
    Decimal(u32, u32),
    Real,
    Double,
}

impl DuckType {
    /// The numeric shape, when the type is a number (BOOLEAN is not).
    pub(crate) fn number(&self) -> Option<Number> {
        match self {
            DuckType::Integer(bits, unsigned) => Some(Number::Integer(*bits, *unsigned)),
            DuckType::Decimal(precision, scale) => Some(Number::Decimal(*precision, *scale)),
            DuckType::Real => Some(Number::Real),
            DuckType::Double => Some(Number::Double),
            _ => None,
        }
    }

    /// Integer width and signedness; BOOLEAN is width 0, as DuckDB's integer coercion reads it.
    pub(crate) fn integer_shape(&self) -> Option<(u16, bool)> {
        match self {
            DuckType::Boolean => Some((0, false)),
            DuckType::Integer(bits, unsigned) => Some((*bits, *unsigned)),
            _ => None,
        }
    }

    /// DuckDB's own spelling of the type.
    pub(crate) fn spelling(&self) -> String {
        match self {
            DuckType::Boolean => "BOOLEAN".to_owned(),
            DuckType::Integer(bits, unsigned) => integer_spelling(*bits, *unsigned).to_owned(),
            DuckType::Decimal(precision, scale) => format!("DECIMAL({precision},{scale})"),
            DuckType::Real => "FLOAT".to_owned(),
            DuckType::Double => "DOUBLE".to_owned(),
            DuckType::Text => "VARCHAR".to_owned(),
            DuckType::Binary => "BLOB".to_owned(),
            DuckType::Date => "DATE".to_owned(),
            DuckType::Time(false) => "TIME".to_owned(),
            DuckType::Time(true) => "TIMETZ".to_owned(),
            DuckType::Timestamp(false) => "TIMESTAMP".to_owned(),
            DuckType::Timestamp(true) => "TIMESTAMPTZ".to_owned(),
            DuckType::Interval => "INTERVAL".to_owned(),
            DuckType::Uuid => "UUID".to_owned(),
            DuckType::Array(element) => format!("{}[]", element.spelling()),
        }
    }
}

/// The integer type of `bits` bits, as DuckDB spells it.
fn integer_spelling(bits: u16, unsigned: bool) -> &'static str {
    DUCKDB_INTEGER_SPELLINGS
        .iter()
        .find(|(width, is_unsigned, _)| *width == bits && *is_unsigned == unsigned)
        .map_or("UHUGEINT", |(_, _, spelling)| spelling)
}

/// The type a spelling names, DECIMAL's defaults filled in, when the rules model it.
pub(crate) fn parsed_type(spelling: &str) -> Option<DuckType> {
    let upper: String = spelling.trim().to_ascii_uppercase().replace(", ", ",");
    if let Some(element) = upper.strip_suffix("[]") {
        return Some(DuckType::Array(Box::new(parsed_type(element)?)));
    }
    let (base, parameters): (&str, Option<&str>) = match upper.split_once('(') {
        Some((base, rest)) => (base.trim(), Some(rest.trim_end_matches(')'))),
        None => (upper.as_str(), None),
    };
    if let Some((_, (bits, unsigned))) = DUCKDB_INTEGER_ALIASES
        .iter()
        .find(|(names, _)| names.contains(&base))
    {
        return Some(DuckType::Integer(*bits, *unsigned));
    }
    let parsed: DuckType = match (base, parameters) {
        ("DECIMAL" | "NUMERIC" | "DEC", None) => {
            DuckType::Decimal(DUCKDB_DEFAULT_DECIMAL.0, DUCKDB_DEFAULT_DECIMAL.1)
        }
        ("DECIMAL" | "NUMERIC" | "DEC", Some(parameters)) => decimal(parameters)?,
        ("FLOAT" | "FLOAT4" | "REAL", None) => DuckType::Real,
        ("DOUBLE" | "FLOAT8" | "DOUBLE PRECISION", None) => DuckType::Double,
        (
            "VARCHAR" | "TEXT" | "STRING" | "CHAR" | "BPCHAR" | "CHARACTER VARYING" | "CHARACTER",
            _,
        ) => DuckType::Text,
        ("BOOLEAN" | "BOOL" | "LOGICAL", None) => DuckType::Boolean,
        ("BLOB" | "BYTEA" | "BINARY" | "VARBINARY", _) => DuckType::Binary,
        ("DATE", None) => DuckType::Date,
        ("TIME" | "TIME WITHOUT TIME ZONE", None) => DuckType::Time(false),
        ("TIMETZ" | "TIME WITH TIME ZONE", None) => DuckType::Time(true),
        ("TIMESTAMP" | "DATETIME" | "TIMESTAMP WITHOUT TIME ZONE", None) => {
            DuckType::Timestamp(false)
        }
        ("TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE", None) => DuckType::Timestamp(true),
        ("INTERVAL", None) => DuckType::Interval,
        ("UUID", None) => DuckType::Uuid,
        _ => return None,
    };
    Some(parsed)
}

/// DECIMAL(p) is DECIMAL(p,0); DECIMAL(p,s) as written.
fn decimal(parameters: &str) -> Option<DuckType> {
    let numbers: Vec<u32> = match parameters
        .split(',')
        .map(|part| part.trim().parse::<u32>())
        .collect::<Result<Vec<u32>, _>>()
    {
        Ok(numbers) => numbers,
        Err(_not_a_number) => return None,
    };
    let (precision, scale): (u32, u32) = match numbers.as_slice() {
        [precision] => (*precision, 0),
        [precision, scale] => (*precision, *scale),
        _ => return None,
    };
    (scale <= precision).then_some(DuckType::Decimal(precision, scale))
}

/// The type a cast target names, when the rules model it.
pub(crate) fn cast_type(data_type: &DataType) -> Option<DuckType> {
    let parsed: DuckType = match data_type {
        DataType::Boolean => DuckType::Boolean,
        DataType::TinyInt { .. } => DuckType::Integer(8, false),
        DataType::SmallInt { .. } => DuckType::Integer(16, false),
        DataType::Int { .. } => DuckType::Integer(32, false),
        DataType::BigInt { .. } => DuckType::Integer(64, false),
        DataType::Int128 => DuckType::Integer(128, false),
        DataType::UInt8 => DuckType::Integer(8, true),
        DataType::UInt16 => DuckType::Integer(16, true),
        DataType::UInt32 => DuckType::Integer(32, true),
        DataType::UInt64 => DuckType::Integer(64, true),
        DataType::UInt128 => DuckType::Integer(128, true),
        DataType::Float {
            precision: None, ..
        } => DuckType::Real,
        DataType::Double { .. } => DuckType::Double,
        DataType::Decimal { precision, scale } => match (precision, scale) {
            (None, _) => DuckType::Decimal(DUCKDB_DEFAULT_DECIMAL.0, DUCKDB_DEFAULT_DECIMAL.1),
            (Some(precision), scale) => DuckType::Decimal(*precision, scale.unwrap_or(0)),
        },
        DataType::Char { .. }
        | DataType::VarChar { .. }
        | DataType::String { .. }
        | DataType::Text
        | DataType::TextWithLength { .. } => DuckType::Text,
        DataType::Binary { .. } | DataType::VarBinary { .. } | DataType::Blob => DuckType::Binary,
        DataType::Date => DuckType::Date,
        DataType::Time { timezone, .. } => DuckType::Time(*timezone),
        DataType::Timestamp { timezone, .. } => DuckType::Timestamp(*timezone),
        DataType::Interval { .. } => DuckType::Interval,
        DataType::Uuid => DuckType::Uuid,
        DataType::Array { element_type, .. } | DataType::List { element_type } => {
            DuckType::Array(Box::new(cast_type(element_type)?))
        }
        _ => return None,
    };
    Some(parsed)
}

/// A cast target as DuckDB spells it (nested types included; sized text as written).
pub(crate) fn cast_spelling(data_type: &DataType) -> Option<String> {
    match data_type {
        DataType::VarChar {
            length: Some(length),
            ..
        }
        | DataType::TextWithLength { length } => Some(format!("VARCHAR({length})")),
        DataType::Struct { fields, .. } => {
            let mut spelled: Vec<String> = Vec::with_capacity(fields.len());
            for field in fields {
                spelled.push(format!(
                    "{} {}",
                    field.name,
                    cast_spelling(&field.data_type)?
                ));
            }
            Some(format!("STRUCT({})", spelled.join(", ")))
        }
        DataType::Map {
            key_type,
            value_type,
        } => Some(format!(
            "MAP({}, {})",
            cast_spelling(key_type)?,
            cast_spelling(value_type)?
        )),
        DataType::Array { element_type, .. } | DataType::List { element_type } => {
            Some(format!("{}[]", cast_spelling(element_type)?))
        }
        DataType::Json => Some(DUCKDB_JSON_TYPE.to_owned()),
        _ => cast_type(data_type).map(|target| target.spelling()),
    }
}

/// The signed integer type of at least `bits` bits, DOUBLE past 128.
fn signed_integer(bits: u16) -> DuckType {
    match bits {
        0..=8 => DuckType::Integer(8, false),
        9..=16 => DuckType::Integer(16, false),
        17..=32 => DuckType::Integer(32, false),
        33..=64 => DuckType::Integer(64, false),
        65..=128 => DuckType::Integer(128, false),
        _ => DuckType::Double,
    }
}

/// DuckDB's coercion of an unsigned integer with another integer, for arithmetic or combination.
pub(crate) fn unsigned_coercion(
    left: &DuckType,
    right: &DuckType,
    arithmetic: bool,
) -> Option<DuckType> {
    let (left_bits, left_unsigned) = left.integer_shape()?;
    let (right_bits, right_unsigned) = right.integer_shape()?;
    if !left_unsigned && !right_unsigned {
        return None;
    }
    if left_unsigned && right_unsigned {
        return Some(if left_bits >= right_bits { left } else { right }.clone());
    }
    let (signed, unsigned, signed_type, unsigned_type) = if left_unsigned {
        (right_bits, left_bits, right, left)
    } else {
        (left_bits, right_bits, left, right)
    };
    if signed == 0 && !arithmetic {
        return Some(unsigned_type.clone());
    }
    if signed > unsigned {
        return Some(signed_type.clone());
    }
    if unsigned == DUCKDB_WIDEST_INTEGER {
        return Some(if arithmetic {
            DuckType::Double
        } else {
            signed_integer(signed + 1)
        });
    }
    let required: u16 = unsigned + 1;
    Some(signed_integer(if arithmetic {
        required.max(64)
    } else {
        required
    }))
}

/// The type DuckDB combines two values into, as CASE, COALESCE and set operations do.
pub(crate) fn common_type(left: &DuckType, right: &DuckType) -> Option<DuckType> {
    if left == right {
        return Some(left.clone());
    }
    if let (
        DuckType::Integer(left_bits, left_unsigned),
        DuckType::Integer(right_bits, right_unsigned),
    ) = (left, right)
    {
        if left_unsigned == right_unsigned {
            return Some(if left_bits >= right_bits { left } else { right }.clone());
        }
        return unsigned_coercion(left, right, false);
    }
    if let (Some(left_number), Some(right_number)) = (left.number(), right.number()) {
        if matches!(left_number, Number::Real | Number::Double)
            || matches!(right_number, Number::Real | Number::Double)
        {
            return Some(
                if left_number == Number::Double || right_number == Number::Double {
                    DuckType::Double
                } else if left_number == Number::Real {
                    left.clone()
                } else {
                    right.clone()
                },
            );
        }
        return common_decimal(left_number, right_number);
    }
    match (left, right) {
        (DuckType::Boolean, DuckType::Integer(..)) => return Some(right.clone()),
        (DuckType::Integer(..), DuckType::Boolean) => return Some(left.clone()),
        (DuckType::Text, DuckType::Text) => return Some(DuckType::Text),
        (DuckType::Binary, DuckType::Binary) => return Some(DuckType::Binary),
        _ => {}
    }
    let text: bool = matches!(left, DuckType::Text) || matches!(right, DuckType::Text);
    let scalar = |value: &DuckType| {
        value.number().is_some() || temporal(value) || matches!(value, DuckType::Boolean)
    };
    if text && (scalar(left) || scalar(right)) {
        return Some(DuckType::Text);
    }
    match (left, right) {
        (DuckType::Date, DuckType::Timestamp(_)) => Some(right.clone()),
        (DuckType::Timestamp(_), DuckType::Date) => Some(left.clone()),
        (DuckType::Array(left), DuckType::Array(right)) => {
            Some(DuckType::Array(Box::new(common_type(left, right)?)))
        }
        _ => None,
    }
}

fn temporal(value: &DuckType) -> bool {
    matches!(
        value,
        DuckType::Date | DuckType::Time(_) | DuckType::Timestamp(_) | DuckType::Interval
    )
}

/// The decimal both numbers fit, integers read as DECIMAL(width, 0).
fn common_decimal(left: Number, right: Number) -> Option<DuckType> {
    let (left_precision, left_scale) = decimal_shape(left, true)?;
    let (right_precision, right_scale) = decimal_shape(right, true)?;
    let scale: u32 = left_scale.max(right_scale);
    let integral: u32 = (left_precision - left_scale).max(right_precision - right_scale);
    let precision: u32 = integral.checked_add(scale)?;
    if precision <= DUCKDB_DECIMAL_LIMIT {
        return Some(DuckType::Decimal(precision, scale));
    }
    (integral <= DUCKDB_DECIMAL_LIMIT)
        .then(|| DuckType::Decimal(DUCKDB_DECIMAL_LIMIT, DUCKDB_DECIMAL_LIMIT - integral))
}

/// A number's decimal precision and scale; a HUGEINT has 39 digits when combining, 38 otherwise.
pub(crate) fn decimal_shape(value: Number, combining: bool) -> Option<(u32, u32)> {
    match value {
        Number::Decimal(precision, scale) => Some((precision, scale)),
        Number::Integer(8, _) => Some((3, 0)),
        Number::Integer(16, _) => Some((5, 0)),
        Number::Integer(32, _) => Some((10, 0)),
        Number::Integer(64, false) => Some((19, 0)),
        Number::Integer(64, true) => Some((20, 0)),
        Number::Integer(128, _) if combining => Some((39, 0)),
        Number::Integer(_, _) => Some((DUCKDB_DECIMAL_LIMIT, 0)),
        Number::Real | Number::Double => None,
    }
}
