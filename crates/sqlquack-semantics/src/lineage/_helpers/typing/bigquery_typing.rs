//! BigQuery's types as GoogleSQL's analyzer gives them; what these rules miss stays untyped.

use std::rc::Rc;

use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::{DataType, Expression};

use crate::lineage::_helpers::checks::coercion::string_literal;
use crate::lineage::constants::DATE_FROM_PARTS_ARGUMENTS;
use crate::lineage::models::{Lineage, LineageContext};

/// BigQuery's scalar types, as lineage reasons about them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Int64,
    Numeric,
    BigNumeric,
    Float64,
    String,
    Bytes,
    Bool,
    Date,
    Datetime,
    Time,
    Timestamp,
    Interval,
    Json,
    Geography,
}

impl Kind {
    fn spelling(self) -> &'static str {
        match self {
            Self::Int64 => "INT64",
            Self::Numeric => "NUMERIC",
            Self::BigNumeric => "BIGNUMERIC",
            Self::Float64 => "FLOAT64",
            Self::String => "STRING",
            Self::Bytes => "BYTES",
            Self::Bool => "BOOL",
            Self::Date => "DATE",
            Self::Datetime => "DATETIME",
            Self::Time => "TIME",
            Self::Timestamp => "TIMESTAMP",
            Self::Interval => "INTERVAL",
            Self::Json => "JSON",
            Self::Geography => "GEOGRAPHY",
        }
    }

    /// The order of the numeric types, each coercing to the ones after it.
    fn numeric_rank(self) -> Option<u8> {
        match self {
            Self::Int64 => Some(0),
            Self::Numeric => Some(1),
            Self::BigNumeric => Some(2),
            Self::Float64 => Some(3),
            _ => None,
        }
    }

    fn of(spelling: &str) -> Option<Self> {
        let upper: String = spelling.trim().to_ascii_uppercase();
        let base: &str = upper.split(['(', '<']).next().unwrap_or_default().trim();
        Some(match base {
            "INT64" | "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" | "BYTEINT" => {
                Self::Int64
            }
            "NUMERIC" | "DECIMAL" => Self::Numeric,
            "BIGNUMERIC" | "BIGDECIMAL" => Self::BigNumeric,
            "FLOAT64" | "FLOAT" | "DOUBLE" | "DOUBLE PRECISION" | "REAL" => Self::Float64,
            "STRING" | "VARCHAR" | "TEXT" | "CHAR" => Self::String,
            "BYTES" => Self::Bytes,
            "BOOL" | "BOOLEAN" => Self::Bool,
            "DATE" => Self::Date,
            "DATETIME" => Self::Datetime,
            "TIME" => Self::Time,
            "TIMESTAMP" | "TIMESTAMPTZ" => Self::Timestamp,
            "INTERVAL" => Self::Interval,
            "JSON" => Self::Json,
            "GEOGRAPHY" => Self::Geography,
            _ => return None,
        })
    }
}

/// A literal's BigQuery type: a number with a point or exponent is FLOAT64.
pub(crate) fn bigquery_literal_type(expression: &Expression) -> Option<&'static str> {
    let kind: Kind = match expression {
        Expression::Boolean(_) => Kind::Bool,
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::Number(text) if text.contains(['.', 'e', 'E']) => Kind::Float64,
            Literal::Number(_) | Literal::HexNumber(_) => Kind::Int64,
            Literal::String(_) | Literal::TripleQuotedString(..) | Literal::RawString(_) => {
                Kind::String
            }
            Literal::ByteString(_) => Kind::Bytes,
            Literal::Date(_) => Kind::Date,
            Literal::Datetime(_) => Kind::Datetime,
            Literal::Time(_) => Kind::Time,
            Literal::Timestamp(_) => Kind::Timestamp,
            _ => return None,
        },
        _ => return None,
    };
    Some(kind.spelling())
}

/// A cast target as BigQuery spells it: DATETIME is a TIMESTAMP without a time zone.
pub(crate) fn bigquery_cast_spelling(data_type: &DataType) -> Option<String> {
    Some(match data_type {
        DataType::Boolean => "BOOL".to_owned(),
        DataType::BigInt { .. } | DataType::Int { .. } => "INT64".to_owned(),
        DataType::Double { .. } | DataType::Float { .. } => "FLOAT64".to_owned(),
        DataType::Decimal {
            precision: None, ..
        } => "NUMERIC".to_owned(),
        DataType::Decimal {
            precision: Some(precision),
            scale: None,
        } => format!("NUMERIC({precision})"),
        DataType::Decimal {
            precision: Some(precision),
            scale: Some(scale),
        } => format!("NUMERIC({precision},{scale})"),
        DataType::String { .. } | DataType::VarChar { .. } | DataType::Text => "STRING".to_owned(),
        DataType::VarBinary { .. } => "BYTES".to_owned(),
        DataType::Date => "DATE".to_owned(),
        DataType::Time { .. } => "TIME".to_owned(),
        DataType::Timestamp {
            timezone: false, ..
        } => "DATETIME".to_owned(),
        DataType::Timestamp { timezone: true, .. } => "TIMESTAMP".to_owned(),
        DataType::Interval { .. } => "INTERVAL".to_owned(),
        DataType::Json => "JSON".to_owned(),
        DataType::Array { element_type, .. } => {
            format!("ARRAY<{}>", bigquery_cast_spelling(element_type)?)
        }
        DataType::Custom { name } => {
            let kind: Kind = Kind::of(name)?;
            match kind {
                Kind::BigNumeric | Kind::Geography => name.to_ascii_uppercase().replace(", ", ","),
                Kind::String | Kind::Bytes if name.to_ascii_uppercase().ends_with("(MAX)") => {
                    kind.spelling().to_owned()
                }
                _ => return None,
            }
        }
        _ => return None,
    })
}

/// The type of computed `expression` under BigQuery's rules, given its children's `parts`.
pub(crate) fn bigquery_type(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> Option<Rc<str>> {
    let name: Option<&str> = match expression {
        Expression::Function(function) => Some(&function.name),
        Expression::AggregateFunction(function) => Some(&function.name),
        _ => None,
    };
    if let Some(declared) = name.and_then(|name| context.schema.function_type(name)) {
        return Some(Rc::from(declared));
    }
    let operand = |child: &Expression| operand_kind(child, parts);
    let kind: Kind = match expression {
        Expression::Eq(_)
        | Expression::Neq(_)
        | Expression::Lt(_)
        | Expression::Lte(_)
        | Expression::Gt(_)
        | Expression::Gte(_)
        | Expression::And(_)
        | Expression::Or(_)
        | Expression::Not(_)
        | Expression::In(_)
        | Expression::Between(_)
        | Expression::Like(_)
        | Expression::Is(_)
        | Expression::IsNull(_)
        | Expression::Exists(_) => Kind::Bool,
        Expression::Neg(unary) => {
            let kind: Kind = operand(&unary.this)?;
            (kind.numeric_rank().is_some() || kind == Kind::Interval).then_some(kind)?
        }
        Expression::Interval(_) => Kind::Interval,
        Expression::Add(operation) | Expression::Sub(operation) | Expression::Mul(operation) => {
            let (left, right) = (operand(&operation.left)?, operand(&operation.right)?);
            arithmetic_kind(expression, left, right)
                .or_else(|| literal_supertype(&[&operation.left, &operation.right], parts))?
        }
        Expression::Div(operation) => {
            let (left, right) = (operand(&operation.left)?, operand(&operation.right)?);
            match (left, right) {
                (Kind::Interval, Kind::Int64) => Kind::Interval,
                (Kind::Int64, Kind::Int64) => Kind::Float64,
                _ => literal_supertype(&[&operation.left, &operation.right], parts)?,
            }
        }
        Expression::Concat(concat) => concat_kind(&[&concat.left, &concat.right], parts)?,
        Expression::Case(case) => {
            let mut values: Vec<&Expression> = case.whens.iter().map(|(_, value)| value).collect();
            values.extend(case.else_.iter());
            supertype(&values, parts)?
        }
        Expression::WindowFunction(window) => return spelled(&window.this, parts),
        Expression::Function(function) => {
            let arguments: Vec<&Expression> = function.args.iter().collect();
            return function_type(&function.name, &arguments, parts);
        }
        Expression::AggregateFunction(function) => {
            let arguments: Vec<&Expression> = function.args.iter().collect();
            return function_type(&function.name, &arguments, parts);
        }
        _ => return None,
    };
    Some(Rc::from(kind.spelling()))
}

/// The type of `child` as its lineage spells it, an ARRAY or STRUCT included.
fn spelled(child: &Expression, parts: &[(&Expression, Lineage)]) -> Option<Rc<str>> {
    parts
        .iter()
        .find(|(candidate, _)| std::ptr::eq(*candidate, child))
        .and_then(|(_, part)| part.data_type.clone())
}

/// A built-in's result type, from its name and its arguments' types.
fn function_type(
    name: &str,
    arguments: &[&Expression],
    parts: &[(&Expression, Lineage)],
) -> Option<Rc<str>> {
    let first = || spelled(arguments.first()?, parts);
    match name.to_ascii_uppercase().as_str() {
        "MIN" | "MAX" | "ANY_VALUE" | "LAG" | "LEAD" | "FIRST_VALUE" | "LAST_VALUE"
        | "NTH_VALUE" => first(),
        "ARRAY_AGG" => {
            let element: Rc<str> = first()?;
            (!element.to_ascii_uppercase().starts_with("ARRAY"))
                .then(|| Rc::from(format!("ARRAY<{element}>")))
        }
        "SPLIT" => match operand_kind(arguments.first()?, parts) {
            Some(Kind::Bytes) => Some(Rc::from("ARRAY<BYTES>")),
            Some(Kind::String) => Some(Rc::from("ARRAY<STRING>")),
            _ => None,
        },
        _ => function_kind(name, arguments, parts).map(|kind| Rc::from(kind.spelling())),
    }
}

/// Whether `child` is a number literal, which takes a NUMERIC or BIGNUMERIC operand's type.
fn number_literal(child: &Expression) -> bool {
    match child {
        Expression::Literal(literal) => {
            matches!(literal.as_ref(), Literal::Number(_) | Literal::HexNumber(_))
        }
        Expression::Neg(unary) => number_literal(&unary.this),
        Expression::Paren(paren) => number_literal(&paren.this),
        _ => false,
    }
}

/// The numeric supertype of `operands`; a number literal takes a NUMERIC operand's type.
fn literal_supertype(operands: &[&Expression], parts: &[(&Expression, Lineage)]) -> Option<Kind> {
    let kinds: Vec<Kind> = operands
        .iter()
        .map(|operand| operand_kind(operand, parts))
        .collect::<Option<_>>()?;
    let columns: Vec<Kind> = operands
        .iter()
        .zip(&kinds)
        .filter(|(operand, _)| !number_literal(operand))
        .map(|(_, kind)| *kind)
        .collect();
    match numeric_supertype(&columns) {
        Some(kind @ (Kind::Numeric | Kind::BigNumeric)) if numeric_supertype(&kinds).is_some() => {
            Some(kind)
        }
        _ => numeric_supertype(&kinds),
    }
}

/// The type of `||` and CONCAT: BYTES for BYTES operands and string literals, else STRING.
fn concat_kind(operands: &[&Expression], parts: &[(&Expression, Lineage)]) -> Option<Kind> {
    let kinds: Vec<Option<Kind>> = operands
        .iter()
        .map(|operand| match string_literal(operand) {
            true => None,
            false => operand_kind(operand, parts)
                .map(Some)
                .unwrap_or(Some(Kind::Json)),
        })
        .collect();
    let bytes: bool = kinds.contains(&Some(Kind::Bytes));
    if kinds
        .iter()
        .all(|kind| kind.is_none() || *kind == Some(Kind::Bytes))
        && bytes
    {
        return Some(Kind::Bytes);
    }
    let scalar = |kind: &Option<Kind>| {
        kind.is_none_or(|kind| !matches!(kind, Kind::Bytes | Kind::Json | Kind::Geography))
    };
    kinds.iter().all(scalar).then_some(Kind::String)
}

/// The type BigQuery combines set-operation branches into, each with whether it is a constant.
pub(crate) fn bigquery_common_type(branches: &[(Option<Rc<str>>, bool)]) -> Option<Rc<str>> {
    let kinds: Vec<Kind> = branches
        .iter()
        .map(|(data_type, _)| Kind::of(data_type.as_deref()?))
        .collect::<Option<_>>()?;
    let combined: Kind = combined_kind(&kinds)?;
    let decimal: bool = kinds
        .iter()
        .any(|kind| matches!(kind, Kind::Numeric | Kind::BigNumeric));
    let constant_float: bool = kinds
        .iter()
        .zip(branches)
        .any(|(kind, (_, constant))| *kind == Kind::Float64 && *constant);
    (!(decimal && constant_float)).then(|| Rc::from(combined.spelling()))
}

/// The type an operand has, a NULL or string literal aside: those take the other operands' type.
fn operand_kind(child: &Expression, parts: &[(&Expression, Lineage)]) -> Option<Kind> {
    if string_literal(child) {
        return None;
    }
    let part: &Lineage = parts
        .iter()
        .find(|(candidate, _)| std::ptr::eq(*candidate, child))
        .map(|(_, part)| part)?;
    Kind::of(part.data_type.as_deref()?)
}

fn arithmetic_kind(node: &Expression, left: Kind, right: Kind) -> Option<Kind> {
    use Kind::{Date, Datetime, Int64, Interval, Time, Timestamp};
    let add: bool = matches!(node, Expression::Add(_));
    let sub: bool = matches!(node, Expression::Sub(_));
    let mul: bool = matches!(node, Expression::Mul(_));
    Some(match (left, right) {
        (Date, Int64) | (Int64, Date) if add => Date,
        (Date, Int64) if sub => Date,
        (Date, Interval) | (Interval, Date) if add => Datetime,
        (Date, Interval) if sub => Datetime,
        (Datetime, Interval) | (Interval, Datetime) if add => Datetime,
        (Datetime, Interval) if sub => Datetime,
        (Timestamp, Interval) | (Interval, Timestamp) if add => Timestamp,
        (Timestamp, Interval) if sub => Timestamp,
        (Date, Date) | (Datetime, Datetime) | (Timestamp, Timestamp) | (Time, Time) if sub => {
            Interval
        }
        (Datetime, Date) | (Date, Datetime) if sub => Interval,
        (Interval, Interval) if add || sub => Interval,
        (Interval, other) | (other, Interval) if mul && other.numeric_rank().is_some() => Interval,
        _ => return None,
    })
}

/// The widest numeric type among `kinds`, when every one is numeric.
fn numeric_supertype(kinds: &[Kind]) -> Option<Kind> {
    let ranks: Vec<u8> = kinds
        .iter()
        .map(|kind| kind.numeric_rank())
        .collect::<Option<_>>()?;
    let widest: u8 = *ranks.iter().max()?;
    kinds
        .iter()
        .copied()
        .find(|kind| kind.numeric_rank() == Some(widest))
}

/// The type values of `kinds` combine into: one type, the widest numeric, or DATE with DATETIME.
fn combined_kind(kinds: &[Kind]) -> Option<Kind> {
    let first: Kind = *kinds.first()?;
    if kinds.iter().all(|kind| *kind == first) {
        return Some(first);
    }
    if kinds
        .iter()
        .all(|kind| matches!(kind, Kind::Date | Kind::Datetime))
    {
        return Some(Kind::Datetime);
    }
    numeric_supertype(kinds)
}

/// The supertype of `values`, NULLs and string literals among them aside, when the rest is typed.
fn supertype(values: &[&Expression], parts: &[(&Expression, Lineage)]) -> Option<Kind> {
    let typed: Vec<&Expression> = values
        .iter()
        .copied()
        .filter(|value| !matches!(value, Expression::Null(_)) && !string_literal(value))
        .collect();
    if typed.is_empty() {
        return values
            .iter()
            .any(|value| string_literal(value))
            .then_some(Kind::String);
    }
    let kinds: Vec<Kind> = typed
        .iter()
        .map(|value| operand_kind(value, parts))
        .collect::<Option<_>>()?;
    let combined: Kind = match combined_kind(&kinds)? {
        kind if kind.numeric_rank().is_some() => literal_supertype(&typed, parts)?,
        kind => kind,
    };
    let literals: bool = typed.len()
        < values
            .iter()
            .filter(|value| !matches!(value, Expression::Null(_)))
            .count();
    (!literals
        || matches!(
            combined,
            Kind::String | Kind::Bytes | Kind::Date | Kind::Datetime | Kind::Time | Kind::Timestamp
        ))
    .then_some(combined)
}

/// A built-in's result type from its arguments' types.
fn function_kind(
    name: &str,
    arguments: &[&Expression],
    parts: &[(&Expression, Lineage)],
) -> Option<Kind> {
    let argument = |index: usize| operand_kind(arguments.get(index)?, parts);
    let name: String = name.to_ascii_uppercase();
    let numeric = |kind: Kind| kind.numeric_rank().is_some();
    Some(match name.as_str() {
        "COUNT" | "COUNTIF" | "ROW_NUMBER" | "RANK" | "DENSE_RANK" | "NTILE" | "LENGTH"
        | "CHAR_LENGTH" | "CHARACTER_LENGTH" | "BYTE_LENGTH" | "STRPOS" | "INSTR"
        | "ARRAY_LENGTH" | "DATE_DIFF" | "DATETIME_DIFF" | "TIMESTAMP_DIFF" | "TIME_DIFF"
        | "UNIX_SECONDS" | "UNIX_MILLIS" | "UNIX_MICROS" | "UNIX_DATE" | "FARM_FINGERPRINT" => {
            Kind::Int64
        }
        "PERCENT_RANK" | "CUME_DIST" | "STDDEV" | "STDDEV_POP" | "STDDEV_SAMP" | "VARIANCE"
        | "VAR_POP" | "VAR_SAMP" | "CORR" | "COVAR_POP" | "COVAR_SAMP" | "IEEE_DIVIDE" | "RAND" => {
            Kind::Float64
        }
        "STARTS_WITH" | "ENDS_WITH" | "REGEXP_CONTAINS" | "LOGICAL_AND" | "LOGICAL_OR"
        | "IS_NAN" | "IS_INF" => Kind::Bool,
        "CURRENT_DATE" => Kind::Date,
        "CURRENT_DATETIME" => Kind::Datetime,
        "CURRENT_TIME" => Kind::Time,
        "CURRENT_TIMESTAMP" | "TIMESTAMP_SECONDS" | "TIMESTAMP_MILLIS" | "TIMESTAMP_MICROS"
        | "PARSE_TIMESTAMP" => Kind::Timestamp,
        "PARSE_DATE" => Kind::Date,
        "LAST_DAY" => match argument(0)? {
            Kind::Date | Kind::Datetime => Kind::Date,
            _ => return None,
        },
        "PARSE_DATETIME" => Kind::Datetime,
        "PARSE_TIME" => Kind::Time,
        "TIME_TRUNC" | "TIME_ADD" | "TIME_SUB" => match argument(0)? {
            Kind::Time => Kind::Time,
            _ => return None,
        },
        "TIMESTAMP_TRUNC" | "DATETIME_TRUNC" => match argument(0)? {
            Kind::Date | Kind::Datetime => Kind::Datetime,
            Kind::Timestamp => Kind::Timestamp,
            _ => return None,
        },
        "TIMESTAMP_ADD" | "TIMESTAMP_SUB" | "DATETIME_ADD" | "DATETIME_SUB" => match argument(0)? {
            kind @ (Kind::Datetime | Kind::Timestamp) => kind,
            _ => return None,
        },
        "FORMAT_DATE" | "FORMAT_DATETIME" | "FORMAT_TIMESTAMP" | "FORMAT_TIME" | "FORMAT"
        | "TO_JSON_STRING" | "TO_HEX" | "TO_BASE64" | "JSON_VALUE" | "STRING" | "GENERATE_UUID" => {
            Kind::String
        }
        "MD5" | "SHA1" | "SHA256" | "SHA512" | "FROM_HEX" | "FROM_BASE64" => Kind::Bytes,
        "PARSE_JSON" | "TO_JSON" | "JSON_OBJECT" | "JSON_ARRAY" => Kind::Json,
        "ST_GEOGPOINT" | "ST_GEOGFROMTEXT" | "ST_MAKELINE" => Kind::Geography,
        "INT64" => Kind::Int64,
        "BOOL" => Kind::Bool,
        "FLOAT64" => Kind::Float64,
        "DATE" => match argument(0) {
            Some(Kind::Int64) if arguments.len() == DATE_FROM_PARTS_ARGUMENTS => Kind::Date,
            Some(Kind::Date | Kind::Datetime | Kind::Timestamp | Kind::String) => Kind::Date,
            _ if arguments.first().is_some_and(|first| string_literal(first)) => Kind::Date,
            _ => return None,
        },
        "DATETIME" => Kind::Datetime,
        "TIMESTAMP" => Kind::Timestamp,
        "TIME" => Kind::Time,
        "EXTRACT" => extract_kind(arguments)?,
        "SUM" => match argument(0)? {
            kind if numeric(kind) || kind == Kind::Interval => kind,
            _ => return None,
        },
        "AVG" => match argument(0)? {
            Kind::Int64 => Kind::Float64,
            kind if numeric(kind) || kind == Kind::Interval => kind,
            _ => return None,
        },
        "ABS" | "SIGN" => match argument(0)? {
            kind if numeric(kind) => kind,
            _ => return None,
        },
        "ROUND" | "TRUNC" | "FLOOR" | "CEIL" | "CEILING" => match argument(0)? {
            Kind::Int64 => Kind::Float64,
            kind if numeric(kind) => kind,
            _ => return None,
        },
        "SQRT" | "LN" | "LOG" | "LOG10" | "EXP" | "POW" | "POWER" => match argument(0)? {
            Kind::Int64 | Kind::Float64 => Kind::Float64,
            kind if numeric(kind) => kind,
            _ => return None,
        },
        "MOD" | "DIV" => {
            let kinds: [Kind; 2] = [argument(0)?, argument(1)?];
            match numeric_supertype(&kinds)? {
                Kind::Float64 => return None,
                kind => kind,
            }
        }
        "SAFE_DIVIDE" => match (argument(0)?, argument(1)?) {
            (Kind::Int64, Kind::Int64) => Kind::Float64,
            (left, right) => numeric_supertype(&[left, right])?,
        },
        "UPPER" | "LOWER" | "TRIM" | "LTRIM" | "RTRIM" | "REPLACE" | "SUBSTR" | "SUBSTRING"
        | "LEFT" | "RIGHT" | "LPAD" | "RPAD" | "REVERSE" | "REPEAT" | "REGEXP_REPLACE" => {
            match argument(0) {
                Some(kind @ (Kind::String | Kind::Bytes)) => kind,
                None if arguments.first().is_some_and(|first| string_literal(first)) => {
                    Kind::String
                }
                _ => return None,
            }
        }
        "CONCAT" if !arguments.is_empty() => concat_kind(arguments, parts)?,
        "STRING_AGG" => match argument(0) {
            Some(kind @ (Kind::String | Kind::Bytes)) => kind,
            None if arguments.first().is_some_and(|first| string_literal(first)) => Kind::String,
            _ => return None,
        },
        "DATE_TRUNC" | "DATE_ADD" | "DATE_SUB" => match argument(0) {
            Some(kind @ (Kind::Date | Kind::Datetime | Kind::Timestamp)) => kind,
            _ => return None,
        },
        "COALESCE" | "IFNULL" | "GREATEST" | "LEAST" => supertype(arguments, parts)?,
        "IF" => supertype(arguments.get(1..)?, parts)?,
        "NULLIF" => argument(0)?,
        _ => return None,
    })
}

/// EXTRACT's type: its DATE, TIME and DATETIME parts give those types, every other part INT64.
fn extract_kind(arguments: &[&Expression]) -> Option<Kind> {
    let Some(Expression::Literal(literal)) = arguments.first() else {
        return None;
    };
    let Literal::String(part) = literal.as_ref() else {
        return None;
    };
    Some(match part.to_ascii_uppercase().as_str() {
        "DATE" => Kind::Date,
        "TIME" => Kind::Time,
        "DATETIME" => Kind::Datetime,
        _ => Kind::Int64,
    })
}
