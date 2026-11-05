//! Reaching the [`BinaryOp`] inside an expression.

use crate::expressions::operators::models::BinaryOp;
use crate::expressions::tree::models::Expression;

/// The operands of every expression variant that wraps a [`BinaryOp`].
pub(crate) fn binary_op_mut(expr: &mut Expression) -> Option<&mut BinaryOp> {
    match expr {
        Expression::And(op)
        | Expression::Or(op)
        | Expression::Add(op)
        | Expression::Sub(op)
        | Expression::Mul(op)
        | Expression::Div(op)
        | Expression::Mod(op)
        | Expression::Eq(op)
        | Expression::Neq(op)
        | Expression::Lt(op)
        | Expression::Lte(op)
        | Expression::Gt(op)
        | Expression::Gte(op)
        | Expression::Match(op)
        | Expression::BitwiseAnd(op)
        | Expression::BitwiseOr(op)
        | Expression::BitwiseXor(op)
        | Expression::Concat(op)
        | Expression::Adjacent(op)
        | Expression::TsMatch(op)
        | Expression::PropertyEQ(op)
        | Expression::ArrayContainsAll(op)
        | Expression::ArrayContainedBy(op)
        | Expression::ArrayOverlaps(op)
        | Expression::JSONBContainsAllTopKeys(op)
        | Expression::JSONBContainsAnyTopKeys(op)
        | Expression::JSONBDeleteAtPath(op)
        | Expression::ExtendsLeft(op)
        | Expression::ExtendsRight(op)
        | Expression::Is(op)
        | Expression::MemberOf(op)
        | Expression::NullSafeEq(op)
        | Expression::NullSafeNeq(op)
        | Expression::Glob(op)
        | Expression::BitwiseLeftShift(op)
        | Expression::BitwiseRightShift(op) => Some(op),
        _ => None,
    }
}
