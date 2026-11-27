//! Date and timestamp functions: fields such as DAY and HOUR, and arithmetic in whole days and months.

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::as_type;
use crate::databricks::conversion::_helpers::functions::rewrites::{Rewrite, small_integer};
use crate::databricks::conversion::constants::{EPOCH_DATE, INT_RANGE, INT_SPAN};
use crate::databricks::conversion::models::{DatabricksType, Emitted};

/// The rewrite of the date function `name`; DAYOFWEEK counts from Sunday = 1, WEEKDAY from Monday = 0, weeks as ISO 8601.
pub(crate) fn date_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "YEAR" => |values| field(values, "year({x})", false),
        "QUARTER" => |values| field(values, "quarter({x})", false),
        "MONTH" => |values| field(values, "month({x})", false),
        "DAY" | "DAYOFMONTH" => |values| field(values, "day({x})", false),
        "DAYOFYEAR" => |values| field(values, "dayofyear({x})", false),
        "DAYOFWEEK" => |values| field(values, "dayofweek({x}) + 1", false),
        "WEEKDAY" => |values| field(values, "isodow({x}) - 1", false),
        "WEEKOFYEAR" => |values| field(values, "weekofyear({x})", false),
        "HOUR" => |values| field(values, "hour({x})", true),
        "MINUTE" => |values| field(values, "minute({x})", true),
        "SECOND" => |values| field(values, "second({x})", true),
        "DAYNAME" => |values| name_of(values, "%a"),
        "MONTHNAME" => |values| name_of(values, "%b"),
        "LAST_DAY" => last_day,
        "DATEDIFF" | "DATE_DIFF" => date_diff,
        "DATE_ADD" | "DATEADD" => |values| days_added(values, ""),
        "DATE_SUB" => |values| days_added(values, "- "),
        "ADD_MONTHS" => add_months,
        _ => return None,
    };
    Some(rewrite)
}

/// A field of a date or timestamp as INT; a time field takes only a timestamp.
fn field(values: &[Emitted], template: &str, time: bool) -> Option<Emitted> {
    let [value] = values else { return None };
    match (value.data_type, time) {
        (DatabricksType::Date, false)
        | (DatabricksType::Timestamp | DatabricksType::TimestampNtz, _) => Some(typed(
            format!("CAST({} AS INTEGER)", template.replace("{x}", &value.sql)),
            DatabricksType::Int,
        )),
        _ => None,
    }
}

/// `DAYNAME` or `MONTHNAME`: the three-letter English name, as `strftime` writes it.
fn name_of(values: &[Emitted], format: &str) -> Option<Emitted> {
    let [value] = values else { return None };
    match value.data_type {
        DatabricksType::Date | DatabricksType::Timestamp | DatabricksType::TimestampNtz => {
            Some(typed(
                format!("strftime({}, '{format}')", value.sql),
                DatabricksType::String,
            ))
        }
        _ => None,
    }
}

/// `LAST_DAY(d)`: the month's last date.
fn last_day(values: &[Emitted]) -> Option<Emitted> {
    let [date] = values else { return None };
    (date.data_type == DatabricksType::Date)
        .then(|| typed(format!("last_day({})", date.sql), DatabricksType::Date))
}

/// `DATEDIFF(end, start)`: the days from `start` to `end` as INT.
fn date_diff(values: &[Emitted]) -> Option<Emitted> {
    let [end, start] = values else { return None };
    let dates = [end, start]
        .iter()
        .all(|date| date.data_type == DatabricksType::Date);
    dates.then(|| {
        typed(
            format!("CAST({} - {} AS INTEGER)", end.sql, start.sql),
            DatabricksType::Int,
        )
    })
}

/// `DATE_ADD(d, n)` or `DATE_SUB(d, n)`: whole days, wrapping as Databricks's INT day count wraps.
fn days_added(values: &[Emitted], sign: &str) -> Option<Emitted> {
    let [date, days] = values else { return None };
    if date.data_type != DatabricksType::Date || !small_integer(days.data_type) {
        return None;
    }
    let total = format!(
        "CAST({} - DATE '{EPOCH_DATE}' AS BIGINT) {} {}",
        date.sql,
        match sign {
            "" => "+",
            _ => "-",
        },
        as_type(days, DatabricksType::BigInt)
    );
    let wrapped =
        format!("((({total}) + {INT_RANGE}) % {INT_SPAN} + {INT_SPAN}) % {INT_SPAN} - {INT_RANGE}");
    Some(typed(
        format!("DATE '{EPOCH_DATE}' + CAST({wrapped} AS INTEGER)"),
        DatabricksType::Date,
    ))
}

/// `ADD_MONTHS(d, n)`: the same day `n` months on, the month's last day when it has fewer.
fn add_months(values: &[Emitted]) -> Option<Emitted> {
    let [date, months] = values else { return None };
    if date.data_type != DatabricksType::Date || !months.data_type.is_integral() {
        return None;
    }
    Some(typed(
        format!(
            "CAST({} + to_months({}) AS DATE)",
            date.sql,
            as_type(months, DatabricksType::Int)
        ),
        DatabricksType::Date,
    ))
}
