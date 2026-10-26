//! Source positions for nodes, borrowed from neighbours when the parser kept none.

use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::Span;

use crate::parsing::models::SourceLocation;

pub(crate) struct LineIndex {
    line_starts: Vec<usize>,
    length: usize,
}

impl LineIndex {
    pub(crate) fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        let mut length = 0;
        for (offset, character) in source.chars().enumerate() {
            if character == '\n' {
                line_starts.push(offset + 1);
            }
            length = offset + 1;
        }
        Self {
            line_starts,
            length,
        }
    }

    pub(crate) fn location(&self, span: Span) -> SourceLocation {
        if span.start > self.length {
            return SourceLocation {
                start: span.start,
                end: span.end,
                line: span.line,
                column: span.column,
            };
        }
        let line_index = self
            .line_starts
            .partition_point(|line_start| *line_start <= span.start)
            .saturating_sub(1);
        SourceLocation {
            start: span.start,
            end: span.end,
            line: line_index + 1,
            column: span.start - self.line_starts[line_index] + 1,
        }
    }
}

pub(crate) fn own_span(node: &Expression) -> Option<Span> {
    match node {
        Expression::Identifier(identifier) => identifier.span,
        Expression::Column(column) => column.span.or(column.name.span),
        Expression::Table(table) => table.span.or(table.name.span),
        Expression::Star(star) => star.span,
        Expression::Function(function) => function.span,
        Expression::Count(count) => count.span,
        _ => None,
    }
}

pub(crate) fn resolve_locations(
    nodes: &[&Expression],
    parents: &[Option<usize>],
    index: &LineIndex,
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
            Some(span) => Some(index.location(span)),
            None => parents[position].and_then(|parent| locations[parent]),
        };
        locations.push(location);
    }
    locations
}
