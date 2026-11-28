//! Literals and casts: the values both engines read the same way from the same text.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::texts::read_text;
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::{field, refused, understood};
use crate::databricks::conversion::_helpers::support::typing::{databricks_type, same_cast};
use crate::databricks::conversion::constants::{
    CAST_RULE, CLOCK_TEXT_LENGTH, DATE_TEXT_LENGTH, DOUBLE_EXACT_DIGITS, DOUBLE_MANTISSA_BITS,
    FLOAT_MANTISSA_BITS, FLOAT_MAX, FLOAT_OVERFLOW, FLOAT_RANGE_ERROR, LITERAL_RULE,
    MAX_DECIMAL_PRECISION, MAX_FRACTION_DIGITS, SPECIAL_DOUBLE_TEXTS,
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
        Some("hex_string")
            if value.len().is_multiple_of(2)
                && value.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Ok(typed(
                format!("from_hex({})", quoted_text(value)),
                DatabricksType::Binary,
            ))
        }
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
    cast_value(&source, target).ok_or_else(|| {
        refused(
            "cast",
            format!(
                "no rule shows DuckDB casts {} to {} as Databricks does",
                source.sql,
                target.duckdb_name()
            ),
        )
    })
}

/// `source` cast to `target`, `None` unless DuckDB converts it as Databricks does.
pub(crate) fn cast_value(source: &Emitted, target: DatabricksType) -> Option<Emitted> {
    let text_column = source.literal.is_none() && source.data_type == DatabricksType::String;
    if let Some(read) = read_text(source, target).filter(|_| text_column) {
        return Some(read);
    }
    if source.literal.is_none()
        && (source.data_type, target) == (DatabricksType::Double, DatabricksType::Float)
    {
        return Some(typed(
            format!(
                "(CASE WHEN isfinite({x}) AND abs({x}) >= {FLOAT_OVERFLOW} \
                 THEN CAST(CASE WHEN {x} > 0 THEN 'Infinity' ELSE '-Infinity' END AS FLOAT) \
                 WHEN isfinite({x}) AND abs({x}) > {FLOAT_MAX} \
                 THEN CAST(error({}) AS FLOAT) ELSE CAST({x} AS FLOAT) END)",
                quoted_text(FLOAT_RANGE_ERROR),
                x = source.sql
            ),
            target,
        ));
    }
    let exact = match (&source.literal, source.data_type) {
        (Some(Literal::Number(number)), _) => number_fits(number, target),
        (Some(Literal::Text(value)), _) => text_fits(value, target),
        (None, from) => same_cast(from, target),
    };
    exact.then(|| {
        typed(
            format!("CAST({} AS {})", source.sql, target.duckdb_name()),
            target,
        )
    })
}

/// A unary minus; on a literal number it folds into the literal, typed by its digits alone.
pub(crate) fn negated(operand: Emitted) -> Result<Emitted, Refusal> {
    match (&operand.literal, operand.data_type) {
        (Some(Literal::Number(digits)), _) if !digits.starts_with('-') => {
            number(&format!("-{digits}"))
        }
        (_, data_type) if data_type.is_numeric() => {
            Ok(typed(format!("(- {})", operand.sql), data_type))
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
        if !value
            .parse::<f64>()
            .is_ok_and(|parsed| parsed.abs() < f64::MAX)
        {
            return Err(out_of_range(value));
        }
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
        DatabricksType::Double => {
            exponent
                || whole.len() + fraction.len() <= DOUBLE_EXACT_DIGITS
                || exactly_binary(whole, fraction, DOUBLE_MANTISSA_BITS)
        }
        DatabricksType::Float => !exponent && exactly_binary(whole, fraction, FLOAT_MANTISSA_BITS),
        DatabricksType::Decimal { scale, .. } => {
            !exponent && fraction.len() <= usize::from(scale) && !whole.is_empty()
        }
        DatabricksType::String => !exponent,
        _ => false,
    }
}

/// Whether `whole.fraction` is a binary fraction of at most `bits` significant bits, so no cast rounds it.
fn exactly_binary(whole: &str, fraction: &str, bits: u32) -> bool {
    let Some(digits) = digits_u128(&format!("{whole}{fraction}")) else {
        return false;
    };
    let Ok(scale) = u32::try_from(fraction.len()) else {
        return false;
    };
    let Some(fives) = 5u128.checked_pow(scale) else {
        return false;
    };
    match digits % fives {
        0 => {
            let mantissa = digits / fives;
            mantissa == 0
                || u128::BITS - mantissa.leading_zeros() - mantissa.trailing_zeros() <= bits
        }
        _ => false,
    }
}

/// The value of a string of ASCII digits that fits 128 bits.
fn digits_u128(text: &str) -> Option<u128> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.bytes().try_fold(0u128, |value, byte| {
        value.checked_mul(10)?.checked_add(u128::from(byte - b'0'))
    })
}

/// Whether casting the string `value` to `target` gives the same value, or raises, in both engines.
fn text_fits(value: &str, target: DatabricksType) -> bool {
    match target {
        DatabricksType::String => true,
        DatabricksType::Double | DatabricksType::Float => {
            SPECIAL_DOUBLE_TEXTS.contains(&value) || plain_decimal(value)
        }
        DatabricksType::Date => is_date(value),
        DatabricksType::Timestamp | DatabricksType::TimestampNtz => {
            is_date(value) || is_timestamp(value)
        }
        DatabricksType::Boolean => matches!(value.to_ascii_lowercase().as_str(), "true" | "false"),
        target if target.is_integral() => {
            let digits = value.strip_prefix('-').unwrap_or(value);
            let integer = !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());
            integer || !value.bytes().any(|byte| byte.is_ascii_digit())
        }
        _ => false,
    }
}

/// An optionally negative decimal such as `-2.50`, which both engines parse correctly rounded.
fn plain_decimal(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let plain = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    !whole.is_empty()
        && plain(whole)
        && plain(fraction)
        && (fraction.is_empty() || digits.ends_with(|c: char| c.is_ascii_digit()))
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
