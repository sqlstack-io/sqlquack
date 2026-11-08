//! `is_row_number`: whether a node is `ROW_NUMBER()`, as a typed node or as written.

use sqlquack_syntax::Expression;

use crate::tree::main::call_name::call_name;

/// Whether `expression` is `ROW_NUMBER()`, as a typed node or as written.
pub(crate) fn is_row_number(expression: &Expression) -> bool {
    matches!(expression, Expression::RowNumber(_))
        || call_name(expression).is_some_and(|name| name.eq_ignore_ascii_case("ROW_NUMBER"))
}
