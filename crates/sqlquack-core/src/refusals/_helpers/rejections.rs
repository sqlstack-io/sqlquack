//! The constructs Snowflake parses but then rejects, recognised one node at a time.

use sqlquack_syntax::expressions::names::models::{Literal, Star};
use sqlquack_syntax::expressions::operators::models::Cast;
use sqlquack_syntax::expressions::types::constants::{
    SNOWFLAKE_UNPARAMETERIZED_LITERAL_TYPES, SNOWFLAKE_UNTYPED_LITERAL_TYPES,
};
use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::{Expression, ExpressionWalk};

/// The registry name of the rejection a node causes, with the reason, if it causes one.
pub(crate) fn rejection(node: &Expression) -> Option<(&'static str, String)> {
    match node {
        Expression::Intersect(operation) if operation.all => Some(set_operation_all("INTERSECT")),
        Expression::Except(operation) if operation.all => {
            Some(set_operation_all("EXCEPT or MINUS"))
        }
        Expression::Select(select) if select.locks.iter().any(|lock| lock.wait.is_some()) => {
            Some((
                "lock_wait",
                "Snowflake does not support WAIT in FOR UPDATE".to_owned(),
            ))
        }
        Expression::Select(select)
            if select.qualify.as_ref().is_some_and(|qualify| {
                !select.expressions.iter().any(has_window) && !has_window(&qualify.this)
            }) =>
        {
            Some((
                "qualify_without_window",
                "QUALIFY is used without a window function in the SELECT list or QUALIFY"
                    .to_owned(),
            ))
        }
        Expression::Star(star) => filtered_out_name(star).map(|name| {
            (
                "star_filter_drops_named_column",
                format!("{name} does not match the star's ILIKE pattern, so REPLACE or RENAME names a column the pattern removed"),
            )
        }),
        Expression::Cast(cast) => untyped_literal(cast),
        _ => None,
    }
}

/// A string written after a date-time type name with parameters, such as `TIMESTAMP_NTZ(3) '...'`.
fn untyped_literal(cast: &Cast) -> Option<(&'static str, String)> {
    let Expression::Literal(literal) = &cast.this else {
        return None;
    };
    if !cast.typed_literal || !matches!(literal.as_ref(), Literal::String(_)) {
        return None;
    }
    let (written, function) = parameterized_date_time(&cast.to)?;
    Some((
        "unsupported_type_literal",
        format!(
            "Snowflake has no {written} literal; cast the string instead, as '...'::{written} or {function}('...')"
        ),
    ))
}

/// A date-time type with a precision in Snowflake's spelling, and the function to suggest, or none.
fn parameterized_date_time(data_type: &DataType) -> Option<(String, &'static str)> {
    match data_type {
        DataType::Timestamp {
            precision: Some(precision),
            timezone: true,
        } => Some((format!("TIMESTAMP_TZ({precision})"), "TO_TIMESTAMP_TZ")),
        DataType::Timestamp {
            precision: Some(precision),
            timezone: false,
        } => Some((format!("TIMESTAMP({precision})"), "TO_TIMESTAMP")),
        DataType::Time {
            precision: Some(precision),
            ..
        } => Some((format!("TIME({precision})"), "TO_TIME")),
        DataType::Custom { name } => {
            let written: String = name.split_whitespace().collect();
            let (base, _) = written.split_once('(')?;
            let (_, function) = SNOWFLAKE_UNTYPED_LITERAL_TYPES
                .iter()
                .chain(SNOWFLAKE_UNPARAMETERIZED_LITERAL_TYPES)
                .find(|(listed, _)| listed.eq_ignore_ascii_case(base))?;
            Some((written, function))
        }
        _ => None,
    }
}

/// A REPLACE or RENAME name the ILIKE pattern removes ignoring case; backslashes and non-ASCII skip.
fn filtered_out_name(star: &Star) -> Option<&str> {
    let pattern = star
        .ilike
        .as_deref()
        .filter(|pattern| !pattern.contains('\\'))?;
    let replaced = star.replace.iter().flatten().map(|alias| &alias.alias);
    let renamed = star.rename.iter().flatten().map(|(old, _)| old);
    replaced
        .chain(renamed)
        .map(|identifier| identifier.name.as_str())
        .find(|name| name.is_ascii() && pattern.is_ascii() && !ilike_matches(pattern, name))
}

/// Snowflake ILIKE over ASCII text: `%` is any run, `_` one character, and case is ignored.
fn ilike_matches(pattern: &str, text: &str) -> bool {
    let (pattern, text) = (pattern.as_bytes(), text.as_bytes());
    let (mut p, mut t, mut resume) = (0, 0, None);
    while t < text.len() {
        match pattern.get(p) {
            Some(b'%') => {
                resume = Some((p, t));
                p += 1;
            }
            Some(&wanted) if wanted == b'_' || wanted.eq_ignore_ascii_case(&text[t]) => {
                p += 1;
                t += 1;
            }
            _ => match resume {
                Some((star, from)) => {
                    p = star + 1;
                    t = from + 1;
                    resume = Some((star, from + 1));
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|&byte| byte == b'%')
}

/// The rejection of a set operation with ALL, which Snowflake supports only for UNION.
fn set_operation_all(operator: &str) -> (&'static str, String) {
    (
        "set_operation_all",
        format!("Snowflake does not support {operator} with ALL"),
    )
}

/// Whether the expression calls a window function outside any nested query.
fn has_window(expression: &Expression) -> bool {
    match expression {
        Expression::WindowFunction(_) => true,
        Expression::Select(_)
        | Expression::Subquery(_)
        | Expression::Union(_)
        | Expression::Intersect(_)
        | Expression::Except(_) => false,
        _ => expression.children().into_iter().any(has_window),
    }
}
