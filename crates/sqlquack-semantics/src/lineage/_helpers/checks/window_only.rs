//! `named_window_only`: whether a call names one of a dialect's window-only functions.

use sqlquack_syntax::Expression;

use crate::lineage::_helpers::checks::signatures::dispatch_name;

/// Whether `expression` calls one of the window-only `functions` by name.
pub(crate) fn named_window_only(expression: &Expression, functions: &[&str]) -> bool {
    let name: String = match expression {
        Expression::Function(function) if !function.quoted => function
            .name
            .rsplit('.')
            .next()
            .unwrap_or(&function.name)
            .to_lowercase(),
        Expression::Function(function) => dispatch_name(&function.name),
        Expression::AggregateFunction(function) => dispatch_name(&function.name),
        _ => return false,
    };
    functions
        .iter()
        .any(|window| name.eq_ignore_ascii_case(window))
}
