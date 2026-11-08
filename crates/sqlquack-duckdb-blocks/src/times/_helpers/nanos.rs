//! TIME values as nanoseconds since midnight.

use crate::times::constants::NANOS_PER_SECOND;

/// The TIME_NS `nanos` nanoseconds after midnight, which must lie within one day.
pub(crate) fn time_of_nanos(nanos: &str) -> String {
    format!("CAST(make_timestamp_ns({nanos}) AS TIME_NS)")
}

/// `nanos` cut to `precision` fractional digits.
pub(crate) fn truncated_nanos(nanos: &str, precision: u8) -> String {
    let unit = NANOS_PER_SECOND / 10_i64.pow(u32::from(precision.min(9)));
    match unit {
        1 => nanos.to_owned(),
        unit => format!("({nanos} - {nanos} % {unit})"),
    }
}
