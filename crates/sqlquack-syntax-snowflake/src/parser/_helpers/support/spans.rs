//! Parser support: the source ranges a located parse records on nodes.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::queries::models::Intersect;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::{SourceSpan, Span};
use sqlquack_syntax::traversal::types::AstNode;

impl Parser {
    /// SQLQuack: `expr` with the range from token `start` to the last consumed token, when located.
    pub(crate) fn spanned(&self, start: usize, expr: Expression) -> Expression {
        if !self.config.record_node_spans {
            return expr;
        }
        self.filled_span(start, expr)
    }

    /// SQLQuack: a select item with its range when the parse records item ranges, which lineage reports.
    pub(crate) fn item_spanned(&self, start: usize, expr: Expression) -> Expression {
        if !self.config.record_item_spans {
            return expr;
        }
        self.filled_span(start, expr)
    }

    /// SQLQuack: `expr` with the range from token `start` to the last consumed token.
    fn filled_span(&self, start: usize, mut expr: Expression) -> Expression {
        if self.current <= start {
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

    /// SQLQuack: the range a set operation's operand records, looking through attached comments.
    pub(crate) fn operand_span(mut operand: &Expression) -> Option<Span> {
        while let Expression::Annotated(annotated) = operand {
            operand = &annotated.this;
        }
        operand.recorded_span()
    }

    /// SQLQuack: the range from `first` to the last consumed token, when located.
    pub(crate) fn range_from(&self, first: Option<Span>) -> SourceSpan {
        let last = self
            .current
            .checked_sub(1)
            .and_then(|at| self.tokens.get(at));
        match (self.config.record_node_spans, first, last) {
            (true, Some(first), Some(last)) => SourceSpan(Some(first.through(last.span))),
            _ => SourceSpan::default(),
        }
    }

    /// SQLQuack: a set operation's range, stretched over the modifiers parsed after its branches.
    pub(crate) fn extend_set_operation_span(&self, operation: &mut Expression) {
        let mut node = operation;
        while let Expression::Annotated(annotated) = node {
            node = &mut annotated.this;
        }
        let span = match node {
            Expression::Union(union) => &mut union.span,
            Expression::Intersect(intersect) => &mut intersect.span,
            Expression::Except(except) => &mut except.span,
            _ => return,
        };
        *span = self.range_from(span.0);
    }

    /// SQLQuack: the last branch's range, without the modifiers moved from it to its set operation.
    pub(crate) fn end_select_before_modifiers(&self, start: usize, operand: &mut Expression) {
        let end = self.select_modifiers_start;
        if !self.config.record_node_spans || end <= start || end > self.current {
            return;
        }
        let mut node = operand;
        while let Expression::Annotated(annotated) = node {
            node = &mut annotated.this;
        }
        if let (Expression::Select(select), Some(first), Some(last)) =
            (node, self.tokens.get(start), self.tokens.get(end - 1))
        {
            select.span = SourceSpan(Some(first.span.through(last.span)));
        }
    }

    /// SQLQuack: the (outer, INTERSECT) ranges after INTERSECT takes the outer's right operand.
    pub(crate) fn rebound_spans(
        intersect: &Intersect,
        outer: SourceSpan,
    ) -> (SourceSpan, SourceSpan) {
        let Some(whole) = intersect.span.0 else {
            return (outer, intersect.span);
        };
        let first = Self::operand_span(&intersect.left);
        (
            SourceSpan(outer.0.map(|span| span.through(whole))),
            SourceSpan(first.map(|span| span.through(whole))),
        )
    }
}
