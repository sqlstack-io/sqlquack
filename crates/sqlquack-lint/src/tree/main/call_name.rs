//! `call_name`: the name of a call the parser kept as written, such as Snowflake's `ROW_NUMBER()`.

use sqlquack_syntax::Expression;

/// The name of an unqualified call the parser kept as written; `None` for any other node.
pub(crate) fn call_name(expression: &Expression) -> Option<&str> {
    match expression {
        Expression::Function(function) if function.qualified_name.is_empty() => {
            Some(function.name.as_str())
        }
        Expression::AggregateFunction(function) => Some(function.name.as_str()),
        _ => None,
    }
}
