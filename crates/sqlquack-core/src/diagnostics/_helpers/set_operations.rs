//! How long a statement's set-operation chain is.

use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, TokenKind, tokenize};

/// How many UNION, INTERSECT, EXCEPT and MINUS operators the source holds.
pub(crate) fn set_operations(source: &str) -> usize {
    tokenize(source, Dialect::Snowflake)
        .unwrap_or_default()
        .iter()
        .filter(|token| {
            matches!(
                token.kind,
                TokenKind::Significant(TokenType::Union | TokenType::Intersect | TokenType::Except)
            )
        })
        .count()
}
