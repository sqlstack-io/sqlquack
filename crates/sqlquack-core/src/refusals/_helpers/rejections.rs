//! The constructs Snowflake parses but then rejects, recognised one node at a time.

use sqlquack_syntax::expressions::names::models::Star;
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
