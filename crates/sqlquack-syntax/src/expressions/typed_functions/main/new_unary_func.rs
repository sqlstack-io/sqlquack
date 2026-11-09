//! `new_unary_func`: create a new UnaryFunc with no original_name.

use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::models::UnaryFunc;

/// Create a new UnaryFunc with no original_name
pub fn new_unary_func(this: Expression) -> UnaryFunc {
    UnaryFunc {
        this,
        original_name: None,
        inferred_type: None,
    }
}
