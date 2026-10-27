//! `identifier_from_token`: construct an identifier directly from its consumed source token.

use crate::expressions::names::models::Identifier;
use crate::tokens::models::Token;

/// Construct an identifier directly from its consumed source token.
pub(crate) fn identifier_from_token(token: Token, quoted: bool) -> Identifier {
    Identifier {
        name: token.text,
        quoted,
        trailing_comments: Vec::new(),
        span: Some(token.span),
    }
}
