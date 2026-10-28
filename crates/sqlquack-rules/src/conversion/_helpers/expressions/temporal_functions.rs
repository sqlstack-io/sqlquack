//! DATE_PART, EXTRACT, DATE_TRUNC, DATEADD, DATEDIFF, CONVERT_TIMEZONE and TO_TIMESTAMP.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::constants::NANOSECOND_DIGITS;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_duckdb::blocks::models::TextFailure;
use sqlquack_duckdb::temporal::constants::DATE_FORMATS;
use sqlquack_duckdb::temporal::main::date_add_amount::date_add_amount;
use sqlquack_duckdb::temporal::main::date_add_value::date_add_value;
use sqlquack_duckdb::temporal::main::date_diff_value::date_diff_value;
use sqlquack_duckdb::temporal::main::date_from_parts::date_from_parts;
use sqlquack_duckdb::temporal::main::date_part_value::date_part_value;
use sqlquack_duckdb::temporal::main::date_trunc_value::date_trunc_value;
use sqlquack_duckdb::temporal::main::epoch_to_timestamp::epoch_to_timestamp;
use sqlquack_duckdb::temporal::main::formatted_text_to_date::formatted_text_to_date;
use sqlquack_duckdb::temporal::main::formatted_text_to_timestamp::formatted_text_to_timestamp;
use sqlquack_duckdb::temporal::main::text_to_timestamp::text_to_timestamp;
use sqlquack_duckdb::temporal::main::text_to_zoned::text_to_zoned;
use sqlquack_duckdb::temporal::main::timestamp_format::timestamp_format;
use sqlquack_duckdb::temporal::main::timestamp_from_parts::timestamp_from_parts;
use sqlquack_duckdb::temporal::main::utc_to_zone::utc_to_zone;
use sqlquack_duckdb::temporal::main::utc_wall_time::utc_wall_time;
use sqlquack_duckdb::temporal::main::zone_checked::zone_checked;
use sqlquack_duckdb::temporal::main::zone_to_zone::zone_to_zone;
use sqlquack_duckdb::times::constants::NANOS_PER_SECOND;
use sqlquack_duckdb::times::main::text_to_time::text_to_time;
use sqlquack_duckdb::times::main::time_add_value::time_add_value;
use sqlquack_duckdb::times::main::time_diff_value::time_diff_value;
use sqlquack_duckdb::times::models::TimeFormat;
use sqlquack_snowflake::typing::constants::{DATEDIFF_PRECISIONS, MAX_PRECISION, TIME_DIFF_TYPE};
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::casts::cast_to;
use crate::conversion::_helpers::expressions::operators::is_iso_date;
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::temporal::temporal_cast;
use crate::conversion::_helpers::expressions::temporal::{proven_time_text, time_submicro};
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::failure::text_failure;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    CURRENT_TIME, CURRENT_TIME_RULE, DATE_ARITHMETIC, DATE_FORMAT_RULE, DATE_NAME, DATE_PART_RULE,
    DATE_PARTS, DATE_TEXT_SPELLING, DECIMAL_EPOCH_RULE, FROM_PARTS, MAX_FRACTION_DIGITS_U8,
    MICROSECOND_DIGITS, SCALE_DIGITS, TEMPORAL_TEXT_FIELDS, TEMPORAL_TEXT_FORMAT,
    TEMPORAL_TEXT_SEPARATORS, TEXT_TO_TIME_RULE, TEXT_TO_TIMESTAMP_RULE, TIME_ARITHMETIC,
    TIME_ZONE_CONVERSION, TIMESTAMP_FORMAT_RULE, TIMESTAMP_RANGE, TO_DATE_TEMPORAL, UNIT_ALIASES,
    UTC_ZONE, ZONE_COLUMNS, ZONED_PARTS,
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
    temporal_or_refused(emitted, construct)
}

/// A DATE, naive timestamp or TIME operand.
fn clock_operand(node: &Value, construct: &Construct, site: Site<'_>) -> Result<Emitted, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::Time { .. } => Ok(emitted),
        _ => temporal_or_refused(emitted, construct),
    }
}

fn temporal_or_refused(emitted: Emitted, construct: &Construct) -> Result<Emitted, Refusal> {
    match emitted.data_type {
        SnowflakeType::Date | SnowflakeType::TimestampNtz { .. } => Ok(emitted),
        other => Err(outside_conditions(
            construct.clone(),
            format!("a {other} operand; only DATE and TIMESTAMP_NTZ are covered"),
        )),
    }
}

/// A DATE_PART operand: an instant's epoch part, or a value whose clock fields are read.
enum ZonedPart {
    Epoch(String),
    Clock(Emitted),
}

/// A zoned operand's epoch part, or its UTC wall time where its offset is UTC; others as given.
fn zoned_part_operand(operand: Emitted, unit: &str, site: Site<'_>) -> Result<ZonedPart, String> {
    let log = site.context.log;
    let (precision, utc) = match operand.data_type {
        SnowflakeType::TimestampLtz { precision } => (precision, true),
        SnowflakeType::TimestampTz { precision } => (precision, operand.utc_offset),
        _ => return Ok(ZonedPart::Clock(operand)),
    };
    let _zoned = log.rule(ZONED_PARTS);
    match unit {
        "EPOCH_SECOND" | "EPOCH_MILLISECOND" => Ok(ZonedPart::Epoch(date_part_value(
            &operand.sql,
            unit,
            operand.data_type,
            log,
        )?)),
        _ if utc => Ok(ZonedPart::Clock(Emitted::new(
            utc_wall_time(&operand.sql, log),
            SnowflakeType::TimestampNtz {
                precision: precision.min(MICROSECOND_DIGITS),
            },
        ))),
        _ => Err(format!(
            "DATE_PART {unit} of {}, whose offset DuckDB does not keep",
            operand.data_type
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
    let operand =
        zoned_part_operand(convert_expression(value, site)?, &unit, site).map_err(refuse)?;
    let operand = match operand {
        ZonedPart::Epoch(sql) => {
            let data_type = SnowflakeType::Number {
                precision: DATE_PARTS
                    .iter()
                    .find(|(name, _)| *name == unit)
                    .map_or(MAX_PRECISION, |(_, precision)| *precision),
                scale: 0,
            };
            return Ok(Emitted::new(
                cast(&sql, data_type, site.context.log).map_err(refuse)?,
                data_type,
            ));
        }
        ZonedPart::Clock(emitted) => match emitted.data_type {
            SnowflakeType::Time { .. } => emitted,
            _ => temporal_or_refused(emitted, construct)?,
        },
    };
    let precision = DATE_PARTS
        .iter()
        .find(|(name, _)| *name == unit)
        .map(|(_, precision)| *precision)
        .ok_or_else(|| refuse(format!("DATE_PART {unit} has no proven conversion")))?;
    let log = site.context.log;
    let value = match (operand.data_type, unit.as_str()) {
        (SnowflakeType::Time { .. }, "HOUR" | "MINUTE" | "SECOND") => {
            let _time = log.rule(TIME_ARITHMETIC);
            format!("{}({})", unit.to_ascii_lowercase(), operand.sql)
        }
        (SnowflakeType::Time { .. }, _) => {
            return Err(refuse(format!(
                "DATE_PART {unit} of a TIME has no recording"
            )));
        }
        _ => date_part_value(&operand.sql, &unit, operand.data_type, log).map_err(refuse)?,
    };
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
    let operand = clock_operand(value, construct, site)?;
    let amount = Emitted::new(
        date_add_amount(&amount.sql, site.context.log),
        amount.data_type,
    );
    if let SnowflakeType::Time { .. } = operand.data_type {
        let _time = site.context.log.rule(TIME_ARITHMETIC);
        let sql =
            time_add_value(&operand.sql, (&unit, &amount.sql), site.context.log).map_err(refuse)?;
        return Ok(Emitted::new(sql, operand.data_type));
    }
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

/// DATEDIFF(unit, start, end) with a keyword unit.
pub(crate) fn date_diff(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("date_diff");
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "expression", "unit"], &construct)?;
    let unit = payload.get("unit").unwrap_or(&Value::Null);
    let end = field(payload, "this", &construct)?;
    let start = field(payload, "expression", &construct)?;
    date_diff_of((unit, start, end), &construct, site)
}

/// DATEDIFF('unit', start, end), a call with the unit as text.
pub(crate) fn date_diff_call(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let [unit, start, end] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "DATEDIFF takes three arguments".to_owned(),
        ));
    };
    date_diff_of((unit, start, end), construct, site)
}

/// Unit boundaries crossed between DATEs, naive timestamps, TIMEs or ISO date literals.
fn date_diff_of(
    (unit, start, end): (&Value, &Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DATE_ARITHMETIC);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let unit = unit_name(unit).ok_or_else(|| refuse("an unreadable unit".to_owned()))?;
    let start = date_diff_operand(start, construct, site)?;
    let end = date_diff_operand(end, construct, site)?;
    let log = site.context.log;
    if let (SnowflakeType::Time { .. }, SnowflakeType::Time { .. }) =
        (start.data_type, end.data_type)
    {
        let _time = log.rule(TIME_ARITHMETIC);
        let value = time_diff_value((&start.sql, &end.sql), &unit, log).map_err(refuse)?;
        return Ok(Emitted::new(
            cast(&value, TIME_DIFF_TYPE, log).map_err(refuse)?,
            TIME_DIFF_TYPE,
        ));
    }
    let precision = DATEDIFF_PRECISIONS
        .iter()
        .find(|(name, _)| *name == unit)
        .map(|(_, precision)| *precision)
        .ok_or_else(|| refuse(format!("DATEDIFF in {unit} has no recorded type")))?;
    let data_type = SnowflakeType::Number {
        precision,
        scale: 0,
    };
    let value = date_diff_value(
        (&start.sql, &end.sql),
        &unit,
        (start.data_type, end.data_type),
        log,
    )
    .map_err(refuse)?;
    Ok(Emitted::new(
        cast(&value, data_type, log).map_err(refuse)?,
        data_type,
    ))
}

/// A DATEDIFF operand: a DATE or naive timestamp, or an ISO date literal read as a DATE.
fn date_diff_operand(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let emitted = convert_expression(node, site)?;
    match (&emitted.data_type, &emitted.literal) {
        (
            SnowflakeType::Date | SnowflakeType::TimestampNtz { .. } | SnowflakeType::Time { .. },
            _,
        ) => Ok(emitted),
        (SnowflakeType::Varchar, Some(Literal::Text(text))) if is_iso_date(text) => {
            Ok(Emitted::new(
                cast(&emitted.sql, SnowflakeType::Date, site.context.log)
                    .map_err(|reason| outside_conditions(construct.clone(), reason))?,
                SnowflakeType::Date,
            ))
        }
        (other, _) => Err(outside_conditions(
            construct.clone(),
            format!(
                "a {other} operand; only DATE, TIMESTAMP_NTZ and ISO date literals are covered"
            ),
        )),
    }
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
            let source = zone_operand(source, construct, site)?;
            let target = zone_operand(target, construct, site)?;
            let operand = convert_expression(value, site)?;
            let SnowflakeType::TimestampNtz { precision } = operand.data_type else {
                return Err(refuse(
                    "a three-argument conversion of a value other than TIMESTAMP_NTZ",
                ));
            };
            if precision > MICROSECOND_DIGITS {
                diverge(site.context, TIMESTAMP_RANGE, construct)?;
            }
            let sql = match source.literal.as_deref() {
                Some(zone) if zone == UTC_ZONE => {
                    utc_to_zone(&operand.sql, &target.sql, operand.data_type, log)
                }
                _ => zone_to_zone(
                    &operand.sql,
                    [&source.sql, &target.sql],
                    operand.data_type,
                    log,
                ),
            };
            Ok(Emitted::new(sql, operand.data_type))
        }
        [target, value] => {
            let target = zone_operand(target, construct, site)?;
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
                utc_offset: target
                    .literal
                    .as_deref()
                    .is_some_and(|zone| zone.eq_ignore_ascii_case(UTC_ZONE)),
                ..Emitted::new(zone_checked(&zoned.0, &target.sql, log), zoned.1)
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
    if let ([value, format], true, false) = (arguments, trying, date_only) {
        return formatted_timestamp((value, format), construct, site);
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
            diverge(site.context, TIMESTAMP_RANGE, construct)?;
            Ok(Emitted::new(
                epoch_to_timestamp(&operand.sql, scale, log),
                SnowflakeType::TimestampNtz { precision: scale },
            ))
        }
        (SnowflakeType::Number { scale, .. }, None, false)
            if !trying && scale <= MAX_FRACTION_DIGITS_U8 =>
        {
            let _decimal = log.rule(DECIMAL_EPOCH_RULE);
            diverge(site.context, TIMESTAMP_RANGE, construct)?;
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
        (
            SnowflakeType::Date
            | SnowflakeType::TimestampNtz { .. }
            | SnowflakeType::TimestampLtz { .. }
            | SnowflakeType::TimestampTz { .. },
            None,
            true,
        ) => {
            let _temporal = log.rule(TO_DATE_TEMPORAL);
            let sql =
                cast_to(&operand, DATE_NAME, false, (site.context, construct)).map_err(refuse)?;
            Ok(Emitted::new(sql, SnowflakeType::Date))
        }
        (other, _, _) => Err(refuse(format!("{name} of {other} with these arguments"))),
    }
}

/// TRY_TO_TIMESTAMP_NTZ or TRY_TO_TIMESTAMP of text in a literal format of recorded elements.
fn formatted_timestamp(
    (value, format): (&Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TIMESTAMP_FORMAT_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let Some(Literal::Text(written)) = convert_expression(format, site)?.literal else {
        return Err(refuse(
            "a timestamp format that is not literal text".to_owned(),
        ));
    };
    let known = timestamp_format(&written).map_err(refuse)?;
    let operand = convert_expression(value, site)?;
    if !matches!(
        operand.data_type,
        SnowflakeType::Varchar | SnowflakeType::Null
    ) {
        return Err(refuse(format!(
            "a formatted timestamp of {}",
            operand.data_type
        )));
    }
    diverge(site.context, DATE_TEXT_SPELLING, construct)?;
    diverge(site.context, TIMESTAMP_RANGE, construct)?;
    Ok(Emitted::new(
        formatted_text_to_timestamp(&operand.sql, &known, site.context.log),
        SnowflakeType::TimestampNtz { precision: 9 },
    ))
}

/// TRY_TO_TIMESTAMP_TZ of text without a format: the instant it names, at the session's UTC.
pub(crate) fn to_timestamp_tz(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_TO_TIMESTAMP_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value] = arguments else {
        return Err(refuse(
            "TRY_TO_TIMESTAMP_TZ with a format or a scale".to_owned(),
        ));
    };
    let operand = convert_expression(value, site)?;
    if !matches!(
        operand.data_type,
        SnowflakeType::Varchar | SnowflakeType::Null
    ) {
        return Err(refuse(format!(
            "TRY_TO_TIMESTAMP_TZ of {}",
            operand.data_type
        )));
    }
    diverge(site.context, DATE_TEXT_SPELLING, construct)?;
    diverge(site.context, TIMESTAMP_RANGE, construct)?;
    let wall = text_to_zoned(&operand.sql, TextFailure::Null, site.context.log);
    let sql = temporal_cast(
        &wall,
        (
            SnowflakeType::TimestampNtz { precision: 9 },
            SnowflakeType::TimestampTz { precision: 9 },
        ),
        (site.context, construct),
    )
    .map_err(refuse)?;
    Ok(Emitted::new(
        sql,
        SnowflakeType::TimestampTz { precision: 9 },
    ))
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

/// A time zone argument: its SQL, and its name when a literal.
struct Zone {
    sql: String,
    literal: Option<String>,
}

/// A literal zone name, or a VARCHAR expression naming one per row.
fn zone_operand(node: &Value, construct: &Construct, site: Site<'_>) -> Result<Zone, Refusal> {
    if let Some(zone) = zone_literal(node, site) {
        return Ok(Zone {
            sql: string_literal(&zone, site.context.log),
            literal: Some(zone),
        });
    }
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::Varchar | SnowflakeType::Null => {
            let _column = site.context.log.rule(ZONE_COLUMNS);
            Ok(Zone {
                sql: emitted.sql,
                literal: None,
            })
        }
        other => Err(outside_conditions(
            construct.clone(),
            format!("a time zone of type {other}"),
        )),
    }
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

/// CURRENT_DATE() and CURRENT_TIMESTAMP() without a precision, in the session's time zone.
pub(crate) fn current_time(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(CURRENT_TIME_RULE);
    if !arguments.is_empty() {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} with a precision"),
        ));
    }
    diverge(site.context, CURRENT_TIME, construct)?;
    Ok(match name {
        "CURRENT_DATE" => Emitted::new("current_date".to_owned(), SnowflakeType::Date),
        _ => Emitted::new(
            "current_timestamp".to_owned(),
            SnowflakeType::TimestampLtz { precision: 9 },
        ),
    })
}

/// TO_TIME and TRY_TO_TIME of text, without a format or with `HH24:MI` or `HH24MI`.
pub(crate) fn to_time(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_TO_TIME_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let (text, format) = match arguments {
        [text] => (text, TimeFormat::Auto),
        [text, format] => match convert_expression(format, site)?.literal {
            Some(Literal::Text(format)) => match format.as_str() {
                "HH24:MI" => (text, TimeFormat::HourMinute),
                "HH24MI" => (text, TimeFormat::CompactHourMinute),
                other => return Err(refuse(format!("{name} with the format '{other}'"))),
            },
            _ => return Err(refuse(format!("{name} with a format other than a literal"))),
        },
        _ => return Err(refuse(format!("{name} with these arguments"))),
    };
    let operand = convert_expression(text, site)?;
    if !matches!(
        operand.data_type,
        SnowflakeType::Varchar | SnowflakeType::Null
    ) {
        return Err(refuse(format!("{name} of {}", operand.data_type)));
    }
    if format == TimeFormat::Auto {
        time_submicro(
            (operand.literal.as_ref(), NANOSECOND_DIGITS),
            construct,
            site.context,
        )?;
    }
    let proven = format == TimeFormat::Auto
        && matches!(&operand.literal, Some(Literal::Text(text)) if proven_time_text(text));
    if !proven {
        diverge(site.context, DATE_TEXT_SPELLING, construct)?;
    }
    let failure = match name.starts_with("TRY_") {
        true => TextFailure::Null,
        false => TextFailure::Text,
    };
    let data_type = SnowflakeType::Time {
        precision: NANOSECOND_DIGITS,
    };
    Ok(Emitted::new(
        text_to_time(
            &operand.sql,
            (format, NANOSECOND_DIGITS),
            failure,
            site.context.log,
        ),
        data_type,
    ))
}

/// DATE_FROM_PARTS(year, month, day) of rounded numbers, with overflowing parts carried.
pub(crate) fn date_from_parts_call(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(FROM_PARTS);
    let [year, month, day] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "DATE_FROM_PARTS with these arguments".to_owned(),
        ));
    };
    let year = whole_part(year, construct, site)?;
    let month = whole_part(month, construct, site)?;
    let day = whole_part(day, construct, site)?;
    Ok(Emitted::new(
        date_from_parts((&year, &month, &day), site.context.log),
        SnowflakeType::Date,
    ))
}

/// TIMESTAMP_NTZ_FROM_PARTS(date, time), or of six numbers with overflowing parts carried.
pub(crate) fn timestamp_from_parts_call(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(FROM_PARTS);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let log = site.context.log;
    let data_type = SnowflakeType::TimestampNtz {
        precision: NANOSECOND_DIGITS,
    };
    diverge(site.context, TIMESTAMP_RANGE, construct)?;
    let sql = match arguments {
        [date, time] => {
            let date = convert_expression(date, site)?;
            let time = convert_expression(time, site)?;
            if !matches!(date.data_type, SnowflakeType::Date | SnowflakeType::Null)
                || !matches!(
                    time.data_type,
                    SnowflakeType::Time { .. } | SnowflakeType::Null
                )
            {
                return Err(refuse(format!(
                    "TIMESTAMP_NTZ_FROM_PARTS of {} and {}",
                    date.data_type, time.data_type
                )));
            }
            timestamp_from_parts(&date.sql, &format!("epoch_ns({})", time.sql), log)
        }
        [year, month, day, hour, minute, second] => {
            let date = date_from_parts(
                (
                    &whole_part(year, construct, site)?,
                    &whole_part(month, construct, site)?,
                    &whole_part(day, construct, site)?,
                ),
                log,
            );
            let nanos = format!(
                "({} * 3600 + {} * 60 + {}) * {NANOS_PER_SECOND}",
                whole_part(hour, construct, site)?,
                whole_part(minute, construct, site)?,
                whole_part(second, construct, site)?
            );
            timestamp_from_parts(&date, &nanos, log)
        }
        _ => {
            return Err(refuse(
                "TIMESTAMP_NTZ_FROM_PARTS with these arguments".to_owned(),
            ));
        }
    };
    Ok(Emitted::new(sql, data_type))
}

/// A NUMBER or FLOAT part as a BIGINT, rounded half away from zero.
fn whole_part(node: &Value, construct: &Construct, site: Site<'_>) -> Result<String, Refusal> {
    let part = convert_expression(node, site)?;
    match part.data_type {
        SnowflakeType::Number { scale: 0, .. } | SnowflakeType::Null => {
            Ok(format!("CAST({} AS BIGINT)", part.sql))
        }
        SnowflakeType::Number { .. } | SnowflakeType::Float => {
            Ok(format!("CAST(round({}) AS BIGINT)", part.sql))
        }
        other => Err(outside_conditions(
            construct.clone(),
            format!("a {other} date or time part"),
        )),
    }
}
