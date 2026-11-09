//! `shared_type`: the type the shared tokenizer gives one token's text.

use sqlquack_parse::tokenize;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, TokenKind};

/// The type the shared tokenizer, reading `dialect`, gives `text` when it reads it as exactly one token.
pub(crate) fn shared_type(text: &str, dialect: Dialect) -> Option<TokenType> {
    let Ok(tokens) = tokenize(text, dialect) else {
        return None;
    };
    match tokens.as_slice() {
        [token] if token.end - token.start == text.len() => match token.kind {
            TokenKind::Significant(kind) => Some(kind),
            _ => None,
        },
        _ => None,
    }
}
