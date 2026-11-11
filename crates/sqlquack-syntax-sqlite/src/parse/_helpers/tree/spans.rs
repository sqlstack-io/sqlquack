//! Source ranges on nodes, recorded as a located parse of the shared parser records them.

use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::{SourceSpan, Span};

use crate::parse::_helpers::tree::cursor::Parser;

/// The span field of `expression`'s node, when its kind has one.
fn span_field(expression: &mut Expression) -> Option<&mut Option<Span>> {
    match expression {
        Expression::Column(node) => Some(&mut node.span),
        Expression::Identifier(node) => Some(&mut node.span),
        Expression::Star(node) => Some(&mut node.span.0),
        Expression::Count(node) => Some(&mut node.span),
        Expression::Function(node) => Some(&mut node.span.0),
        Expression::AggregateFunction(node) => Some(&mut node.span.0),
        Expression::WindowFunction(node) => Some(&mut node.span.0),
        Expression::Alias(node) => Some(&mut node.span.0),
        Expression::Cast(node) | Expression::SafeCast(node) | Expression::TryCast(node) => {
            Some(&mut node.span.0)
        }
        Expression::Case(node) => Some(&mut node.span.0),
        Expression::Like(node) | Expression::ILike(node) => Some(&mut node.span.0),
        Expression::Not(node)
        | Expression::Neg(node)
        | Expression::Pos(node)
        | Expression::BitwiseNot(node) => Some(&mut node.span.0),
        Expression::In(node) => Some(&mut node.span.0),
        Expression::Between(node) => Some(&mut node.span.0),
        Expression::IsNull(node) => Some(&mut node.span.0),
        Expression::Exists(node) => Some(&mut node.span.0),
        Expression::Paren(node) => Some(&mut node.span.0),
        Expression::Subquery(node) => Some(&mut node.span.0),
        Expression::Lateral(node) => Some(&mut node.span.0),
        Expression::Select(node) => Some(&mut node.span.0),
        Expression::Union(node) => Some(&mut node.span.0),
        Expression::Intersect(node) => Some(&mut node.span.0),
        Expression::Except(node) => Some(&mut node.span.0),
        Expression::Add(node)
        | Expression::Adjacent(node)
        | Expression::And(node)
        | Expression::ArrayContainedBy(node)
        | Expression::ArrayContainsAll(node)
        | Expression::ArrayOverlaps(node)
        | Expression::BitwiseAnd(node)
        | Expression::BitwiseLeftShift(node)
        | Expression::BitwiseOr(node)
        | Expression::BitwiseRightShift(node)
        | Expression::BitwiseXor(node)
        | Expression::Concat(node)
        | Expression::Div(node)
        | Expression::Eq(node)
        | Expression::ExtendsLeft(node)
        | Expression::ExtendsRight(node)
        | Expression::Glob(node)
        | Expression::Gt(node)
        | Expression::Gte(node)
        | Expression::Is(node)
        | Expression::JSONBContainsAllTopKeys(node)
        | Expression::JSONBContainsAnyTopKeys(node)
        | Expression::JSONBDeleteAtPath(node)
        | Expression::Lt(node)
        | Expression::Lte(node)
        | Expression::Match(node)
        | Expression::MemberOf(node)
        | Expression::Mod(node)
        | Expression::Mul(node)
        | Expression::Neq(node)
        | Expression::NullSafeEq(node)
        | Expression::NullSafeNeq(node)
        | Expression::Or(node)
        | Expression::PropertyEQ(node)
        | Expression::Sub(node)
        | Expression::TsMatch(node) => Some(&mut node.span.0),
        _ => None,
    }
}

impl Parser<'_> {
    /// The range from token `first` to the last consumed token; none when nothing was consumed.
    pub(crate) fn source_range(&self, first: usize) -> SourceSpan {
        let range: Option<Span> = match (self.tokens.get(first), self.at.checked_sub(1)) {
            (Some(from), Some(last)) if last >= first => self
                .tokens
                .get(last)
                .map(|to| Span::new(from.span.start, to.span.end, to.span.line, to.span.column)),
            _ => None,
        };
        SourceSpan(range)
    }

    /// `expression` with the range from token `first` here, replacing any its node records.
    pub(crate) fn respanned(&self, first: usize, mut expression: Expression) -> Expression {
        if let Some(field) = span_field(&mut expression) {
            *field = self.source_range(first).0;
        }
        expression
    }

    /// `expression` with the range from token `first` here, unless its node records one already.
    pub(crate) fn spanned(&self, first: usize, mut expression: Expression) -> Expression {
        if let (Some(field), SourceSpan(Some(range))) =
            (span_field(&mut expression), self.source_range(first))
            && field.is_none()
        {
            *field = Some(range);
        }
        expression
    }
}
