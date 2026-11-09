//! `is_keyword`: whether the tokenizer typed a word as a keyword.

use sqlquack_syntax::tokens::models::TokenType;

use crate::dialects::main::is_unquoted_word::is_unquoted_word;
use crate::dialects::models::Lexeme;

/// Whether the tokenizer types this unquoted word as a keyword rather than a name.
pub(crate) fn is_keyword(lexeme: &Lexeme) -> bool {
    is_unquoted_word(&lexeme.raw)
        && !matches!(
            lexeme.kind,
            TokenType::Var | TokenType::Identifier | TokenType::QuotedIdentifier
        )
}
