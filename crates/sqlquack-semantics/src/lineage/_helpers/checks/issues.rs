//! Findings of the checks: where polyglot locates them, and their codes.

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::tokens::models::Span;

use crate::lineage::models::NameFinding;

/// A finding about `node`, at `span`.
pub(crate) fn issue(
    (code, phase): (&'static str, u8),
    message: String,
    span: Option<Span>,
    node: &Expression,
) -> NameFinding {
    NameFinding {
        code,
        message,
        span,
        written: String::new(),
        phase,
        node: std::ptr::from_ref(node) as usize,
    }
}

/// A type finding's place: the first column in `node`'s own scope, as polyglot's calls have none.
pub(crate) fn type_location(node: &Expression) -> Option<Span> {
    in_scope(node).into_iter().find_map(|node| match node {
        Expression::Column(column) => column.span.or(column.name.span),
        _ => None,
    })
}

/// A scope rule's place: the node itself, else the first column, name, star, count or function.
pub(crate) fn rule_location(node: &Expression) -> Option<Span> {
    own_location(node).or_else(|| in_scope(node).into_iter().find_map(own_location))
}

fn own_location(node: &Expression) -> Option<Span> {
    match node {
        Expression::Column(column) => column.span.or(column.name.span),
        Expression::Identifier(identifier) => identifier.span,
        Expression::Star(star) => star.span.0,
        Expression::Count(count) => count.span,
        Expression::Function(function) => function.span.0,
        _ => None,
    }
}

/// `node` and what it reads in its own scope, in pre-order, not entering queries it holds.
pub(crate) fn in_scope(node: &Expression) -> Vec<&Expression> {
    let mut visited: Vec<&Expression> = Vec::new();
    let mut pending: Vec<&Expression> = vec![node];
    while let Some(current) = pending.pop() {
        let nested: bool = !std::ptr::eq(current, node) && query_boundary(current);
        if nested {
            continue;
        }
        visited.push(current);
        let mut children: Vec<&Expression> = current.children();
        children.reverse();
        pending.extend(children);
    }
    visited
}

/// Whether `node` starts a query of its own: a SELECT, set operation, CTE or derived table.
pub(crate) fn query_boundary(node: &Expression) -> bool {
    match node {
        Expression::Select(_)
        | Expression::Union(_)
        | Expression::Intersect(_)
        | Expression::Except(_)
        | Expression::Cte(_) => true,
        Expression::Subquery(subquery) => subquery.alias.is_some(),
        _ => false,
    }
}
