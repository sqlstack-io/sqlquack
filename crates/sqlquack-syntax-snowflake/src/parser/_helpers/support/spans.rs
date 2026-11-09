//! Parser support: the source ranges a located parse records on nodes.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::traversal::types::AstNode;

impl Parser {
    /// SQLQuack: `expr` with the range from token `start` to the last consumed token, when located.
    pub(crate) fn spanned(&self, start: usize, mut expr: Expression) -> Expression {
        if !self.config.record_node_spans || self.current <= start {
            return expr;
        }
        if let (Some(first), Some(last)) =
            (self.tokens.get(start), self.tokens.get(self.current - 1))
        {
            expr.fill_span(first.span.through(last.span));
        }
        expr
    }

    /// SQLQuack: the range from token `start` to the last consumed token, when located.
    pub(crate) fn source_range(&self, start: usize) -> SourceSpan {
        if !self.config.record_node_spans || self.current <= start {
            return SourceSpan::default();
        }
        match (self.tokens.get(start), self.tokens.get(self.current - 1)) {
            (Some(first), Some(last)) => SourceSpan(Some(first.span.through(last.span))),
            _ => SourceSpan::default(),
        }
    }
}
