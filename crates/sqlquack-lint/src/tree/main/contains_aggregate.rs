//! `contains_aggregate`: whether a node or any node below it is an aggregate call.

use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::tree::main::is_aggregate::is_aggregate;

/// Whether `expression` or any node below it is an aggregate call.
pub(crate) fn contains_aggregate(expression: &Expression) -> bool {
    expression.contains(is_aggregate)
}
