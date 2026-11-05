//! Guard: ast.

use crate::expressions::tree::models::Expression;
use crate::guard::_helpers::codes::parse_guard_error;
use crate::guard::models::AstStats;
use crate::guard::models::ComplexityGuardOptions;
use crate::guard::models::ConnectorKind;
use crate::tokens::types::Result;

pub(crate) fn enforce_ast_depth_only(
    expressions: &[Expression],
    options: &ComplexityGuardOptions,
) -> Result<()> {
    let Some(max_depth) = options.max_ast_depth else {
        return Ok(());
    };

    let stats = ast_stats(expressions, None, Some(max_depth))?;
    if stats.max_depth > max_depth {
        return Err(parse_guard_error(
            "E_GUARD_AST_DEPTH_EXCEEDED",
            stats.max_depth,
            max_depth,
            None,
        ));
    }

    Ok(())
}

pub(crate) fn ast_stats(
    expressions: &[Expression],
    max_nodes: Option<usize>,
    max_depth: Option<usize>,
) -> Result<AstStats> {
    let mut stats = AstStats::default();
    let mut stack: Vec<(&Expression, usize)> = expressions.iter().rev().map(|e| (e, 0)).collect();

    while let Some((expr, depth)) = stack.pop() {
        stats.node_count += 1;
        stats.max_depth = stats.max_depth.max(depth);

        if let Some(max) = max_nodes {
            if stats.node_count > max {
                return Ok(stats);
            }
        }

        if let Some(max) = max_depth {
            if stats.max_depth > max {
                return Ok(stats);
            }
        }

        stack = push_ast_children(stack, expr, depth);
    }

    Ok(stats)
}

pub(crate) fn push_ast_children<'a>(
    mut stack: Vec<(&'a Expression, usize)>,
    expr: &'a Expression,
    depth: usize,
) -> Vec<(&'a Expression, usize)> {
    match expr {
        Expression::And(op) if is_commentless_binary_op(op) => {
            stack = push_connector_child(stack, &op.right, depth, ConnectorKind::And);
            stack = push_connector_child(stack, &op.left, depth, ConnectorKind::And);
        }
        Expression::Or(op) if is_commentless_binary_op(op) => {
            stack = push_connector_child(stack, &op.right, depth, ConnectorKind::Or);
            stack = push_connector_child(stack, &op.left, depth, ConnectorKind::Or);
        }
        _ => match chain_operator(expr).filter(|op| is_commentless_binary_op(op)) {
            Some(op) => {
                stack.push((&op.right, depth + 1));
                let left_depth = match chain_operator(&op.left) {
                    Some(left) if is_commentless_binary_op(left) => depth,
                    _ => depth + 1,
                };
                stack.push((&op.left, left_depth));
            }
            None => {
                let child_start = stack.len();
                crate::traversal::main::for_each_child::for_each_child(expr, |_, child| {
                    stack.push((child, depth + 1));
                });
                stack[child_start..].reverse();
            }
        },
    }
    stack
}

/// A left-associative operator whose flat chain, such as `1 + 1 + 1`, is one level, not nesting.
pub(crate) fn chain_operator(
    expr: &Expression,
) -> Option<&crate::expressions::operators::models::BinaryOp> {
    match expr {
        Expression::Add(op)
        | Expression::Sub(op)
        | Expression::Mul(op)
        | Expression::Div(op)
        | Expression::Mod(op)
        | Expression::Concat(op)
        | Expression::BitwiseAnd(op)
        | Expression::BitwiseOr(op)
        | Expression::BitwiseXor(op)
        | Expression::BitwiseLeftShift(op)
        | Expression::BitwiseRightShift(op) => Some(op),
        _ => None,
    }
}

pub(crate) fn push_connector_child<'a>(
    mut stack: Vec<(&'a Expression, usize)>,
    child: &'a Expression,
    depth: usize,
    kind: ConnectorKind,
) -> Vec<(&'a Expression, usize)> {
    let child_depth = if is_commentless_connector(child, kind) {
        depth
    } else {
        depth + 1
    };
    stack.push((child, child_depth));
    stack
}

pub(crate) fn is_commentless_connector(expr: &Expression, kind: ConnectorKind) -> bool {
    match (kind, expr) {
        (ConnectorKind::And, Expression::And(op)) | (ConnectorKind::Or, Expression::Or(op)) => {
            is_commentless_binary_op(op)
        }
        _ => false,
    }
}

pub(crate) fn is_commentless_binary_op(
    op: &crate::expressions::operators::models::BinaryOp,
) -> bool {
    op.left_comments.is_empty()
        && op.operator_comments.is_empty()
        && op.trailing_comments.is_empty()
}
