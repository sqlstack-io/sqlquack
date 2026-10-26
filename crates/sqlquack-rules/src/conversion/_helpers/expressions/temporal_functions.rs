//! DATE_PART, EXTRACT, DATE_TRUNC, DATEADD, DATEDIFF, CONVERT_TIMEZONE and TO_TIMESTAMP.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_duckdb::temporal::constants::DATE_FORMATS;
use sqlquack_duckdb::temporal::main::date_add_value::date_add_value;
use sqlquack_duckdb::temporal::main::date_diff_value::date_diff_value;
use sqlquack_duckdb::temporal::main::date_part_value::date_part_value;
use sqlquack_duckdb::temporal::main::date_trunc_value::date_trunc_value;
use sqlquack_duckdb::temporal::main::epoch_to_timestamp::epoch_to_timestamp;
use sqlquack_duckdb::temporal::main::formatted_text_to_date::formatted_text_to_date;
use sqlquack_duckdb::temporal::main::text_to_timestamp::text_to_timestamp;
use sqlquack_duckdb::temporal::main::utc_to_zone::utc_to_zone;
use sqlquack_duckdb::temporal::main::zone_checked::zone_checked;
use sqlquack_duckdb::temporal::main::zone_to_zone::zone_to_zone;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::temporal::temporal_cast;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::failure::text_failure;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    DATE_ARITHMETIC, DATE_FORMAT_RULE, DATE_PART_RULE, DATE_PARTS, DATE_TEXT_SPELLING,
    DATEDIFF_TYPE, DECIMAL_EPOCH_RULE, MAX_FRACTION_DIGITS_U8, MICROSECOND_DIGITS, SCALE_DIGITS,
    TEMPORAL_TEXT_FIELDS, TEMPORAL_TEXT_FORMAT, TEMPORAL_TEXT_SEPARATORS, TEXT_TO_TIMESTAMP_RULE,
    TIME_ZONE_CONVERSION, TIMESTAMP_RANGE, UNIT_ALIASES, UTC_ZONE,
};

/// The canonical Snowflake name of a unit written as a keyword, string or parsed field name.
fn unit_name(node: &Value) -> Option<String> {
    let text = match node {
        Value::String(text) => text.clone(),
        Value::Object(object) => match (
            object.get("var"),
            object.get("literal"),
            object.get("Custom"),
        ) {
            (Some(var), _, _) => var.get("this")?.as_str()?.to_owned(),
            (_, Some(literal), _) => literal.get("value")?.as_str()?.to_owned(),
            (_, _, Some(custom)) => custom.as_str()?.to_owned(),
            _ => return None,
        },
        _ => return None,
    };
    let upper = text.to_ascii_uppercase();
    Some(
        UNIT_ALIASES
            .iter()
            .find(|(alias, _)| *alias == upper)
            .map_or(upper, |(_, name)| (*name).to_owned()),
    )
}

/// A DATE or naive timestamp operand.
fn temporal_operand(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::Date | SnowflakeType::TimestampNtz { .. } => Ok(emitted),
        other => Err(outside_conditions(
            construct.clone(),
            format!("a {other} operand; only DATE and TIMESTAMP_NTZ are covered"),
        )),
    }
}

/// DATE_PART(unit, value), EXTRACT(unit FROM value) and YEAR(value)-style calls.
pub(crate) fn date_part(
    unit: &Value,
    value: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DATE_PART_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let unit = unit_name(unit).ok_or_else(|| refuse("an unreadable date part".to_owned()))?;
    let operand = temporal_operand(value, construct, site)?;
    let precision = DATE_PARTS
        .iter()
        .find(|(name, _)| *name == unit)
        .map(|(_, precision)| *precision)
        .ok_or_else(|| refuse(format!("DATE_PART {unit} has no proven conversion")))?;
    let log = site.context.log;
    let value = date_part_value(&operand.sql, &unit, operand.data_type, log).map_err(refuse)?;
    let data_type = SnowflakeType::Number {
        precision,
        scale: 0,
    };
    Ok(Emitted::new(
        cast(&value, data_type, log).map_err(refuse)?,
        data_type,
    ))
}

/// EXTRACT and the single-part functions such as YEAR(value).
pub(crate) fn extract(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "field"], &construct)?;
    let value = field(payload, "this", &construct)?;
    let unit = match payload.get("field") {
        Some(unit) => unit.clone(),
        None => Value::String(kind.to_owned()),
    };
    date_part(&unit, value, &construct, site)
}

/// DATE_TRUNC(unit, value), keeping the value's type.
pub(crate) fn date_trunc(
    unit: &Value,
    value: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DATE_ARITHMETIC);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let unit = unit_name(unit).ok_or_else(|| refuse("an unreadable unit".to_owned()))?;
    let operand = temporal_operand(value, construct, site)?;
    let sql = date_trunc_value(&operand.sql, &unit, operand.data_type, site.context.log)
        .map_err(refuse)?;
    Ok(Emitted::new(sql, operand.data_type))
}

/// DATEADD(unit, amount, value) with a NUMBER(p,0) amount, keeping the value's type.
pub(crate) fn date_add(
    (unit, amount, value): (&Value, &Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DATE_ARITHMETIC);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let unit = unit_name(unit).ok_or_else(|| refuse("an unreadable unit".to_owned()))?;
    let amount = convert_expression(amount, site)?;
    let SnowflakeType::Number { scale: 0, .. } = amount.data_type else {
        return Err(refuse(format!("a {} amount", amount.data_type)));
    };
    let operand = temporal_operand(value, construct, site)?;
    if let SnowflakeType::TimestampNtz { precision } = operand.data_type
        && precision > MICROSECOND_DIGITS
    {
        diverge(site.context, TIMESTAMP_RANGE, construct)?;
    }
    let sql = date_add_value(
        &operand.sql,
        (&unit, &format!("CAST({} AS BIGINT)", amount.sql)),
        operand.data_type,
        site.context.log,
    )
    .map_err(refuse)?;
    Ok(Emitted::new(sql, operand.data_type))
}

/// DATEDIFF(unit, start, end): unit boundaries crossed, between two DATEs or two naive timestamps.
pub(crate) fn date_diff(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("date_diff");
    let _rule = site.context.log.rule(DATE_ARITHMETIC);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "expression", "unit"], &construct)?;
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let unit = payload
        .get("unit")
        .and_then(unit_name)
        .ok_or_else(|| refuse("an unreadable unit".to_owned()))?;
    let end = temporal_operand(field(payload, "this", &construct)?, &construct, site)?;
    let start = temporal_operand(field(payload, "expression", &construct)?, &construct, site)?;
    let same_kind = matches!(
        (start.data_type, end.data_type),
        (SnowflakeType::Date, SnowflakeType::Date)
            | (
                SnowflakeType::TimestampNtz { .. },
                SnowflakeType::TimestampNtz { .. }
            )
    );
    if !same_kind {
        return Err(refuse(format!(
            "DATEDIFF between {} and {}",
            start.data_type, end.data_type
        )));
    }
    let log = site.context.log;
    let value = date_diff_value(
        (&start.sql, &end.sql),
        &unit,
        (start.data_type, end.data_type),
        log,
    )
    .map_err(refuse)?;
    Ok(Emitted::new(
        cast(&value, DATEDIFF_TYPE, log).map_err(refuse)?,
        DATEDIFF_TYPE,
    ))
}

/// CONVERT_TIMEZONE: three arguments from 'UTC' on a naive timestamp, or two on any timestamp.
pub(crate) fn convert_timezone(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TIME_ZONE_CONVERSION);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let log = site.context.log;
    match arguments {
        [source, target, value] => {
            let source = zone_literal(source, site)
                .ok_or_else(|| refuse("a source zone other than a literal"))?;
            let target = zone_literal(target, site)
                .ok_or_else(|| refuse("a target zone other than a literal"))?;
            let operand = convert_expression(value, site)?;
            let SnowflakeType::TimestampNtz { precision } = operand.data_type else {
                return Err(refuse(
                    "a three-argument conversion of a value other than TIMESTAMP_NTZ",
                ));
            };
            if precision > MICROSECOND_DIGITS {
                diverge(site.context, TIMESTAMP_RANGE, construct)?;
            }
            let sql = if source == UTC_ZONE {
                utc_to_zone(&operand.sql, &target, operand.data_type, log)
            } else {
                zone_to_zone(&operand.sql, [&source, &target], operand.data_type, log)
            };
            Ok(Emitted::new(sql, operand.data_type))
        }
        [target, value] => {
            let target = zone_literal(target, site)
                .ok_or_else(|| refuse("a target zone other than a literal"))?;
            let operand = convert_expression(value, site)?;
            let zoned = match operand.data_type {
                SnowflakeType::TimestampTz { precision }
                | SnowflakeType::TimestampLtz { precision } => {
                    let to = SnowflakeType::TimestampTz { precision };
                    (
                        cast(&operand.sql, to, log).map_err(|reason| refuse(&reason))?,
                        to,
                    )
                }
                SnowflakeType::TimestampNtz { precision } => {
                    let to = SnowflakeType::TimestampTz { precision };
                    let sql = temporal_cast(
                        &operand.sql,
                        (operand.data_type, to),
                        (site.context, construct),
                    )
                    .map_err(|reason| refuse(&reason))?;
                    (sql, to)
                }
                _ => return Err(refuse("CONVERT_TIMEZONE of a value other than a timestamp")),
            };
            Ok(Emitted {
                utc_offset: target.eq_ignore_ascii_case(UTC_ZONE),
                ..Emitted::new(zone_checked(&zoned.0, &target, log), zoned.1)
            })
        }
        _ => Err(refuse("CONVERT_TIMEZONE with these arguments")),
    }
}

/// TO_TIMESTAMP, TO_TIMESTAMP_NTZ and TO_DATE of text, and TO_TIMESTAMP of an integer NUMBER.
pub(crate) fn to_timestamp(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    if let ("TRY_TO_DATE" | "TO_DATE", [value, format]) = (name, arguments) {
        return formatted_date((value, format), name.starts_with("TRY_"), construct, site);
    }
    let _rule = site.context.log.rule(TEXT_TO_TIMESTAMP_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let log = site.context.log;
    let trying = name.starts_with("TRY_");
    let date_only = name.ends_with("_DATE");
    if trying && !date_only {
        return Err(refuse(format!(
            "{name}: Snowflake reads more spellings than the recorded ones, so a NULL here could \
             hide a value"
        )));
    }
    let [value, rest @ ..] = arguments else {
        return Err(refuse(format!("{name} without an argument")));
    };
    let operand = convert_expression(value, site)?;
    let scale = match rest {
        [] => None,
        [scale] => match convert_expression(scale, site)?.literal {
            Some(Literal::Number(digits)) => {
                small_integer(&digits).filter(|scale| *scale <= MAX_FRACTION_DIGITS_U8)
            }
            _ => None,
        }
        .map(Some)
        .ok_or_else(|| refuse(format!("{name} with a format or a non-literal scale")))?,
        _ => return Err(refuse(format!("{name} with these arguments"))),
    };
    match (operand.data_type, scale, date_only) {
        (SnowflakeType::Varchar | SnowflakeType::Null, None, _) => {
            diverge(site.context, DATE_TEXT_SPELLING, construct)?;
            if !date_only {
                diverge(site.context, TIMESTAMP_RANGE, construct)?;
            }
            let data_type = match date_only {
                true => SnowflakeType::Date,
                false => SnowflakeType::TimestampNtz { precision: 9 },
            };
            Ok(Emitted::new(
                text_to_timestamp(&operand.sql, (text_failure(trying), date_only), log),
                data_type,
            ))
        }
        (SnowflakeType::Number { scale: 0, .. }, scale, false) if !trying => {
            let scale = scale.unwrap_or(0);
            if scale > MICROSECOND_DIGITS {
                diverge(site.context, TIMESTAMP_RANGE, construct)?;
            }
            Ok(Emitted::new(
                epoch_to_timestamp(&operand.sql, scale, log),
                SnowflakeType::TimestampNtz { precision: scale },
            ))
        }
        (SnowflakeType::Number { scale, .. }, None, false)
            if !trying && scale <= MAX_FRACTION_DIGITS_U8 =>
        {
            let _decimal = log.rule(DECIMAL_EPOCH_RULE);
            if scale > MICROSECOND_DIGITS {
                diverge(site.context, TIMESTAMP_RANGE, construct)?;
            }
            let units = format!(
                "(CAST({} AS DECIMAL(38,{scale})) * {})",
                operand.sql,
                10_u64.pow(u32::from(scale))
            );
            Ok(Emitted::new(
                epoch_to_timestamp(&units, scale, log),
                SnowflakeType::TimestampNtz { precision: scale },
            ))
        }
        (other, _, _) => Err(refuse(format!("{name} of {other} with these arguments"))),
    }
}

/// TO_DATE or TRY_TO_DATE of text in a literal format SQLQuack reads exactly.
fn formatted_date(
    (value, format): (&Value, &Value),
    trying: bool,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DATE_FORMAT_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let Some(Literal::Text(written)) = convert_expression(format, site)?.literal else {
        return Err(refuse(
            "TO_DATE with a format that is not literal text".to_owned(),
        ));
    };
    let Some(known) = DATE_FORMATS
        .iter()
        .find(|known| known.snowflake.eq_ignore_ascii_case(&written))
    else {
        return Err(refuse(format!(
            "TO_DATE with format '{written}', which no recording shows"
        )));
    };
    let operand = convert_expression(value, site)?;
    if !matches!(
        operand.data_type,
        SnowflakeType::Varchar | SnowflakeType::Null
    ) {
        return Err(refuse(format!(
            "TO_DATE of {} with a format",
            operand.data_type
        )));
    }
    diverge(site.context, DATE_TEXT_SPELLING, construct)?;
    Ok(Emitted::new(
        formatted_text_to_date(&operand.sql, (known, trying), site.context.log),
        SnowflakeType::Date,
    ))
}

/// TO_VARCHAR or TO_CHAR of a DATE or TIMESTAMP_NTZ with a literal format of numeric fields.
pub(crate) fn temporal_text(
    (value, format): (&Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEMPORAL_TEXT_FORMAT);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let Some(Literal::Text(written)) = convert_expression(format, site)?.literal else {
        return Err(refuse("a format that is not literal text".to_owned()));
    };
    let operand = convert_expression(value, site)?;
    let with_time = match operand.data_type {
        SnowflakeType::Date => false,
        SnowflakeType::TimestampNtz { .. } => true,
        SnowflakeType::Null => true,
        other => return Err(refuse(format!("a format for {other}"))),
    };
    let pattern = strftime_pattern(&written, with_time)
        .ok_or_else(|| refuse(format!("the format '{written}', which no recording shows")))?;
    Ok(Emitted::new(
        format!(
            "strftime({}, {})",
            operand.sql,
            string_literal(&pattern, site.context.log)
        ),
        SnowflakeType::Varchar,
    ))
}

/// The strftime pattern of a format of known fields between separators, or None.
fn strftime_pattern(format: &str, with_time: bool) -> Option<String> {
    let upper = format.to_ascii_uppercase();
    let mut rest = upper.as_str();
    let mut pattern = String::new();
    while !rest.is_empty() {
        let field = TEMPORAL_TEXT_FIELDS
            .iter()
            .find(|(token, _, time)| rest.starts_with(token) && (with_time || !time));
        match field {
            Some((token, replacement, _)) => {
                pattern.push_str(replacement);
                rest = &rest[token.len()..];
            }
            None => {
                let separator = rest.chars().next()?;
                if !TEMPORAL_TEXT_SEPARATORS.contains(separator) {
                    return None;
                }
                pattern.push(separator);
                rest = &rest[separator.len_utf8()..];
            }
        }
    }
    Some(pattern)
}

/// The value of up to two decimal digits.
fn small_integer(digits: &str) -> Option<u8> {
    if digits.is_empty() || digits.len() > SCALE_DIGITS {
        return None;
    }
    digits.bytes().try_fold(0_u8, |value, byte| match byte {
        b'0'..=b'9' => value.checked_mul(10)?.checked_add(byte - b'0'),
        _ => None,
    })
}

/// A zone name given as a string literal in IANA form.
fn zone_literal(node: &Value, site: Site<'_>) -> Option<String> {
    let literal = match convert_expression(node, site) {
        Ok(emitted) => emitted.literal,
        Err(_) => None,
    };
    match literal {
        Some(Literal::Text(zone))
            if !zone.is_empty()
                && zone.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "/_+-".contains(character)
                }) =>
        {
            Some(zone)
        }
        _ => None,
    }
}
