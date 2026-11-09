//! The values BigQuery's profile and lineage rules read.

/// The arguments of `DATE(year, month, day)`.
pub(crate) const DATE_FROM_PARTS_ARGUMENTS: usize = 3;
/// BigQuery's functions that take a date part, with the part's argument position.
pub(crate) const BIGQUERY_DATE_PART_FUNCTIONS: [(&str, usize); 9] = [
    ("DATE_TRUNC", 1),
    ("DATETIME_TRUNC", 1),
    ("TIMESTAMP_TRUNC", 1),
    ("TIME_TRUNC", 1),
    ("LAST_DAY", 1),
    ("DATE_DIFF", 2),
    ("DATETIME_DIFF", 2),
    ("TIMESTAMP_DIFF", 2),
    ("TIME_DIFF", 2),
];
/// The date parts BigQuery writes with a weekday argument, as `WEEK(MONDAY)`.
pub(crate) const BIGQUERY_WEEKDAY_PARTS: [&str; 1] = ["WEEK"];
