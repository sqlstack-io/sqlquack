//! The constructs Snowflake parses but then rejects, recognised one node at a time.

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
        _ => None,
    }
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
