//! Literals and casts: the values both engines read the same way from the same text.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::expression;
use crate::databricks::conversion::_helpers::scope::Site;
use crate::databricks::conversion::_helpers::tree::{field, refused, understood};
use crate::databricks::conversion::_helpers::typing::{databricks_type, same_cast};
use crate::databricks::conversion::constants::{
    CAST_RULE, CLOCK_TEXT_LENGTH, DATE_TEXT_LENGTH, DOUBLE_EXACT_DIGITS, FLOAT_EXACT_DIGITS,
    LITERAL_RULE, MAX_DECIMAL_PRECISION, MAX_FRACTION_DIGITS,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal};

/// A literal number, string, date or timestamp.
pub(crate) fn literal(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let literal = understood(payload, "literal", &["literal_type", "value"])?;
    let value = field(literal, "value").as_str().unwrap_or_default();
    let _rule = site.log.rule(LITERAL_RULE);
    match field(literal, "literal_type").as_str() {
        Some("number") => number(value),
        Some("string") => Ok(text(value)),
        Some("date") if is_date(value) => Ok(typed(
            format!("DATE {}", quoted_text(value)),
            DatabricksType::Date,
        )),
        Some("timestamp") if is_timestamp(value) => Ok(typed(
            format!("CAST({} AS TIMESTAMPTZ)", quoted_text(value)),
            DatabricksType::Timestamp,
        )),
        Some(kind) => Err(refused(
            "literal",
            format!("no rule reads the {kind} literal `{value}`"),
        )),
        None => Err(refused("literal", "the literal has no type".to_owned())),
    }
}

/// `TRUE` or `FALSE`.
pub(crate) fn boolean(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let boolean = understood(payload, "boolean", &["value"])?;
    let _rule = site.log.rule(LITERAL_RULE);
    let sql = match field(boolean, "value").as_bool() {
        Some(true) => "TRUE",
        _ => "FALSE",
    };
    Ok(typed(sql.to_owned(), DatabricksType::Boolean))
}

/// A bare `NULL`.
pub(crate) fn null(site: Site<'_>) -> Emitted {
    let _rule = site.log.rule(LITERAL_RULE);
    typed("NULL".to_owned(), DatabricksType::Null)
}

/// `CAST(x AS T)`, when DuckDB converts the value as Databricks does.
pub(crate) fn cast(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let cast = understood(payload, "cast", &["this", "to"])?;
    let target = databricks_type(field(cast, "to"))?;
    let source = expression(field(cast, "this"), site)?;
    let _rule = site.log.rule(CAST_RULE);
    let exact = match (&source.literal, source.data_type) {
        (Some(Literal::Number(number)), _) => number_fits(number, target),
        (Some(Literal::Text(value)), _) => text_fits(value, target),
        (None, from) => same_cast(from, target),
    };
    match exact {
        true => Ok(typed(
            format!("CAST({} AS {})", source.sql, target.duckdb_name()),
            target,
        )),
        false => Err(refused(
            "cast",
            format!(
                "no rule shows DuckDB casts {} to {} as Databricks does",
                source.sql,
                target.duckdb_name()
            ),
        )),
    }
}

/// A unary minus; on a literal number it folds into the literal, typed by its digits alone.
pub(crate) fn negated(operand: Emitted) -> Result<Emitted, Refusal> {
    match (&operand.literal, operand.data_type) {
        (Some(Literal::Number(digits)), _) if !digits.starts_with('-') => {
            number(&format!("-{digits}"))
        }
        (_, data_type) if data_type.is_numeric() => {
            Ok(typed(format!("(-{})", operand.sql), data_type))
        }
        _ => Err(refused(
            "neg",
            format!("no rule negates {}, which is not a number", operand.sql),
        )),
    }
}

/// A number typed as Databricks types it, `-2147483648` a negated BIGINT: INT, BIGINT, DECIMAL or DOUBLE.
fn number(value: &str) -> Result<Emitted, Refusal> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    let (mantissa, exponent) = match digits.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (digits, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let plain = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    let exponent_plain = exponent.is_none_or(|exponent| {
        let unsigned = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        !unsigned.is_empty() && plain(unsigned)
    });
    if whole.is_empty() || !plain(whole) || !plain(fraction) || !exponent_plain {
        return Err(refused(
            "literal",
            format!("no rule reads the number `{value}`"),
        ));
    }
    let literal = Some(Literal::Number(value.to_owned()));
    if exponent.is_some() {
        return Ok(Emitted {
            sql: value.to_owned(),
            data_type: DatabricksType::Double,
            literal,
        });
    }
    if !mantissa.contains('.') {
        let data_type = match whole.parse::<u64>() {
            Ok(number) if number <= i32::MAX as u64 => DatabricksType::Int,
            Ok(number) if number <= i64::MAX as u64 => DatabricksType::BigInt,
            _ => return Err(out_of_range(value)),
        };
        let sql = match data_type {
            DatabricksType::BigInt => format!("CAST({value} AS BIGINT)"),
            _ => value.to_owned(),
        };
        return Ok(Emitted {
            sql,
            data_type,
            literal,
        });
    }
    let scale = fraction.len();
    let precision = (whole.trim_start_matches('0').len() + scale).max(1);
    match (u8::try_from(precision), u8::try_from(scale)) {
        (Ok(precision), Ok(scale)) if precision <= MAX_DECIMAL_PRECISION => {
            let data_type = DatabricksType::Decimal { precision, scale };
            Ok(Emitted {
                sql: format!("CAST({value} AS {})", data_type.duckdb_name()),
                data_type,
                literal,
            })
        }
        _ => Err(out_of_range(value)),
    }
}

fn out_of_range(value: &str) -> Refusal {
    refused(
        "literal",
        format!(
            "the number `{value}` is outside the integers and decimals both engines type alike"
        ),
    )
}

/// A string literal.
fn text(value: &str) -> Emitted {
    Emitted {
        sql: quoted_text(value),
        data_type: DatabricksType::String,
        literal: Some(Literal::Text(value.to_owned())),
    }
}

/// `value` as a DuckDB string literal.
pub(crate) fn quoted_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// An expression of a known type that is no literal.
pub(crate) fn typed(sql: String, data_type: DatabricksType) -> Emitted {
    Emitted {
        sql,
        data_type,
        literal: None,
    }
}

/// Whether casting the number `number` to `target` gives the same value, or raises, in both engines.
fn number_fits(number: &str, target: DatabricksType) -> bool {
    let unsigned = number.trim_start_matches('-');
    let exponent = unsigned.contains(['e', 'E']);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    match target {
        target if target.is_integral() => !exponent && fraction.is_empty(),
        DatabricksType::Double => exponent || whole.len() + fraction.len() <= DOUBLE_EXACT_DIGITS,
        DatabricksType::Float => {
            !exponent && fraction.is_empty() && whole.len() <= FLOAT_EXACT_DIGITS
        }
        DatabricksType::Decimal { scale, .. } => {
            !exponent && fraction.len() <= usize::from(scale) && !whole.is_empty()
        }
        DatabricksType::String => !exponent,
        _ => false,
    }
}

/// Whether casting the string `value` to `target` gives the same value in both engines.
fn text_fits(value: &str, target: DatabricksType) -> bool {
    match target {
        DatabricksType::String => true,
        DatabricksType::Date => is_date(value),
        DatabricksType::Timestamp | DatabricksType::TimestampNtz => {
            is_date(value) || is_timestamp(value)
        }
        DatabricksType::Boolean => matches!(value.to_ascii_lowercase().as_str(), "true" | "false"),
        target if target.is_integral() => {
            let digits = value.strip_prefix('-').unwrap_or(value);
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        }
        _ => false,
    }
}

/// `YYYY-MM-DD`; both engines raise alike on an impossible date.
fn is_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == DATE_TEXT_LENGTH
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// `YYYY-MM-DD HH:MM:SS` with up to six fraction digits.
fn is_timestamp(value: &str) -> bool {
    let Some((date, time)) = value.split_once(' ') else {
        return false;
    };
    let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
    let clock = clock.as_bytes();
    is_date(date)
        && clock.len() == CLOCK_TEXT_LENGTH
        && clock.iter().enumerate().all(|(index, byte)| match index {
            2 | 5 => *byte == b':',
            _ => byte.is_ascii_digit(),
        })
        && fraction.len() <= MAX_FRACTION_DIGITS
        && (fraction.is_empty() == !time.contains('.'))
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}
