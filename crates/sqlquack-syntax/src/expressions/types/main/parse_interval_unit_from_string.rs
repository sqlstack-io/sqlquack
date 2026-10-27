//! `parse_interval_unit_from_string`: parse an interval unit from a string (used for splitting compound interval strings).

use crate::expressions::types::models::IntervalUnit;

/// Parse an interval unit from a string (used for splitting compound interval strings)
pub(crate) fn parse_interval_unit_from_string(s: &str) -> Option<IntervalUnit> {
    let unit = s.trim().to_ascii_uppercase();

    fn parse_base_unit(base: &str) -> Option<IntervalUnit> {
        match base {
            "YEAR" | "Y" | "YR" | "YY" | "YYYY" => Some(IntervalUnit::Year),
            "QUARTER" | "Q" | "QTR" | "QQ" => Some(IntervalUnit::Quarter),
            "MONTH" | "MON" | "MM" => Some(IntervalUnit::Month),
            "WEEK" | "W" | "WK" | "WW" => Some(IntervalUnit::Week),
            "DAY" | "D" | "DD" => Some(IntervalUnit::Day),
            "HOUR" | "H" | "HH" | "HR" => Some(IntervalUnit::Hour),
            "MINUTE" | "MI" | "MIN" | "N" => Some(IntervalUnit::Minute),
            "SECOND" | "S" | "SEC" | "SS" => Some(IntervalUnit::Second),
            "MILLISECOND" | "MS" | "MSEC" | "MSECOND" | "MILLISEC" | "MILLISECON" => {
                Some(IntervalUnit::Millisecond)
            }
            "MICROSECOND" | "US" | "USEC" | "USECOND" | "MICROSEC" | "MCS" => {
                Some(IntervalUnit::Microsecond)
            }
            "NANOSECOND" | "NS" | "NSEC" | "NSECOND" | "NANOSEC" => Some(IntervalUnit::Nanosecond),
            _ => None,
        }
    }

    parse_base_unit(&unit).or_else(|| {
        unit.strip_suffix('S')
            .filter(|base| !base.is_empty())
            .and_then(parse_base_unit)
    })
}
