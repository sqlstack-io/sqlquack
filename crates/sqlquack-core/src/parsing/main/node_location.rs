//! `node_location`: where a parsed node is in its source.

use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::parsing::models::{SourceLines, SourceLocation};

/// The node's own recorded range, else the range covering the ranges recorded inside it.
pub fn node_location(node: &Expression, lines: &SourceLines) -> Option<SourceLocation> {
    if let Some(span) = node.source_span() {
        return Some(lines.location(span.start, span.end));
    }
    let mut cover: Option<(usize, usize)> = None;
    for descendant in node.dfs() {
        if let Some(span) = descendant.source_span() {
            cover = Some(match cover {
                Some((start, end)) => (start.min(span.start), end.max(span.end)),
                None => (span.start, span.end),
            });
        }
    }
    cover.map(|(start, end)| lines.location(start, end))
}
