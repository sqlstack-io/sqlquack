//! Rendering canonical values as exact text for human review.

use sqlquack_compare::values::models::CanonicalValue;

/// Nanoseconds in a second.
const NANOS_PER_SECOND: i128 = 1_000_000_000;
/// The March-based month index of January; indexes from it on belong to the next civil year.
const MONTHS_FROM_MARCH: i128 = 10;
/// The last month that belongs to the next civil year in the March-based calendar.
const FEBRUARY: i128 = 2;
/// Seconds in a day.
const SECONDS_PER_DAY: i128 = 86_400;

/// The value as exact text: decimals padded to the column's declared scale, times to the nanosecond.
pub(crate) fn render_value(value: &CanonicalValue, declared_scale: Option<u8>) -> String {
    match value {
        CanonicalValue::Null => "NULL".to_owned(),
        CanonicalValue::Boolean(value) => value.to_string(),
        CanonicalValue::Number { unscaled, scale } => match declared_scale {
            Some(declared) if i16::from(declared) > i16::from(*scale) && *scale >= 0 => decimal(
                unscaled * 10_i128.pow(u32::from(declared) - u32::from(scale.unsigned_abs())),
                i16::from(declared),
            ),
            _ => decimal(*unscaled, i16::from(*scale)),
        },
        CanonicalValue::Float32(bits) => format!("{:?}", f32::from_bits(*bits)),
        CanonicalValue::Float64(bits) => format!("{:?}", f64::from_bits(*bits)),
        CanonicalValue::Text(text) => text_cell(text),
        CanonicalValue::Binary(bytes) => bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
        CanonicalValue::Date(days) => civil_date(i128::from(*days)),
        CanonicalValue::Time(nanos) => clock(i128::from(*nanos)),
        CanonicalValue::Timestamp { nanos, zone } => {
            let seconds = nanos.div_euclid(NANOS_PER_SECOND);
            let days = seconds.div_euclid(SECONDS_PER_DAY);
            let within_day = nanos - days * SECONDS_PER_DAY * NANOS_PER_SECOND;
            let zone = zone
                .as_ref()
                .map_or_else(String::new, |zone| format!(" {zone}"));
            format!("{} {}{zone}", civil_date(days), clock(within_day))
        }
    }
}

fn decimal(unscaled: i128, scale: i16) -> String {
    let sign = if unscaled < 0 { "-" } else { "" };
    let digits = unscaled.unsigned_abs().to_string();
    let Ok(scale) = usize::try_from(scale) else {
        let zeros = "0".repeat(usize::from(scale.unsigned_abs()));
        return format!("{sign}{digits}{zeros}");
    };
    if scale == 0 {
        return format!("{sign}{digits}");
    }
    let padded = format!("{digits:0>width$}", width = scale + 1);
    let (whole, fraction) = padded.split_at(padded.len() - scale);
    format!("{sign}{whole}.{fraction}")
}

/// Text with tabs, line breaks and backslashes escaped, and the empty string marked.
fn text_cell(text: &str) -> String {
    if text.is_empty() {
        return "(empty)".to_owned();
    }
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// `HH:MM:SS.fffffffff` for nanoseconds since midnight.
fn clock(nanos: i128) -> String {
    let seconds = nanos.div_euclid(NANOS_PER_SECOND);
    let fraction = nanos.rem_euclid(NANOS_PER_SECOND);
    format!(
        "{:02}:{:02}:{:02}.{fraction:09}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}

/// `YYYY-MM-DD` for days since 1970-01-01, in the proleptic Gregorian calendar.
fn civil_date(days: i128) -> String {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = match month_index < MONTHS_FROM_MARCH {
        true => month_index + 3,
        false => month_index - 9,
    };
    let year = year_of_era + era * 400 + i128::from(month <= FEBRUARY);
    format!("{year:04}-{month:02}-{day:02}")
}
