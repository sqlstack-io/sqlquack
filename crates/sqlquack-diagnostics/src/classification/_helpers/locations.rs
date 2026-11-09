//! Source positions for nodes, borrowed from neighbours when the parser kept none.

use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::Span;

use sqlquack_syntax::tokens::models::{SourceLines, SourceLocation};

pub(crate) fn own_span(node: &Expression) -> Option<Span> {
    match node {
        Expression::Column(column) => column.span.or(column.name.span),
        Expression::Table(table) => table.span.or(table.name.span),
        other => other.source_span(),
    }
}

pub(crate) fn resolve_locations(
    nodes: &[&Expression],
    parents: &[Option<usize>],
    index: &SourceLines,
) -> Vec<Option<SourceLocation>> {
    let count = nodes.len();
    let mut subtree_end: Vec<usize> = (1..=count).collect();
    for position in (0..count).rev() {
        if let Some(parent) = parents[position] {
            subtree_end[parent] = subtree_end[parent].max(subtree_end[position]);
        }
    }
    let spans: Vec<Option<Span>> = nodes.iter().map(|node| own_span(node)).collect();
    let mut next_span: Vec<Option<usize>> = vec![None; count + 1];
    for position in (0..count).rev() {
        next_span[position] = if spans[position].is_some() {
            Some(position)
        } else {
            next_span[position + 1]
        };
    }
    let mut locations: Vec<Option<SourceLocation>> = Vec::with_capacity(count);
    for position in 0..count {
        let inside = next_span[position]
            .filter(|found| *found < subtree_end[position])
            .and_then(|found| spans[found]);
        let location = match inside {
            Some(span) => Some(index.location(span.start, span.end)),
            None => parents[position].and_then(|parent| locations[parent]),
        };
        locations.push(location);
    }
    locations
}
