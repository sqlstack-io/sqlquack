//! Date and timestamp functions: fields such as DAY and HOUR, and arithmetic in whole days and months.

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::as_type;
use crate::databricks::conversion::_helpers::functions::rewrites::{Rewrite, small_integer};
use crate::databricks::conversion::constants::{
    DATE_TRUNC_UNITS, EPOCH_DATE, INT_RANGE, INT_SPAN, MICROS_PER_MILLI, MICROS_PER_SECOND,
    MONTHS_BETWEEN_SCALE, SECONDS_PER_DAY, SECONDS_PER_MONTH, TIMESTAMP_TRUNC_UNITS,
    TIMESTAMP_YEARS,
};
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
        "MONTHS_BETWEEN" => months_between,
        "UNIX_SECONDS" => |values| since_epoch(values, MICROS_PER_SECOND),
        "UNIX_MILLIS" => |values| since_epoch(values, MICROS_PER_MILLI),
        "UNIX_MICROS" => |values| since_epoch(values, 1),
        "UNIX_DATE" => unix_date,
        "TIMESTAMP_SECONDS" => |values| from_epoch(values, MICROS_PER_SECOND),
        "TIMESTAMP_MILLIS" => |values| from_epoch(values, MICROS_PER_MILLI),
        "TIMESTAMP_MICROS" => |values| from_epoch(values, 1),
        "DATE_FROM_UNIX_DATE" => date_from_unix_date,
        "DATE_TRUNC" => |values| truncated(values, true),
        "TRUNC" => |values| truncated(values, false),
        "TRY_MAKE_TIMESTAMP" | "TRY_MAKE_TIMESTAMP_LTZ" => {
            |values| made_timestamp(values, DatabricksType::Timestamp)
        }
        "TRY_MAKE_TIMESTAMP_NTZ" => |values| made_timestamp(values, DatabricksType::TimestampNtz),
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

/// `TRY_MAKE_TIMESTAMP` of integers: NULL for an invalid field, raising only in the edge years DuckDB holds less of.
fn made_timestamp(values: &[Emitted], result: DatabricksType) -> Option<Emitted> {
    let [year, month, day, hour, minute, second] = values else {
        return None;
    };
    if !values.iter().all(|value| small_integer(value.data_type)) {
        return None;
    }
    let (y, h, mi, s) = (&year.sql, &hour.sql, &minute.sql, &second.sql);
    let made = format!(
        "make_timestamp(CAST({y} AS BIGINT), CAST({} AS BIGINT), CAST({} AS BIGINT), CAST({h} AS BIGINT), \
         CAST({mi} AS BIGINT), CAST({s} AS DOUBLE))",
        month.sql, day.sql
    );
    let (first, last) = TIMESTAMP_YEARS;
    Some(typed(
        format!(
            "CAST(CASE WHEN NOT ({h} BETWEEN 0 AND 23 AND {mi} BETWEEN 0 AND 59 AND {s} BETWEEN 0 AND 60) \
             OR {y} < {} OR {y} > {} THEN NULL WHEN {y} BETWEEN {first} AND {last} THEN try({made}) \
             ELSE {made} END AS {})",
            first - 1,
            last + 1,
            result.duckdb_name()
        ),
        result,
    ))
}

/// `UNIX_SECONDS`, `UNIX_MILLIS` or `UNIX_MICROS` of a timestamp: whole units since 1970, rounded down.
fn since_epoch(values: &[Emitted], unit: i64) -> Option<Emitted> {
    let [value] = values else { return None };
    if !matches!(
        value.data_type,
        DatabricksType::Timestamp | DatabricksType::TimestampNtz
    ) {
        return None;
    }
    let micros = format!("epoch_us({})", value.sql);
    let sql = match unit {
        1 => micros,
        unit => format!("(({micros} - (({micros} % {unit}) + {unit}) % {unit}) // {unit})"),
    };
    Some(typed(sql, DatabricksType::BigInt))
}

/// `UNIX_DATE(d)`: the days since 1970-01-01 as INT.
fn unix_date(values: &[Emitted]) -> Option<Emitted> {
    let [date] = values else { return None };
    (date.data_type == DatabricksType::Date).then(|| {
        typed(
            format!("CAST({} - DATE '{EPOCH_DATE}' AS INTEGER)", date.sql),
            DatabricksType::Int,
        )
    })
}

/// `TIMESTAMP_SECONDS`, `_MILLIS` or `_MICROS` of an INT, whose every value both engines hold.
fn from_epoch(values: &[Emitted], unit: i64) -> Option<Emitted> {
    let [count] = values else { return None };
    small_integer(count.data_type).then(|| {
        typed(
            format!(
                "CAST(make_timestamp(CAST({} AS BIGINT) * {unit}) AS TIMESTAMPTZ)",
                count.sql
            ),
            DatabricksType::Timestamp,
        )
    })
}

/// `DATE_FROM_UNIX_DATE(n)` of a SMALLINT or TINYINT; DuckDB's dates end a day short of INT's range.
fn date_from_unix_date(values: &[Emitted]) -> Option<Emitted> {
    let [days] = values else { return None };
    matches!(
        days.data_type,
        DatabricksType::TinyInt | DatabricksType::SmallInt
    )
    .then(|| {
        typed(
            format!("DATE '{EPOCH_DATE}' + CAST({} AS INTEGER)", days.sql),
            DatabricksType::Date,
        )
    })
}

/// `DATE_TRUNC(unit, t)` as a TIMESTAMP or `TRUNC(d, unit)` as a DATE: NULL for a unit Databricks does not name.
fn truncated(values: &[Emitted], timestamp: bool) -> Option<Emitted> {
    let (unit, value) = match (values, timestamp) {
        ([unit, value], true) | ([value, unit], false) => (unit, value),
        _ => return None,
    };
    let texts = matches!(
        unit.data_type,
        DatabricksType::String | DatabricksType::Null
    );
    let source = match (value.data_type, timestamp) {
        (DatabricksType::Date, false) => value.sql.clone(),
        (DatabricksType::Date | DatabricksType::Timestamp | DatabricksType::TimestampNtz, true) => {
            as_type(value, DatabricksType::Timestamp)
        }
        _ => return None,
    };
    if !texts {
        return None;
    }
    let (units, result) = match timestamp {
        true => (TIMESTAMP_TRUNC_UNITS, DatabricksType::Timestamp),
        false => (DATE_TRUNC_UNITS, DatabricksType::Date),
    };
    let mut sql = "CASE".to_owned();
    for (part, names) in units {
        let names: Vec<String> = names.iter().map(|name| format!("'{name}'")).collect();
        sql.push_str(&format!(
            " WHEN upper({}) IN ({}) THEN CAST(date_trunc('{part}', {source}) AS {})",
            unit.sql,
            names.join(", "),
            result.duckdb_name()
        ));
    }
    sql.push_str(" END");
    Some(typed(sql, result))
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

/// `MONTHS_BETWEEN(a, b[, round])`: whole months on the same or both last days, else plus 31-day months of seconds, rounded to 8 places as Java's `Math.round` does.
fn months_between(values: &[Emitted]) -> Option<Emitted> {
    let (later, earlier, rounded) = match values {
        [later, earlier] => (later, earlier, true),
        [later, earlier, round] => match round.sql.as_str() {
            "TRUE" => (later, earlier, true),
            "FALSE" => (later, earlier, false),
            _ => return None,
        },
        _ => return None,
    };
    let local = |value: &Emitted| match value.data_type {
        DatabricksType::Date | DatabricksType::Timestamp | DatabricksType::TimestampNtz => {
            Some(as_type(value, DatabricksType::TimestampNtz))
        }
        _ => None,
    };
    let (a, b) = (local(later)?, local(earlier)?);
    let (da, db) = (format!("CAST({a} AS DATE)"), format!("CAST({b} AS DATE)"));
    let months = format!(
        "CAST((year({da}) * 12 + month({da})) - (year({db}) * 12 + month({db})) AS DOUBLE)"
    );
    let seconds = |t: &str| format!("(hour({t}) * 3600 + minute({t}) * 60 + second({t}))");
    let fraction = format!(
        "({months} + CAST((day({da}) - day({db})) * {SECONDS_PER_DAY} + {sa} - {sb} AS DOUBLE) / {SECONDS_PER_MONTH})",
        sa = seconds(&a),
        sb = seconds(&b)
    );
    let fraction = match rounded {
        true => {
            let scaled = format!("({fraction} * {MONTHS_BETWEEN_SCALE})");
            format!(
                "((floor({scaled}) + CASE WHEN {scaled} - floor({scaled}) >= 0.5 THEN 1 ELSE 0 END) / {MONTHS_BETWEEN_SCALE})"
            )
        }
        false => fraction,
    };
    Some(typed(
        format!(
            "CASE WHEN day({da}) = day({db}) OR ({da} = last_day({da}) AND {db} = last_day({db})) \
             THEN {months} ELSE {fraction} END"
        ),
        DatabricksType::Double,
    ))
}
