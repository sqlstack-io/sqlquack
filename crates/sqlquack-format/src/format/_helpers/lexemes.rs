//! The significant tokens and comments of SQL, read from the lossless tokenizer.

use sqlquack_parse::tokenize;
use sqlquack_syntax::TokenKind;
use sqlquack_syntax::tokens::models::TokenType;

use crate::dialects::main::comment::comment;
use crate::dialects::main::is_unquoted_word::is_unquoted_word;
use crate::dialects::models::{Comment, FormatProfile, Lexeme};
use crate::format::constants::TOKENIZE_FAILURE;

/// The significant tokens and the comments of `sql`, as `profile`'s dialect reads them.
pub(crate) fn lex(
    sql: &str,
    profile: &FormatProfile,
) -> Result<(Vec<Lexeme>, Vec<Comment>), String> {
    if let Some(lex_native) = profile.lex {
        return lex_native(sql);
    }
    let tokens =
        tokenize(sql, profile.dialect).map_err(|error| format!("{TOKENIZE_FAILURE}: {error}"))?;
    let mut lexemes: Vec<Lexeme> = Vec::with_capacity(tokens.len());
    let mut comments: Vec<Comment> = Vec::new();
    for token in &tokens {
        let text = token.text(sql);
        match token.kind {
            TokenKind::Significant(kind) => lexemes.push(Lexeme {
                start: token.start,
                end: token.end,
                kind,
                raw: text.to_string(),
            }),
            TokenKind::LineComment | TokenKind::BlockComment => comments.push(comment(
                sql,
                token.start,
                token.end,
                token.kind == TokenKind::LineComment,
            )),
            TokenKind::Whitespace => {}
        }
    }
    match profile.keywords {
        Some(keywords) => Ok((keywords(sql, lexemes)?, comments)),
        None => Ok((lexemes, comments)),
    }
}

/// Whether the token can end an operand, so a `-` or `[` after it is binary or a subscript.
pub(crate) fn ends_operand(lexeme: &Lexeme) -> bool {
    matches!(
        lexeme.kind,
        TokenType::Var
            | TokenType::Identifier
            | TokenType::QuotedIdentifier
            | TokenType::Number
            | TokenType::Parameter
            | TokenType::RParen
            | TokenType::RBracket
            | TokenType::RBrace
            | TokenType::Null
            | TokenType::True
            | TokenType::False
            | TokenType::End
    ) || is_string_literal(lexeme.kind)
        || (lexeme.kind == TokenType::Unknown && is_unquoted_word(&lexeme.raw))
}

/// Whether the token type is a string literal of any quoting.
pub(crate) fn is_string_literal(kind: TokenType) -> bool {
    matches!(
        kind,
        TokenType::String
            | TokenType::NationalString
            | TokenType::EscapeString
            | TokenType::RawString
            | TokenType::ByteString
            | TokenType::UnicodeString
            | TokenType::HexString
            | TokenType::BitString
            | TokenType::DollarString
            | TokenType::TripleSingleQuotedString
            | TokenType::TripleDoubleQuotedString
    )
}

/// Each token's bracket partner, for `(`, `[` and `{`.
pub(crate) fn bracket_partners(lexemes: &[Lexeme]) -> Vec<Option<usize>> {
    let mut partners: Vec<Option<usize>> = vec![None; lexemes.len()];
    let mut open: Vec<usize> = Vec::new();
    for (index, lexeme) in lexemes.iter().enumerate() {
        match lexeme.kind {
            TokenType::LParen | TokenType::LBracket | TokenType::LBrace => open.push(index),
            TokenType::RParen | TokenType::RBracket | TokenType::RBrace => {
                if let Some(opener) = open.pop() {
                    partners[opener] = Some(index);
                    partners[index] = Some(opener);
                }
            }
            _ => {}
        }
    }
    partners
}
