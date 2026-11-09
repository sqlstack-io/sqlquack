//! Operators: boolean chains.

use crate::expressions::names::models::Null;
use crate::expressions::operators::constants::BALANCED_CHAIN_MAX_TERMS;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::tree::models::Expression;

pub(crate) fn should_rebalance_boolean_chain(expr: &Expression, is_and: bool) -> bool {
    let mut leaf_count = 0usize;
    let mut stack = vec![expr];

    while let Some(node) = stack.pop() {
        match (is_and, node) {
            (true, Expression::And(op)) => {
                if !op.left_comments.is_empty()
                    || !op.operator_comments.is_empty()
                    || !op.trailing_comments.is_empty()
                {
                    return false;
                }
                stack.push(&op.right);
                stack.push(&op.left);
            }
            (false, Expression::Or(op)) => {
                if !op.left_comments.is_empty()
                    || !op.operator_comments.is_empty()
                    || !op.trailing_comments.is_empty()
                {
                    return false;
                }
                stack.push(&op.right);
                stack.push(&op.left);
            }
            _ => leaf_count += 1,
        }
    }

    leaf_count > BALANCED_CHAIN_MAX_TERMS
}

pub(crate) fn flatten_boolean_terms_owned(expr: Expression, is_and: bool) -> Vec<Expression> {
    let mut terms: Vec<Expression> = Vec::new();
    let mut stack = vec![expr];

    while let Some(node) = stack.pop() {
        match (is_and, node) {
            (true, Expression::And(mut op)) | (false, Expression::Or(mut op)) => {
                stack.push(std::mem::replace(&mut op.right, Expression::Null(Null)));
                stack.push(std::mem::replace(&mut op.left, Expression::Null(Null)));
            }
            (_, other) => terms.push(other),
        }
    }

    terms
}

pub(crate) fn build_balanced_boolean_tree(mut terms: Vec<Expression>, is_and: bool) -> Expression {
    if terms.is_empty() {
        return Expression::Null(Null);
    }

    while terms.len() > 1 {
        let mut next = Vec::with_capacity(terms.len().div_ceil(2));
        let mut iter = terms.into_iter();

        while let Some(left) = iter.next() {
            if let Some(right) = iter.next() {
                let combined = if is_and {
                    Expression::And(Box::new(BinaryOp::new(left, right)))
                } else {
                    Expression::Or(Box::new(BinaryOp::new(left, right)))
                };
                next.push(combined);
            } else {
                next.push(left);
            }
        }

        terms = next;
    }

    terms.pop().unwrap_or(Expression::Null(Null))
}
