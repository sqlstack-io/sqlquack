//! A DATE or naive timestamp plus or minus an interval literal, such as `d - INTERVAL '7 days'`.

use serde_json::Value;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::temporal::main::date_add_value::date_add_value;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, is_inert, node_construct, node_kind, payload_object,
};
use crate::conversion::constants::{
    INTERVAL_ARITHMETIC, INTERVAL_DATE_UNITS, INTERVAL_FIELDS, INTERVAL_KIND, INTERVAL_MAX_DIGITS,
    INTERVAL_REFUSAL, INTERVAL_SIMPLE_UNIT, INTERVAL_TIMESTAMP_PRECISION, INTERVAL_UNITS,
};
use sqlquack_policy::policy::constants::TIMESTAMP_RANGE;

/// Whether `payload`, a `+` or `-`, adds an interval to its left operand.
pub(crate) fn adds_interval(payload: &Value) -> bool {
    payload
        .get("right")
        .and_then(node_kind)
        .is_some_and(|(kind, _)| kind == INTERVAL_KIND)
}

/// `value + INTERVAL 'n unit'` or `value - INTERVAL 'n unit'`, as DATEADD of n or -n units.
pub(crate) fn interval_arithmetic(
    (kind, node): (&str, &Value),
    payload: &Value,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let log = site.context.log;
    let _rule = log.rule(INTERVAL_ARITHMETIC);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let payload = payload_object(payload, &construct)?;
    let value = convert_expression(field(payload, "left", &construct)?, site)?;
    let interval = field(payload, "right", &construct)?;
    let quoted_value = site
        .context
        .locator
        .source_text(node)
        .is_some_and(|written| {
            let lower = written.to_ascii_lowercase();
            lower.rfind(INTERVAL_KIND).is_some_and(|start| {
                lower[start + INTERVAL_KIND.len()..]
                    .trim_start()
                    .starts_with('\'')
            })
        });
    if !quoted_value {
        return Err(refuse(
            "an INTERVAL whose value is not a quoted string, which no recording shows".to_owned(),
        ));
    }
    let (amount, unit) =
        interval_literal(interval, site)?.ok_or_else(|| refuse(INTERVAL_REFUSAL.to_owned()))?;
    let accepted = match value.data_type {
        SnowflakeType::Date => INTERVAL_DATE_UNITS.contains(&unit),
        SnowflakeType::TimestampNtz { .. } => true,
        _ => false,
    };
    if !accepted {
        return Err(refuse(format!(
            "an INTERVAL in {unit} added to {}",
            value.data_type
        )));
    }
    let unit_outside_quotes = node_kind(interval)
        .and_then(|(_, payload)| payload.get("unit"))
        .is_some_and(|unit| !is_inert(unit));
    let result_type = match value.data_type {
        SnowflakeType::Date if !unit_outside_quotes => SnowflakeType::Date,
        SnowflakeType::Date | SnowflakeType::TimestampNtz { .. } => {
            diverge(site.context, TIMESTAMP_RANGE, &construct)?;
            SnowflakeType::TimestampNtz {
                precision: INTERVAL_TIMESTAMP_PRECISION,
            }
        }
        other => other,
    };
    let signed = match kind {
        "sub" => -amount,
        _ => amount,
    };
    let sql = date_add_value(
        &value.sql,
        (unit, &format!("CAST({signed} AS BIGINT)")),
        value.data_type,
        log,
    )
    .map_err(refuse)?;
    let sql = match result_type == value.data_type {
        true => sql,
        false => cast(&sql, result_type, log).map_err(refuse)?,
    };
    Ok(Emitted::new(sql, result_type))
}

/// The amount and canonical unit of `INTERVAL '<n> <unit>'` or `INTERVAL '<n>' <unit>`.
pub(crate) fn interval_literal(
    node: &Value,
    site: Site<'_>,
) -> Result<Option<(i64, &'static str)>, Refusal> {
    let Some(payload) = node_kind(node).and_then(|(_, payload)| payload.as_object()) else {
        return Ok(None);
    };
    let (Some(this), true) = (
        payload.get("this"),
        payload
            .keys()
            .all(|key| INTERVAL_FIELDS.contains(&key.as_str())),
    ) else {
        return Ok(None);
    };
    let Some(Literal::Text(text)) = convert_expression(this, site)?.literal else {
        return Ok(None);
    };
    let written = match payload.get("unit").filter(|unit| !is_inert(unit)) {
        None => {
            let mut words = text.split(' ');
            match (words.next(), words.next(), words.next()) {
                (Some(digits), Some(unit), None) => Some((digits.to_owned(), unit.to_owned())),
                _ => None,
            }
        }
        Some(spec) => spec
            .get("unit")
            .and_then(Value::as_str)
            .filter(|_| spec.get("type").and_then(Value::as_str) == Some(INTERVAL_SIMPLE_UNIT))
            .map(|unit| (text.clone(), unit.to_owned())),
    };
    let Some((digits, written_unit)) = written else {
        return Ok(None);
    };
    let written_unit = written_unit.to_ascii_uppercase();
    if digits.is_empty()
        || digits.len() > INTERVAL_MAX_DIGITS
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Ok(None);
    }
    let amount = digits
        .bytes()
        .fold(0_i64, |amount, digit| amount * 10 + i64::from(digit - b'0'));
    Ok(INTERVAL_UNITS
        .iter()
        .find(|(spellings, _)| spellings.contains(&written_unit.as_str()))
        .map(|(_, unit)| (amount, *unit)))
}
