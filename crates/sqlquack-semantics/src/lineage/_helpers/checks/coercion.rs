//! Each engine's conservative coercion policy: what compares, combines and converts.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;

use crate::lineage::_helpers::checks::families::Family;
use crate::lineage::constants::{
    BIGQUERY_PROFILE, DATABRICKS_PROFILE, DECIMAL_BASE, DUCKDB_PROFILE, LAST_HOUR,
    LAST_MINUTE_OR_SECOND, POSTGRES_PROFILE, SQLITE_PROFILE,
};
use crate::lineage::models::LineageContext;

/// The length of an ISO date, `YYYY-MM-DD`, and where its dashes and parts are.
const ISO_DATE_LENGTH: usize = 10;
const DATE_SEPARATORS: [usize; 2] = [4, 7];
const YEAR: std::ops::Range<usize> = 0..4;
const MONTH: std::ops::Range<usize> = 5..7;
const DAY: std::ops::Range<usize> = 8..10;
/// The length of an ISO timestamp to the second, and where its colons and parts are.
const ISO_TIMESTAMP_LENGTH: usize = 19;
const TIME_SEPARATORS: [usize; 2] = [13, 16];
const HOUR: std::ops::Range<usize> = 11..13;
const MINUTE: std::ops::Range<usize> = 14..16;
const SECOND: std::ops::Range<usize> = 17..19;
/// `24:00:00`, the end of a day, is the one valid time past hour 23.
const DAY_END_HOUR: u32 = 24;
const LAST_MINUTE: u32 = 59;
/// PostgreSQL reads second 60, a leap second, as the next minute.
const POSTGRES_LAST_SECOND: u32 = 60;
/// The words PostgreSQL reads as a date or timestamp, past an optional sign on `infinity`.
const POSTGRES_TEMPORAL_WORDS: [&str; 8] = [
    "infinity",
    "-infinity",
    "+infinity",
    "epoch",
    "today",
    "tomorrow",
    "yesterday",
    "now",
];
/// The words `numeric`, `real` or `double precision` reads, after an optional sign.
const POSTGRES_NUMBER_WORDS: [&str; 3] = ["inf", "infinity", "nan"];
/// The words whose every prefix PostgreSQL reads as a boolean.
const POSTGRES_BOOLEAN_WORDS: [&str; 4] = ["true", "false", "yes", "no"];
const DECIMAL_RADIX: u32 = 10;
const HEX_RADIX: u32 = 16;
const OCTAL_RADIX: u32 = 8;
const BINARY_RADIX: u32 = 2;

/// The engine whose binder the checks follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Engine {
    DuckDB,
    /// Snowflake; implicit conversion on USING keys isn't checked, as polyglot leaves it.
    Snowflake,
    /// PostgreSQL: no implicit predicate, comparison or string set-operation coercions.
    Postgres,
    /// SQLite, whose dynamic typing is not recorded: no type findings, only scope and name rules.
    Sqlite,
    /// BigQuery (GoogleSQL): as PostgreSQL, and a string literal converts only to a date or time.
    BigQuery,
    /// Databricks SQL, coercions not recorded: no type findings, only scope and name rules.
    Databricks,
}

impl Engine {
    pub(crate) fn of(context: LineageContext<'_>) -> Self {
        match context.profile.name() {
            DUCKDB_PROFILE => Self::DuckDB,
            POSTGRES_PROFILE => Self::Postgres,
            SQLITE_PROFILE => Self::Sqlite,
            BIGQUERY_PROFILE => Self::BigQuery,
            DATABRICKS_PROFILE => Self::Databricks,
            _ => Self::Snowflake,
        }
    }

    /// The families a predicate position accepts besides BOOLEAN.
    fn predicates(self) -> &'static [Family] {
        match self {
            Self::DuckDB => &[Family::Integer, Family::Numeric],
            Self::Snowflake => &[Family::Integer, Family::Numeric, Family::String],
            Self::Postgres | Self::Sqlite | Self::BigQuery | Self::Databricks => &[],
        }
    }

    /// The family pairs a comparison accepts beyond equal, numeric and temporal ones.
    fn comparison_pairs(self) -> &'static [(Family, Family)] {
        match self {
            Self::DuckDB => &[
                (Family::Boolean, Family::Integer),
                (Family::Boolean, Family::Numeric),
            ],
            Self::Snowflake => &[
                (Family::String, Family::Integer),
                (Family::String, Family::Numeric),
                (Family::String, Family::Boolean),
                (Family::String, Family::Date),
                (Family::String, Family::Time),
                (Family::String, Family::Timestamp),
                (Family::Integer, Family::Boolean),
                (Family::Numeric, Family::Boolean),
            ],
            Self::Postgres | Self::Sqlite | Self::BigQuery | Self::Databricks => &[],
        }
    }
}

/// Whether two families compare without any conversion.
pub(crate) fn naturally_comparable(left: Family, right: Family) -> bool {
    left == Family::Unknown
        || right == Family::Unknown
        || left == right
        || (left.is_numeric() && right.is_numeric())
        || (left.is_temporal() && right.is_temporal())
}

/// Whether a predicate position accepts a value of `family`.
pub(crate) fn predicate(engine: Engine, family: Family) -> bool {
    matches!(family, Family::Unknown | Family::Boolean)
        || !family.fully_modelled()
        || engine.predicates().contains(&family)
}

/// Whether `expression` is a string literal, past aliases, parentheses and comments.
pub(crate) fn string_literal(expression: &Expression) -> bool {
    match expression {
        Expression::Literal(literal) => matches!(literal.as_ref(), Literal::String(_)),
        Expression::Alias(alias) => string_literal(&alias.this),
        Expression::Paren(paren) => string_literal(&paren.this),
        Expression::Annotated(annotated)
            if annotated.type_assertion.is_none() && annotated.type_like.is_none() =>
        {
            string_literal(&annotated.this)
        }
        _ => false,
    }
}

/// The text of a string literal, past aliases, parentheses and comments.
fn string_literal_text(expression: &Expression) -> Option<&str> {
    match expression {
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::String(value) => Some(value),
            _ => None,
        },
        Expression::Alias(alias) => string_literal_text(&alias.this),
        Expression::Paren(paren) => string_literal_text(&paren.this),
        Expression::Annotated(annotated)
            if annotated.type_assertion.is_none() && annotated.type_like.is_none() =>
        {
            string_literal_text(&annotated.this)
        }
        _ => None,
    }
}

/// The digits of `text` up to `separator`, at least one and at most `widest`, with the rest.
fn digits_then(text: &str, widest: usize, separator: Option<char>) -> Option<(u32, &str)> {
    let end: usize = text
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(text.len());
    if end == 0 || end > widest {
        return None;
    }
    let rest: &str = &text[end..];
    let rest: &str = match separator {
        Some(separator) => rest.strip_prefix(separator)?,
        None => rest,
    };
    let value: u32 = text[..end].bytes().fold(0, |value, digit| {
        value * DECIMAL_BASE + u32::from(digit - b'0')
    });
    Some((value, rest))
}

/// `[Y]YYY-[M]M-[D]D` at the start of `text`, with what follows it.
fn date_prefix(text: &str) -> Option<&str> {
    let (_, rest) = digits_then(text, 4, Some('-'))?;
    let (month, rest) = digits_then(rest, 2, Some('-'))?;
    let (day, rest) = digits_then(rest, 2, None)?;
    ((1..=12).contains(&month) && (1..=31).contains(&day)).then_some(rest)
}

/// `[H]H:[M]M[:[S]S[.F]]` at the start of `text`, with what follows it.
fn time_prefix(text: &str) -> Option<&str> {
    let (hour, rest) = digits_then(text, 2, Some(':'))?;
    let (minute, rest) = digits_then(rest, 2, None)?;
    let rest: &str = match rest.strip_prefix(':') {
        Some(seconds) => {
            let (second, rest) = digits_then(seconds, 2, None)?;
            if second > LAST_MINUTE_OR_SECOND {
                return None;
            }
            match rest.strip_prefix('.') {
                Some(fraction) => {
                    fraction.trim_start_matches(|character: char| character.is_ascii_digit())
                }
                None => rest,
            }
        }
        None => rest,
    };
    (hour <= LAST_HOUR && minute <= LAST_MINUTE_OR_SECOND).then_some(rest)
}

/// Whether BigQuery reads string literal `expression` as a valid value of temporal `family`.
pub(crate) fn literal_reads_as(expression: &Expression, family: Family) -> bool {
    let Some(text) = string_literal_text(expression).map(str::trim) else {
        return false;
    };
    match family {
        Family::Date => date_prefix(text) == Some(""),
        Family::Time => time_prefix(text) == Some(""),
        Family::Timestamp => date_prefix(text).is_some_and(|rest| {
            rest.is_empty()
                || rest
                    .strip_prefix([' ', 'T', 't'])
                    .and_then(time_prefix)
                    .is_some()
        }),
        _ => false,
    }
}

/// Whether a string literal converts to `target`; in BigQuery only to a date, time or bytes.
pub(crate) fn literal_coerces(engine: Engine, expression: &Expression, target: Family) -> bool {
    if engine == Engine::BigQuery {
        return string_literal(expression)
            && matches!(
                target,
                Family::Date | Family::Time | Family::Timestamp | Family::Binary
            );
    }
    string_literal(expression)
        && (target.is_numeric()
            || matches!(target, Family::Date | Family::Time | Family::Timestamp)
            || target == Family::Boolean)
}

/// Whether two values of these families and forms may be compared.
pub(crate) fn comparable(
    engine: Engine,
    (left, right): (&Expression, &Expression),
    (left_family, right_family): (Family, Family),
) -> bool {
    !left_family.fully_modelled()
        || !right_family.fully_modelled()
        || naturally_comparable(left_family, right_family)
        || engine
            .comparison_pairs()
            .contains(&(left_family, right_family))
        || engine
            .comparison_pairs()
            .contains(&(right_family, left_family))
        || literal_coerces(engine, left, right_family)
        || literal_coerces(engine, right, left_family)
}

/// Accepted comparisons whose success depends on the runtime value.
pub(crate) fn runtime_comparison(left: Family, right: Family) -> bool {
    let converts =
        |family: Family| family.is_numeric() || family == Family::Boolean || family.is_temporal();
    (left == Family::String && converts(right)) || (right == Family::String && converts(left))
}

/// DuckDB equality between distinct non-numeric families, which converts at runtime.
pub(crate) fn runtime_equality(engine: Engine, left: Family, right: Family) -> bool {
    engine == Engine::DuckDB
        && left.fully_modelled()
        && right.fully_modelled()
        && left != right
        && !(left.is_numeric() && right.is_numeric())
}

/// Whether a temporal argument position accepts this value.
pub(crate) fn temporal_argument(engine: Engine, expression: &Expression, family: Family) -> bool {
    family.is_temporal()
        || (engine == Engine::Snowflake && family == Family::String)
        || literal_coerces(engine, expression, Family::Timestamp)
}

/// A function's temporal argument; Snowflake's DATE_TRUNC, EXTRACT and DATE_PART take no VARCHAR.
pub(crate) fn temporal_function_argument(
    name: &str,
    engine: Engine,
    (expression, family): (&Expression, Family),
) -> bool {
    if engine == Engine::Snowflake && matches!(name, "date_trunc" | "extract" | "date_part") {
        return family.is_temporal();
    }
    temporal_argument(engine, expression, family)
}

/// Whether BigQuery reads a string literal operand of arithmetic `node` as the other's date or time.
pub(crate) fn temporal_literal_arithmetic(
    engine: Engine,
    node: &Expression,
    (left, right): (&Expression, &Expression),
    (left_family, right_family): (Family, Family),
) -> bool {
    const TEMPORAL: [Family; 3] = [Family::Date, Family::Time, Family::Timestamp];
    let target = |other: Family| match TEMPORAL.contains(&other) {
        true => other,
        false => Family::Date,
    };
    engine == Engine::BigQuery
        && ((literal_reads_as(left, target(right_family))
            && arithmetic(engine, node, (target(right_family), right_family)))
            || (literal_reads_as(right, target(left_family))
                && arithmetic(engine, node, (left_family, target(left_family)))))
}

/// Whether set-operation branches of these families combine.
pub(crate) fn setop(engine: Engine, left: Family, right: Family) -> bool {
    let nested = |family: Family| matches!(family, Family::Array | Family::Map | Family::Struct);
    !left.fully_modelled()
        || !right.fully_modelled()
        || naturally_comparable(left, right)
        || (!matches!(engine, Engine::Postgres | Engine::BigQuery)
            && (left == Family::String || right == Family::String)
            && !nested(left)
            && !nested(right))
        || (engine == Engine::DuckDB
            && ((left == Family::Boolean && right.is_numeric())
                || (right == Family::Boolean && left.is_numeric())))
}

/// The string literal a query projects at `index`, when it is one.
pub(crate) fn projection_literal(query: &Expression, index: usize) -> Option<&Expression> {
    match query {
        Expression::Select(select) => select
            .expressions
            .get(index)
            .filter(|projection| string_literal(projection)),
        Expression::Subquery(subquery) => projection_literal(&subquery.this, index),
        Expression::Paren(paren) => projection_literal(&paren.this, index),
        _ => None,
    }
}

/// Whether arithmetic `node` over these families is an engine overload.
pub(crate) fn arithmetic(
    engine: Engine,
    node: &Expression,
    (left, right): (Family, Family),
) -> bool {
    use Family::{Date, Integer, Interval, String, Time, Timestamp};
    if !left.fully_modelled()
        || !right.fully_modelled()
        || (left.is_numeric() && right.is_numeric())
    {
        return true;
    }
    let add: bool = matches!(node, Expression::Add(_));
    let sub: bool = matches!(node, Expression::Sub(_));
    if matches!(engine, Engine::DuckDB | Engine::Postgres | Engine::BigQuery)
        && ((matches!(node, Expression::Mul(_))
            && ((left == Interval && right.is_numeric())
                || (right == Interval && left.is_numeric())))
            || (matches!(node, Expression::Div(_)) && left == Interval && right.is_numeric()))
    {
        return true;
    }
    const ADD: [(Family, Family); 9] = [
        (Date, Integer),
        (Integer, Date),
        (Date, Interval),
        (Interval, Date),
        (Timestamp, Interval),
        (Interval, Timestamp),
        (Time, Interval),
        (Interval, Time),
        (Interval, Interval),
    ];
    const SUB: [(Family, Family); 7] = [
        (Date, Integer),
        (Date, Date),
        (Date, Interval),
        (Timestamp, Interval),
        (Timestamp, Timestamp),
        (Time, Interval),
        (Interval, Interval),
    ];
    if (add && ADD.contains(&(left, right))) || (sub && SUB.contains(&(left, right))) {
        return true;
    }
    if engine == Engine::BigQuery
        && sub
        && matches!(
            (left, right),
            (Date, Timestamp) | (Timestamp, Date) | (Time, Time)
        )
    {
        return true;
    }
    if matches!(engine, Engine::DuckDB | Engine::Postgres)
        && ((add && matches!((left, right), (Date, Time) | (Time, Date)))
            || (sub && matches!((left, right), (Date, Timestamp) | (Timestamp, Date)))
            || (sub && engine == Engine::Postgres && (left, right) == (Time, Time)))
    {
        return true;
    }
    engine == Engine::Snowflake
        && ((left == String && right.is_numeric()) || (right == String && left.is_numeric()))
}

/// Whether `expression` is a string literal that cannot convert to `target` under `engine`.
pub(crate) fn invalid_literal_for(engine: Engine, expression: &Expression, target: Family) -> bool {
    match expression {
        Expression::Alias(alias) => invalid_literal_for(engine, &alias.this, target),
        Expression::Paren(paren) => invalid_literal_for(engine, &paren.this, target),
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::String(value) if engine == Engine::Postgres => {
                invalid_postgres_literal(value, target)
            }
            Literal::String(value) => match target {
                Family::Integer | Family::Numeric => value.trim().parse::<f64>().is_err(),
                Family::Date | Family::Timestamp => invalid_temporal_literal(value),
                Family::Boolean => !matches!(
                    value.to_ascii_lowercase().as_str(),
                    "true" | "false" | "t" | "f" | "yes" | "no" | "y" | "n" | "1" | "0"
                ),
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}

/// Whether PostgreSQL's input function for every type of `target` refuses `value`.
fn invalid_postgres_literal(value: &str, target: Family) -> bool {
    let text: &str = value.trim_matches(|character: char| character.is_ascii_whitespace());
    let lower: String = text.to_ascii_lowercase();
    match target {
        Family::Integer => !postgres_number(&lower, true),
        Family::Numeric => !postgres_number(&lower, false),
        Family::Date | Family::Timestamp => {
            invalid_temporal(text, POSTGRES_LAST_SECOND)
                || (!text.chars().any(|character| character.is_ascii_digit())
                    && !POSTGRES_TEMPORAL_WORDS.contains(&lower.as_str()))
        }
        Family::Boolean => !postgres_boolean(&lower),
        _ => false,
    }
}

/// Whether PostgreSQL reads lower-cased `text` as an integer, or as `numeric`, `real` or `double precision`.
fn postgres_number(text: &str, integer: bool) -> bool {
    let unsigned: &str = text.strip_prefix(['+', '-']).unwrap_or(text);
    if !integer && POSTGRES_NUMBER_WORDS.contains(&unsigned) {
        return true;
    }
    if let Some(digits) = unsigned.strip_prefix("0x") {
        return !integer || digit_groups(digits, HEX_RADIX, true);
    }
    if let Some(digits) = unsigned.strip_prefix("0o") {
        return digit_groups(digits, OCTAL_RADIX, true);
    }
    if let Some(digits) = unsigned.strip_prefix("0b") {
        return digit_groups(digits, BINARY_RADIX, true);
    }
    if integer {
        return digit_groups(unsigned, DECIMAL_RADIX, false);
    }
    let (mantissa, exponent): (&str, Option<&str>) = match unsigned.split_once('e') {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (unsigned, None),
    };
    let (whole, fraction): (&str, &str) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let decimal = |part: &str| part.is_empty() || digit_groups(part, DECIMAL_RADIX, false);
    (!whole.is_empty() || !fraction.is_empty())
        && decimal(whole)
        && decimal(fraction)
        && exponent.is_none_or(|exponent| {
            let digits: &str = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            digit_groups(digits, DECIMAL_RADIX, false)
        })
}

/// Digits of `radix` with single underscores between them, or after a base prefix when `prefixed`.
fn digit_groups(text: &str, radix: u32, prefixed: bool) -> bool {
    let mut after_digit: bool = prefixed;
    let mut any_digit: bool = false;
    for character in text.chars() {
        if character == '_' && after_digit {
            after_digit = false;
        } else if character.is_digit(radix) {
            after_digit = true;
            any_digit = true;
        } else {
            return false;
        }
    }
    any_digit && after_digit
}

/// Whether PostgreSQL reads lower-cased `text` as a boolean: a word's prefix, `on`, `of`, `off`, `1` or `0`.
fn postgres_boolean(text: &str) -> bool {
    !text.is_empty()
        && (POSTGRES_BOOLEAN_WORDS
            .iter()
            .any(|word| word.starts_with(text))
            || matches!(text, "on" | "of" | "off" | "1" | "0"))
}

/// An ISO calendar date that does not exist; other forms stay unchecked.
fn invalid_iso_date(value: &str) -> bool {
    let bytes: &[u8] = value.as_bytes();
    if bytes.len() < ISO_DATE_LENGTH
        || DATE_SEPARATORS
            .iter()
            .any(|position| bytes[*position] != b'-')
    {
        return false;
    }
    let part = |range: std::ops::Range<usize>| value.get(range).unwrap_or_default().parse::<u32>();
    let (Ok(year), Ok(month), Ok(day)) = (part(YEAR), part(MONTH), part(DAY)) else {
        return false;
    };
    let leap: bool = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days: u32 = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    day == 0 || day > days
}

/// A temporal literal no engine reads: an impossible date or time, or no digits at all.
pub(crate) fn invalid_temporal_literal(value: &str) -> bool {
    invalid_temporal(value, LAST_MINUTE)
        || (!value.chars().any(|character| character.is_ascii_digit())
            && !matches!(
                value.to_ascii_lowercase().as_str(),
                "infinity" | "-infinity" | "epoch" | "today" | "tomorrow" | "yesterday" | "now"
            ))
}

/// An impossible ISO date, or an ISO time past `last_second` or otherwise out of range.
fn invalid_temporal(value: &str, last_second: u32) -> bool {
    let bytes: &[u8] = value.as_bytes();
    let part = |range: std::ops::Range<usize>| value.get(range).unwrap_or_default().parse::<u32>();
    let invalid_time: bool = bytes.len() >= ISO_TIMESTAMP_LENGTH
        && TIME_SEPARATORS
            .iter()
            .all(|position| bytes[*position] == b':')
        && match (part(HOUR), part(MINUTE), part(SECOND)) {
            (Ok(hour), Ok(minute), Ok(second)) => {
                hour > DAY_END_HOUR
                    || minute > LAST_MINUTE
                    || second > last_second
                    || (hour == DAY_END_HOUR && (minute != 0 || second != 0))
            }
            _ => false,
        };
    invalid_iso_date(value) || invalid_time
}
