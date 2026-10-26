//! Timestamp literals and casts between DATE and timestamps.

use sqlquack_core::classification::models::Construct;
use sqlquack_duckdb::blocks::constants::SUBMICRO_MESSAGE;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::raised::raised;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_duckdb::temporal::main::submicro_check::submicro_check;
use sqlquack_duckdb::temporal::main::truncate_timestamp::truncate_timestamp;
use sqlquack_duckdb::temporal::main::truncate_timestamp_to_date::truncate_timestamp_to_date;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::Context;
use crate::conversion::constants::{
    HOURS_PER_DAY, MAX_FRACTION_DIGITS, MAX_OFFSET_HOURS, MICROSECOND_DIGITS, MINUTES_PER_HOUR,
    NANOSECOND_RANGE_END, NANOSECOND_RANGE_START, OFFSET_WITH_COLON, OFFSET_WITHOUT_COLON,
    TIMESTAMP_RANGE, TIMESTAMP_TZ_SUBMICRO,
};

/// A timestamp literal in the recorded ISO form: `YYYY-MM-DD[ HH:MI:SS[.f]][ ±HH:MM]`.
struct TimestampText<'a> {
    /// Year, month and day.
    date: (u16, u16, u16),
    /// `YYYY-MM-DD HH:MI:SS`, or the date alone.
    whole: &'a str,
    /// The fractional-second digits.
    fraction: &'a str,
    /// The UTC offset as `±HH:MM`.
    offset: Option<&'a str>,
}

/// The DuckDB SQL of a text literal cast to a timestamp type.
pub(crate) fn timestamp_literal(
    text: &str,
    to: SnowflakeType,
    (context, construct): (&Context<'_>, &Construct),
) -> Result<String, String> {
    let unproven = || format!("casting the text '{text}' to {to} has no proven conversion");
    let log = context.log;
    match to {
        SnowflakeType::TimestampNtz { precision } => {
            let parsed = parse_timestamp(text, false).ok_or_else(unproven)?;
            fits(&parsed, precision).ok_or_else(unproven)?;
            if precision > MICROSECOND_DIGITS && !in_nanosecond_range(parsed.date) {
                diverge(context, TIMESTAMP_RANGE, construct).map_err(|refusal| refusal.reason)?;
            }
            cast(&string_literal(text, log), to, log)
        }
        SnowflakeType::TimestampTz { precision } | SnowflakeType::TimestampLtz { precision } => {
            let parsed = parse_timestamp(text, true).ok_or_else(unproven)?;
            fits(&parsed, precision).ok_or_else(unproven)?;
            let (micros, submicros) = parsed
                .fraction
                .split_at(parsed.fraction.len().min(usize::from(MICROSECOND_DIGITS)));
            if submicros.bytes().any(|digit| digit != b'0') {
                diverge(context, TIMESTAMP_TZ_SUBMICRO, construct)
                    .map_err(|refusal| refusal.reason)?;
                return cast(&raised(SUBMICRO_MESSAGE), to, log);
            }
            let fraction = match micros.is_empty() {
                true => String::new(),
                false => format!(".{micros}"),
            };
            let offset = parsed.offset.map(offset_with_colon).unwrap_or_default();
            let normalised = format!("{}{fraction}{offset}", parsed.whole);
            cast(&string_literal(&normalised, log), to, log)
        }
        _ => Err(unproven()),
    }
}

/// The DuckDB SQL of a cast between DATE and timestamp types, or why it is unproven.
pub(crate) fn temporal_cast(
    sql: &str,
    (from, to): (SnowflakeType, SnowflakeType),
    (context, construct): (&Context<'_>, &Construct),
) -> Result<String, String> {
    let log = context.log;
    let range = || diverge(context, TIMESTAMP_RANGE, construct).map_err(|refusal| refusal.reason);
    match (from, to) {
        (
            SnowflakeType::TimestampNtz { precision: from },
            SnowflakeType::TimestampNtz { precision },
        ) if precision < from => Ok(truncate_timestamp(sql, (from, precision), log)),
        (
            SnowflakeType::TimestampNtz { precision: from },
            SnowflakeType::TimestampNtz { precision },
        ) => {
            if from <= MICROSECOND_DIGITS && precision > MICROSECOND_DIGITS {
                range()?;
            }
            cast(sql, to, log)
        }
        (SnowflakeType::TimestampNtz { precision }, SnowflakeType::Date) => {
            Ok(truncate_timestamp_to_date(sql, precision, log))
        }
        (SnowflakeType::Date, SnowflakeType::TimestampNtz { precision }) => {
            if precision > MICROSECOND_DIGITS {
                range()?;
            }
            cast(sql, to, log)
        }
        (
            SnowflakeType::TimestampNtz { precision: from },
            SnowflakeType::TimestampTz { precision } | SnowflakeType::TimestampLtz { precision },
        ) => {
            let kept = from.min(precision);
            let truncated = truncate_timestamp(sql, (from, kept), log);
            match kept > MICROSECOND_DIGITS {
                true => {
                    diverge(context, TIMESTAMP_TZ_SUBMICRO, construct)
                        .map_err(|refusal| refusal.reason)?;
                    cast(&submicro_check(&truncated, log), to, log)
                }
                false => cast(&truncated, to, log),
            }
        }
        (
            SnowflakeType::Date,
            SnowflakeType::TimestampTz { .. } | SnowflakeType::TimestampLtz { .. },
        ) => cast(sql, to, log),
        _ => Err(format!("casting {from} to {to} has no proven conversion")),
    }
}

/// Whether a type is DATE or a timestamp.
pub(crate) fn is_temporal(data_type: SnowflakeType) -> bool {
    matches!(
        data_type,
        SnowflakeType::Date
            | SnowflakeType::TimestampNtz { .. }
            | SnowflakeType::TimestampTz { .. }
            | SnowflakeType::TimestampLtz { .. }
    )
}

fn fits(parsed: &TimestampText<'_>, precision: u8) -> Option<()> {
    (parsed.fraction.len() <= usize::from(precision)).then_some(())
}

fn in_nanosecond_range(date: (u16, u16, u16)) -> bool {
    (NANOSECOND_RANGE_START..NANOSECOND_RANGE_END).contains(&date)
}

fn parse_timestamp(text: &str, zoned: bool) -> Option<TimestampText<'_>> {
    let date = text.get(..10)?;
    let parts: Vec<&str> = date.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return None;
    };
    let year = digits(year, 4)?;
    let month = digits(month, 2)?;
    let day = digits(day, 2)?;
    valid_date(year, month, day)?;
    let rest = &text[10..];
    let (time, offset) = match zoned {
        true => split_offset(rest)?,
        false => (rest, None),
    };
    let (whole_length, fraction) = match time {
        "" if offset.is_some() => return None,
        "" => (10, ""),
        _ => {
            let clock = time.strip_prefix(' ')?;
            let (clock, fraction) = clock.split_once('.').unwrap_or((clock, ""));
            let fields: Vec<&str> = clock.split(':').collect();
            let [hour, minute, second] = fields.as_slice() else {
                return None;
            };
            let in_range = digits(hour, 2)? < HOURS_PER_DAY
                && digits(minute, 2)? < MINUTES_PER_HOUR
                && digits(second, 2)? < MINUTES_PER_HOUR;
            let fraction_ok = (!time.contains('.') || !fraction.is_empty())
                && fraction.len() <= MAX_FRACTION_DIGITS
                && fraction.bytes().all(|byte| byte.is_ascii_digit());
            (in_range && fraction_ok).then_some(())?;
            (19, fraction)
        }
    };
    Some(TimestampText {
        date: (year, month, day),
        whole: &text[..whole_length],
        fraction,
        offset,
    })
}

/// Splits a trailing ` ±HH:MM` or ` ±HHMM` offset from the time.
fn split_offset(rest: &str) -> Option<(&str, Option<&str>)> {
    for length in [OFFSET_WITH_COLON, OFFSET_WITHOUT_COLON] {
        let Some(start) = rest.len().checked_sub(length) else {
            continue;
        };
        let Some(candidate) = rest.get(start..) else {
            continue;
        };
        let sign = candidate.as_bytes().get(1).copied();
        if !candidate.starts_with(' ') || !matches!(sign, Some(b'+' | b'-')) {
            continue;
        }
        let (hours, minutes) = candidate[2..]
            .split_once(':')
            .unwrap_or_else(|| candidate[2..].split_at(2));
        (digits(hours, 2)? <= MAX_OFFSET_HOURS && digits(minutes, 2)? < MINUTES_PER_HOUR)
            .then_some(())?;
        return Some((&rest[..start], Some(&candidate[1..])));
    }
    Some((rest, None))
}

/// An offset as `±HH:MM`, the spelling DuckDB reads.
fn offset_with_colon(offset: &str) -> String {
    match offset.contains(':') {
        true => offset.to_owned(),
        false => format!("{}:{}", &offset[..3], &offset[3..]),
    }
}

/// The value of exactly `length` decimal digits.
fn digits(text: &str, length: usize) -> Option<u16> {
    if text.len() != length {
        return None;
    }
    text.bytes().try_fold(0_u16, |value, byte| match byte {
        b'0'..=b'9' => value.checked_mul(10)?.checked_add(u16::from(byte - b'0')),
        _ => None,
    })
}

fn valid_date(year: u16, month: u16, day: u16) -> Option<()> {
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    (year >= 1 && day >= 1 && day <= days).then_some(())
}
