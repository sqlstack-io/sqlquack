//! DATE_PART, EXTRACT, DATE_TRUNC, DATEADD, DATEDIFF, CONVERT_TIMEZONE and TO_TIMESTAMP.

use serde_json::Value;
use sqlquack_blocks::blocks::constants::NANOSECOND_DIGITS;
use sqlquack_blocks::blocks::main::cast::cast;
use sqlquack_blocks::blocks::main::exact_decimal_divide::exact_decimal_divide;
use sqlquack_blocks::blocks::main::string_literal::string_literal;
use sqlquack_blocks::blocks::models::TextFailure;
use sqlquack_blocks::temporal::constants::DATE_FORMATS;
use sqlquack_blocks::temporal::main::date_add_amount::date_add_amount;
use sqlquack_blocks::temporal::main::date_add_value::date_add_value;
use sqlquack_blocks::temporal::main::date_diff_value::date_diff_value;
use sqlquack_blocks::temporal::main::date_from_parts::date_from_parts;
use sqlquack_blocks::temporal::main::date_part_value::date_part_value;
use sqlquack_blocks::temporal::main::date_trunc_value::date_trunc_value;
use sqlquack_blocks::temporal::main::epoch_to_timestamp::epoch_to_timestamp;
use sqlquack_blocks::temporal::main::formatted_text_to_date::formatted_text_to_date;
use sqlquack_blocks::temporal::main::formatted_text_to_timestamp::formatted_text_to_timestamp;
use sqlquack_blocks::temporal::main::text_to_timestamp::text_to_timestamp;
use sqlquack_blocks::temporal::main::text_to_zoned::text_to_zoned;
use sqlquack_blocks::temporal::main::timestamp_format::timestamp_format;
use sqlquack_blocks::temporal::main::timestamp_from_parts::timestamp_from_parts;
use sqlquack_blocks::temporal::main::truncate_timestamp_to_date::truncate_timestamp_to_date;
use sqlquack_blocks::temporal::main::utc_to_zone::utc_to_zone;
use sqlquack_blocks::temporal::main::utc_wall_time::utc_wall_time;
use sqlquack_blocks::temporal::main::zone_checked::zone_checked;
use sqlquack_blocks::temporal::main::zone_to_zone::zone_to_zone;
use sqlquack_blocks::temporal::models::TimestampField;
use sqlquack_blocks::times::constants::NANOS_PER_SECOND;
use sqlquack_blocks::times::main::text_to_time::text_to_time;
use sqlquack_blocks::times::main::time_add_value::time_add_value;
use sqlquack_blocks::times::main::time_diff_value::time_diff_value;
use sqlquack_blocks::times::main::timestamp_time::timestamp_time;
use sqlquack_blocks::times::models::TimeFormat;
use sqlquack_blocks::zones::main::utc_zoned::utc_zoned;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::constants::{
    DATEDIFF_PRECISIONS, MAX_PRECISION, TIME_DIFF_MICROSECONDS_TYPE, TIME_DIFF_TYPE, UNIT_ALIASES,
};
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::casts::cast_to;
use crate::conversion::_helpers::expressions::operators::is_iso_date;
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::temporal::temporal_cast;
use crate::conversion::_helpers::expressions::temporal::{
    proven_time_text, time_submicro, zoned_local,
};
use crate::conversion::_helpers::support::declared::rely_on_declared;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::failure::text_failure;
use crate::conversion::_helpers::support::scope::Context;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, node_construct, payload_object, require_understood,
};
use crate::conversion::constants::{
    ADD_MONTHS_RULE, CURRENT_TIME_RULE, DATE_ARITHMETIC, DATE_FORMAT_RULE, DATE_NAME,
    DATE_PART_RULE, DATE_PARTS, DAY_NAME_RULE, DAY_UNIT, DECIMAL_EPOCH_RULE,
    FRACTIONAL_DATE_AMOUNT, FROM_PARTS, LAST_DAY_RULE, MAX_FRACTION_DIGITS_U8, MICROSECOND_DIGITS,
    MONTH_UNIT, MONTHS_BETWEEN_MONTH_NANOSECONDS, MONTHS_BETWEEN_RULE, MONTHS_BETWEEN_TYPE,
    NULL_BRANCHES, NUMBER_TEXT_FORMAT, OFFSET_FREE_UNITS, RECORDED_TIME_ZONES, SCALE_DIGITS,
    TEMPORAL_TEXT_FORMAT, TEXT_TO_TIME_RULE, TEXT_TO_TIMESTAMP_RULE, TIME_ARITHMETIC,
    TIME_ZONE_CONVERSION, TIMESTAMP_FORMAT_RULE, TIMESTAMP_NTZ_NAME, TIMESTAMP_TIME_RULE,
    TO_DATE_TEMPORAL, TO_TIMESTAMP_TEMPORAL, UTC_OFFSET_ARITHMETIC, UTC_ZONE, ZONE_COLUMNS,
    ZONED_PARTS,
};
use crate::text_formats::main::clock_format::clock_format;
use crate::text_formats::main::strftime_pattern::strftime_pattern;
use sqlquack_blocks::raises::main::raised::raised;
use sqlquack_blocks::temporal::constants::{DUCKDB_CURRENT_DATE, DUCKDB_CURRENT_TIMESTAMP};
use sqlquack_policy::policy::constants::{
    CURRENT_TIME, DATE_TEXT_SPELLING, SESSION_TIMEZONE_UTC, TIME_ZONE_SPELLING, TIMESTAMP_RANGE,
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

/// A DATE or naive timestamp operand, or a UTC-offset one as its UTC wall time and zoned type.
fn temporal_operand(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<(Emitted, Option<SnowflakeType>), Refusal> {
    let emitted = convert_expression(node, site)?;
    temporal_or_refused(emitted, construct, site.context)
}

fn temporal_or_refused(
    emitted: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(Emitted, Option<SnowflakeType>), Refusal> {
    let log = context.log;
    match emitted.data_type {
        SnowflakeType::Date | SnowflakeType::TimestampNtz { .. } => Ok((emitted, None)),
        zoned @ SnowflakeType::TimestampTz { precision } if emitted.utc_offset => {
            let _zoned = log.rule(UTC_OFFSET_ARITHMETIC);
            rely_on_declared(&emitted.declared, construct, context)?;
            Ok((
                Emitted {
                    declared: emitted.declared.clone(),
                    ..Emitted::new(
                        utc_wall_time(&emitted.sql, log),
                        SnowflakeType::TimestampNtz {
                            precision: precision.min(MICROSECOND_DIGITS),
                        },
                    )
                },
                Some(zoned),
            ))
        }
        zoned @ SnowflakeType::TimestampTz { .. } => Err(outside_conditions(
            construct.clone(),
            zoned_local(&construct.name, zoned),
        )),
        other => Err(outside_conditions(
            construct.clone(),
            format!("a {other} operand; only DATE and TIMESTAMP_NTZ are covered"),
        )),
    }
}

/// A result computed on a UTC wall time read back as the zoned value at +00:00, or as computed.
fn rezoned(result: Emitted, zoned: Option<SnowflakeType>, log: &ProvenanceLog) -> Emitted {
    match zoned {
        Some(zoned) => Emitted {
            utc_offset: true,
            declared: result.declared,
            ..Emitted::new(utc_zoned(&result.sql, log), zoned)
        },
        None => result,
    }
}

/// A DATE_PART operand: an instant's epoch part, or a value whose clock fields are read.
enum ZonedPart {
    Epoch(String),
    Clock(Emitted),
}

/// A zoned operand's epoch part, or its UTC wall time where its offset is UTC; others as given.
fn zoned_part_operand(
    operand: Emitted,
    unit: &str,
    (construct, site): (&Construct, Site<'_>),
) -> Result<ZonedPart, String> {
    let log = site.context.log;
    let (precision, utc) = match operand.data_type {
        SnowflakeType::TimestampLtz { precision } => (precision, true),
        SnowflakeType::TimestampTz { precision } => (precision, operand.utc_offset),
        _ => return Ok(ZonedPart::Clock(operand)),
    };
    if utc && !matches!(unit, "EPOCH_SECOND" | "EPOCH_MILLISECOND") {
        rely_on_declared(&operand.declared, construct, site.context)
            .map_err(|refusal| refusal.reason)?;
    }
    let _zoned = log.rule(ZONED_PARTS);
    match unit {
        "EPOCH_SECOND" | "EPOCH_MILLISECOND" => Ok(ZonedPart::Epoch(date_part_value(
            &operand.sql,
            unit,
            operand.data_type,
            log,
        )?)),
        _ if utc => {
            if let SnowflakeType::TimestampLtz { .. } = operand.data_type {
                diverge(site.context, SESSION_TIMEZONE_UTC, construct)
                    .map_err(|refusal| refusal.reason)?;
            }
            Ok(ZonedPart::Clock(Emitted::new(
                utc_wall_time(&operand.sql, log),
                SnowflakeType::TimestampNtz {
                    precision: precision.min(MICROSECOND_DIGITS),
                },
            )))
        }
        _ => Err(zoned_local(&format!("DATE_PART {unit}"), operand.data_type)),
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
    let value = convert_expression(value, site)?;
    if value.data_type == SnowflakeType::Null {
        let _null = site.context.log.rule(NULL_BRANCHES);
        let null = cast("NULL", SnowflakeType::Varchar, site.context.log).map_err(refuse)?;
        return Ok(Emitted::new(null, SnowflakeType::Null));
    }
    let operand = zoned_part_operand(value, &unit, (construct, site)).map_err(refuse)?;
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
            _ => temporal_or_refused(emitted, construct, site.context)?.0,
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
    let (operand, zoned) = temporal_operand(value, construct, site)?;
    let sql = date_trunc_value(&operand.sql, &unit, operand.data_type, site.context.log)
        .map_err(refuse)?;
    Ok(rezoned(
        Emitted {
            declared: operand.declared,
            ..Emitted::new(sql, operand.data_type)
        },
        zoned,
        site.context.log,
    ))
}

/// DAYNAME of a DATE or TIMESTAMP_NTZ: the day's three-letter English name.
pub(crate) fn day_name(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DAY_NAME_RULE);
    let [value] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "DAYNAME takes one argument".to_owned(),
        ));
    };
    let (operand, _) =
        temporal_or_refused(convert_expression(value, site)?, construct, site.context)?;
    Ok(Emitted::new(
        format!("strftime({}, '%a')", operand.sql),
        SnowflakeType::Varchar,
    ))
}

/// LAST_DAY of a DATE or TIMESTAMP_NTZ at the default MONTH unit: the month's last DATE.
pub(crate) fn last_day(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(LAST_DAY_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let value = match arguments {
        [value] => value,
        [value, unit] if unit_name(unit).as_deref() == Some("MONTH") => value,
        _ => return Err(refuse("LAST_DAY of a unit other than MONTH".to_owned())),
    };
    let (operand, _) =
        temporal_or_refused(convert_expression(value, site)?, construct, site.context)?;
    let date = match operand.data_type {
        SnowflakeType::TimestampNtz { precision } => {
            truncate_timestamp_to_date(&operand.sql, precision, site.context.log)
        }
        _ => operand.sql.clone(),
    };
    Ok(Emitted {
        declared: operand.declared,
        ..Emitted::new(format!("last_day({date})"), SnowflakeType::Date)
    })
}

/// DATEADD(unit, amount, value) with a NUMBER(p,0) amount; a TIMESTAMP_NTZ widens to nanoseconds.
pub(crate) fn date_add(
    (unit, amount, value): (&Value, &Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DATE_ARITHMETIC);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let unit = unit_name(unit).ok_or_else(|| refuse("an unreadable unit".to_owned()))?;
    let amount = convert_expression(amount, site)?;
    let operand = convert_expression(value, site)?;
    shift((&unit, amount), operand, false, (construct, site))
}

/// ADD_MONTHS(value, months): DATEADD of the rounded months, a month's last day kept the last day.
pub(crate) fn add_months(
    parts: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(ADD_MONTHS_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value, amount] = parts else {
        return Err(refuse(format!("ADD_MONTHS of {} arguments", parts.len())));
    };
    let operand = convert_expression(value, site)?;
    let amount = convert_expression(amount, site)?;
    match (operand.data_type, amount.data_type) {
        (
            SnowflakeType::Date
            | SnowflakeType::TimestampNtz { .. }
            | SnowflakeType::TimestampTz { .. },
            SnowflakeType::Number { .. } | SnowflakeType::Null,
        ) => shift((MONTH_UNIT, amount), operand, true, (construct, site)),
        (operand, amount) => Err(refuse(format!(
            "ADD_MONTHS of {operand} and {amount}, which no recording shows"
        ))),
    }
}

/// MONTHS_BETWEEN(a, b) of DATEs and naive timestamps, whole or over a 31-day month, as recorded.
pub(crate) fn months_between(
    parts: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(MONTHS_BETWEEN_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [later, earlier] = parts else {
        return Err(refuse(format!(
            "MONTHS_BETWEEN of {} arguments",
            parts.len()
        )));
    };
    let (later, earlier) = (
        convert_expression(later, site)?,
        convert_expression(earlier, site)?,
    );
    let read = |value: &Emitted| match value.data_type {
        SnowflakeType::Date => Ok((value.sql.clone(), "0".to_owned())),
        SnowflakeType::TimestampNtz { precision } => {
            let day = truncate_timestamp_to_date(&value.sql, precision, log);
            let clock = format!(
                "(epoch_ns({}) - epoch_ns(CAST({day} AS TIMESTAMP)))",
                value.sql
            );
            Ok((day, clock))
        }
        other => Err(refuse(format!(
            "MONTHS_BETWEEN of a {other}, which no recording shows"
        ))),
    };
    let ((a, a_clock), (b, b_clock)) = (read(&later)?, read(&earlier)?);
    let whole = format!("((year({a}) - year({b})) * 12 + month({a}) - month({b}))");
    let whole_only =
        format!("(day({a}) = day({b}) OR (last_day({a}) = {a} AND last_day({b}) = {b}))");
    let month = MONTHS_BETWEEN_MONTH_NANOSECONDS;
    let numerator = format!(
        "(CAST({whole} AS HUGEINT) * {month} + CAST(day({a}) - day({b}) AS HUGEINT) * {} \
         + CAST({a_clock} AS HUGEINT) - CAST({b_clock} AS HUGEINT))",
        month / 31
    );
    let (divided, _) = exact_decimal_divide(
        (&numerator, MAX_PRECISION, 0),
        (&month.to_string(), MAX_PRECISION, 0),
        (MONTHS_BETWEEN_TYPE, false),
        log,
    )
    .map_err(refuse)?;
    let whole = cast(whole.as_str(), MONTHS_BETWEEN_TYPE, log).map_err(refuse)?;
    Ok(Emitted::new(
        format!("(CASE WHEN {whole_only} THEN {whole} ELSE {divided} END)"),
        MONTHS_BETWEEN_TYPE,
    ))
}

/// `operand` moved by `amount` of `unit`; with `month_end`, a month's last day stays a last day.
fn shift(
    (unit, mut amount): (&str, Emitted),
    operand: Emitted,
    month_end: bool,
    (construct, site): (&Construct, Site<'_>),
) -> Result<Emitted, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    match amount.data_type {
        SnowflakeType::Number { scale: 0, .. } => {}
        SnowflakeType::Number { .. } | SnowflakeType::Float => {
            let _rule = site.context.log.rule(FRACTIONAL_DATE_AMOUNT);
            amount = Emitted::new(
                format!("round({})", amount.sql),
                SnowflakeType::Number {
                    precision: 38,
                    scale: 0,
                },
            );
        }
        SnowflakeType::Null if month_end => {}
        other => return Err(refuse(format!("a {other} amount"))),
    }
    let operand = match operand.data_type {
        SnowflakeType::Varchar => {
            let ntz = SnowflakeType::TimestampNtz { precision: 9 };
            let sql = cast_to(&operand, &ntz.to_string(), false, (site.context, construct))
                .map_err(refuse)?;
            Emitted::new(sql, ntz)
        }
        _ => operand,
    };
    let (operand, zoned) = match operand.data_type {
        SnowflakeType::Time { .. } => (operand, None),
        local @ SnowflakeType::TimestampLtz { precision } => {
            let _zoned = site.context.log.rule(UTC_OFFSET_ARITHMETIC);
            diverge(site.context, SESSION_TIMEZONE_UTC, construct)?;
            let wall = SnowflakeType::TimestampNtz {
                precision: precision.min(MICROSECOND_DIGITS),
            };
            let sql = utc_wall_time(&operand.sql, site.context.log);
            (Emitted::new(sql, wall), Some(local))
        }
        _ => temporal_or_refused(operand, construct, site.context)?,
    };
    let amount = Emitted::new(
        date_add_amount(&amount.sql, site.context.log),
        amount.data_type,
    );
    if let SnowflakeType::Time { .. } = operand.data_type {
        let _time = site.context.log.rule(TIME_ARITHMETIC);
        let sql =
            time_add_value(&operand.sql, (unit, &amount.sql), site.context.log).map_err(refuse)?;
        return Ok(Emitted::new(sql, operand.data_type));
    }
    if let SnowflakeType::TimestampNtz { precision } = operand.data_type
        && precision > MICROSECOND_DIGITS
    {
        diverge(site.context, TIMESTAMP_RANGE, construct)?;
    }
    let sql = date_add_value(
        &operand.sql,
        (unit, &format!("CAST({} AS BIGINT)", amount.sql)),
        operand.data_type,
        site.context.log,
    )
    .map_err(refuse)?;
    let sql = match month_end {
        true => kept_month_end((&operand.sql, sql), operand.data_type, site.context.log)
            .map_err(refuse)?,
        false => sql,
    };
    let (sql, data_type) = match operand.data_type {
        SnowflakeType::TimestampNtz { precision } if precision < MAX_FRACTION_DIGITS_U8 => {
            let nanoseconds = SnowflakeType::TimestampNtz {
                precision: MAX_FRACTION_DIGITS_U8,
            };
            let widened = temporal_cast(
                &sql,
                (operand.data_type, nanoseconds),
                (site.context, construct),
            )
            .map_err(refuse)?;
            (widened, nanoseconds)
        }
        other => (sql, other),
    };
    Ok(rezoned(
        Emitted {
            declared: operand.declared,
            ..Emitted::new(sql, data_type)
        },
        zoned,
        site.context.log,
    ))
}

/// `shifted` moved on to its month's last day where `original`, of `data_type`, is a month's last day.
fn kept_month_end(
    (original, shifted): (&str, String),
    data_type: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    let day = |sql: &str| match data_type {
        SnowflakeType::TimestampNtz { precision } => {
            truncate_timestamp_to_date(sql, precision, log)
        }
        _ => sql.to_owned(),
    };
    let (before, after) = (day(original), day(&shifted));
    let days = format!(
        "CASE WHEN last_day({before}) = {before} \
         THEN datediff('day', {after}, last_day({after})) ELSE 0 END"
    );
    date_add_value(&shifted, (DAY_UNIT, &days), data_type, log)
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
    let zoned = |emitted: &Emitted| matches!(emitted.data_type, SnowflakeType::TimestampTz { .. });
    let (start, end) = match (start.utc_offset, end.utc_offset) {
        _ if zoned(&start) && zoned(&end) && OFFSET_FREE_UNITS.contains(&unit.as_str()) => {
            (utc_wall(start, log), utc_wall(end, log))
        }
        (false, false) if zoned(&start) || zoned(&end) => {
            let zoned = match zoned(&start) {
                true => start.data_type,
                false => end.data_type,
            };
            return Err(refuse(zoned_local(&format!("DATEDIFF in {unit}"), zoned)));
        }
        (false, false) => (start, end),
        (true, true) => {
            let _zoned = log.rule(UTC_OFFSET_ARITHMETIC);
            rely_on_declared(&start.declared, construct, site.context)?;
            rely_on_declared(&end.declared, construct, site.context)?;
            (utc_wall(start, log), utc_wall(end, log))
        }
        (true, false) if zoned(&end) => {
            return Err(refuse(zoned_local(
                &format!("DATEDIFF in {unit}"),
                end.data_type,
            )));
        }
        (false, true) if zoned(&start) => {
            return Err(refuse(zoned_local(
                &format!("DATEDIFF in {unit}"),
                start.data_type,
            )));
        }
        _ => {
            return Err(refuse(
                "a UTC-offset TIMESTAMP_TZ against a value of another kind".to_owned(),
            ));
        }
    };
    if let (SnowflakeType::Time { .. }, SnowflakeType::Time { .. }) =
        (start.data_type, end.data_type)
    {
        let _time = log.rule(TIME_ARITHMETIC);
        let value = time_diff_value((&start.sql, &end.sql), &unit, log).map_err(refuse)?;
        let data_type = match unit.as_str() {
            "MICROSECOND" => TIME_DIFF_MICROSECONDS_TYPE,
            _ => TIME_DIFF_TYPE,
        };
        return Ok(Emitted::new(
            cast(&value, data_type, log).map_err(refuse)?,
            data_type,
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

/// A DATEDIFF operand: a DATE, naive timestamp or local one at its UTC wall time, or an ISO date.
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
        (SnowflakeType::TimestampTz { .. }, _) => Ok(emitted),
        (SnowflakeType::TimestampLtz { precision }, _) => {
            let _zoned = site.context.log.rule(UTC_OFFSET_ARITHMETIC);
            diverge(site.context, SESSION_TIMEZONE_UTC, construct)?;
            Ok(Emitted::new(
                utc_wall_time(&emitted.sql, site.context.log),
                SnowflakeType::TimestampNtz {
                    precision: (*precision).min(MICROSECOND_DIGITS),
                },
            ))
        }
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
                "a {other} operand; only DATE, TIMESTAMP_NTZ, TIMESTAMP_LTZ and ISO date literals are covered"
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
                    if let SnowflakeType::TimestampLtz { .. } = operand.data_type {
                        diverge(site.context, SESSION_TIMEZONE_UTC, construct)?;
                    }
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
                    .is_some_and(|zone| zone == UTC_ZONE),
                zone: Some(target.sql.clone()),
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
    if let ([value, format], false) = (arguments, date_only)
        && (trying
            || matches!(
                convert_expression(format, site)?.literal,
                Some(Literal::Text(_))
            ))
    {
        return formatted_timestamp((value, format), trying, construct, site);
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
        (SnowflakeType::TimestampNtz { precision: 9 }, None, false) if !trying => {
            let _temporal = log.rule(TO_TIMESTAMP_TEMPORAL);
            Ok(operand)
        }
        (SnowflakeType::Date | SnowflakeType::TimestampNtz { .. }, None, false) if !trying => {
            let _temporal = log.rule(TO_TIMESTAMP_TEMPORAL);
            let sql = cast_to(
                &operand,
                TIMESTAMP_NTZ_NAME,
                false,
                (site.context, construct),
            )
            .map_err(refuse)?;
            Ok(Emitted::new(
                sql,
                SnowflakeType::TimestampNtz { precision: 9 },
            ))
        }
        (other, _, _) => Err(refuse(format!("{name} of {other} with these arguments"))),
    }
}

/// TO_TIMESTAMP_NTZ, TO_TIMESTAMP or their TRY_ forms of text in a literal format of recorded elements.
fn formatted_timestamp(
    (value, format): (&Value, &Value),
    trying: bool,
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
    if !trying
        && known
            .fields
            .iter()
            .any(|field| matches!(field, TimestampField::ShortYear | TimestampField::MonthName))
    {
        return Err(refuse(format!(
            "the format '{written}', recorded only under the TRY_ forms"
        )));
    }
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
        formatted_text_to_timestamp(&operand.sql, (&known, trying), site.context.log),
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
    let Some(known) = DATE_FORMATS.iter().find(|known| match known.named_month {
        true => known.snowflake == written,
        false => known.snowflake.eq_ignore_ascii_case(&written),
    }) else {
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
    if known.named_month && !trying {
        return Err(refuse(format!(
            "TO_DATE with format '{written}', recorded only under TRY_TO_DATE"
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
    if let SnowflakeType::Time { precision } = operand.data_type {
        if !clock_format(&written) {
            return Err(refuse(format!(
                "the TIME format '{written}', which no recording shows"
            )));
        }
        let pattern = strftime_pattern(&written, Some(precision))
            .ok_or_else(|| refuse(format!("the format '{written}', which no recording shows")))?;
        let micros = match precision > MICROSECOND_DIGITS {
            true => format!(
                "(CAST('00:00:00' AS TIME) + to_microseconds(epoch_ns({}) // 1000))",
                operand.sql
            ),
            false => format!("CAST({} AS TIME)", operand.sql),
        };
        return Ok(Emitted::new(
            format!(
                "strftime(CAST('1970-01-01' AS DATE) + {micros}, {})",
                string_literal(&pattern, site.context.log)
            ),
            SnowflakeType::Varchar,
        ));
    }
    let precision = match operand.data_type {
        SnowflakeType::Date => None,
        SnowflakeType::TimestampNtz { precision } => Some(precision),
        SnowflakeType::Null => Some(NANOSECOND_DIGITS),
        SnowflakeType::Number { precision, scale } => {
            return number_text((&operand.sql, precision, scale), &written, construct, site);
        }
        zoned @ SnowflakeType::TimestampTz { .. } => {
            return Err(refuse(zoned_local(
                &format!("the format '{written}'"),
                zoned,
            )));
        }
        other => return Err(refuse(format!("a format for {other}"))),
    };
    let pattern = strftime_pattern(&written, precision)
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

/// TO_CHAR of a NUMBER with a format `FM9...90.0...0` its whole digits always hold.
fn number_text(
    (sql, precision, scale): (&str, u8, u8),
    written: &str,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(NUMBER_TEXT_FORMAT);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let (whole, fraction) = fixed_number_format(written).ok_or_else(|| {
        refuse(format!(
            "the number format '{written}', which no recording shows"
        ))
    })?;
    let carry = u8::from(scale > fraction);
    let needed = (precision - scale).saturating_add(carry);
    if needed > whole {
        return Err(refuse(format!(
            "the number format '{written}' for NUMBER({precision},{scale}), whose values may need more than {whole} whole digits"
        )));
    }
    let wide = scale.max(fraction);
    Ok(Emitted::new(
        format!(
            "list_transform([CAST({sql} AS DECIMAL(38,{wide}))], lambda number_value: \
             CASE WHEN number_value IS NULL THEN NULL \
             ELSE (CASE WHEN number_value < 0 THEN '-' ELSE '' END) || \
             CAST(CAST(abs(round(number_value, {fraction})) AS DECIMAL(38,{fraction})) AS VARCHAR) END)[1]"
        ),
        SnowflakeType::Varchar,
    ))
}

/// The whole digits and fraction digits of a format `FM`, nines, one zero, a point and zeros.
fn fixed_number_format(written: &str) -> Option<(u8, u8)> {
    let upper = written.to_ascii_uppercase();
    let body = upper.strip_prefix("FM")?;
    let (whole, fraction) = body.split_once('.')?;
    let nines = whole.strip_suffix('0')?;
    if !nines.bytes().all(|byte| byte == b'9')
        || fraction.is_empty()
        || !fraction.bytes().all(|byte| byte == b'0')
    {
        return None;
    }
    let (Ok(whole), Ok(fraction)) = (u8::try_from(whole.len()), u8::try_from(fraction.len()))
    else {
        return None;
    };
    Some((whole, fraction))
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
        if !RECORDED_TIME_ZONES.contains(&zone.as_str()) {
            return Err(outside_conditions(
                construct.clone(),
                format!(
                    "the time zone '{zone}', whose spelling no recording shows Snowflake accepting"
                ),
            ));
        }
        return Ok(Zone {
            sql: string_literal(&zone, site.context.log),
            literal: Some(zone),
        });
    }
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::Varchar | SnowflakeType::Null => {
            let log = site.context.log;
            let _column = log.rule(ZONE_COLUMNS);
            diverge(site.context, TIME_ZONE_SPELLING, construct)?;
            let names: Vec<String> = RECORDED_TIME_ZONES
                .iter()
                .map(|zone| string_literal(zone, log))
                .collect();
            let unrecorded = raised(&format!(
                "{TIME_ZONE_SPELLING}: a time zone name whose spelling no recording shows \
                 Snowflake accepting"
            ));
            Ok(Zone {
                sql: format!(
                    "list_transform([{}], lambda z: CASE WHEN z IS NULL OR z IN ({}) THEN z \
                     ELSE CAST({unrecorded} AS VARCHAR) END)[1]",
                    emitted.sql,
                    names.join(", ")
                ),
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
    diverge(site.context, SESSION_TIMEZONE_UTC, construct)?;
    Ok(match name {
        "CURRENT_DATE" => Emitted::new(DUCKDB_CURRENT_DATE.to_owned(), SnowflakeType::Date),
        _ => Emitted::new(
            DUCKDB_CURRENT_TIMESTAMP.to_owned(),
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
    let mut operand = convert_expression(text, site)?;
    if let (SnowflakeType::TimestampTz { precision }, true) =
        (operand.data_type, operand.utc_offset)
    {
        let _zoned = site.context.log.rule(UTC_OFFSET_ARITHMETIC);
        rely_on_declared(&operand.declared, construct, site.context)?;
        operand = Emitted::new(
            utc_wall_time(&operand.sql, site.context.log),
            SnowflakeType::TimestampNtz { precision },
        );
    }
    if let (SnowflakeType::TimestampNtz { precision }, TimeFormat::Auto, false) =
        (operand.data_type, format, name.starts_with("TRY_"))
    {
        let _time = site.context.log.rule(TIMESTAMP_TIME_RULE);
        return Ok(Emitted::new(
            timestamp_time(
                &operand.sql,
                precision > MICROSECOND_DIGITS,
                site.context.log,
            ),
            SnowflakeType::Time {
                precision: NANOSECOND_DIGITS,
            },
        ));
    }
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

/// A UTC-offset TIMESTAMP_TZ as its UTC wall time, a naive timestamp of at most microseconds.
fn utc_wall(emitted: Emitted, log: &ProvenanceLog) -> Emitted {
    let precision = match emitted.data_type {
        SnowflakeType::TimestampTz { precision } => precision.min(MICROSECOND_DIGITS),
        _ => MICROSECOND_DIGITS,
    };
    Emitted::new(
        utc_wall_time(&emitted.sql, log),
        SnowflakeType::TimestampNtz { precision },
    )
}
