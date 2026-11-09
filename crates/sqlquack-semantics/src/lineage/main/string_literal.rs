//! `string_literal`: whether an expression is a string literal, through aliases and parentheses.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;

/// Whether `expression` is a string literal, past aliases, parentheses and comments.
pub(crate) fn string_literal(expression: &Expression) -> bool {
    match expression {
        Expression::Literal(literal) => matches!(literal.as_ref(), Literal::String(_)),
        Expression::Alias(alias) => string_literal(&alias.this),
        Expression::Paren(paren) => string_literal(&paren.this),
        Expression::Annotated(annotated)
            if annotated.type_assertion.is_none() && annotated.type_like.is_none() =>
        {
            string_literal(&annotated.this)
        }
        _ => false,
    }
}
