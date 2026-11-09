//! `highlight_sql`: DuckDB SQL styled for a terminal by the lossless tokenizer, every byte kept.

use sqlquack_diagnostics::diagnostics::models::Palette;
use sqlquack_syntax::lossless::models::{LosslessToken, TokenKind};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, tokenize};

/// `sql` with its keywords, strings, numbers, comments and identifiers styled; unreadable SQL as is.
pub fn highlight_sql(sql: &str, style: &Palette) -> String {
    let Ok(tokens) = tokenize(sql, Dialect::DuckDB) else {
        return sql.to_owned();
    };
    let mut highlighted = String::with_capacity(sql.len());
    let mut written = 0;
    for token in &tokens {
        highlighted.push_str(&sql[written..token.start]);
        let text = token.text(sql);
        match token_style(token, text, style) {
            "" => highlighted.push_str(text),
            styled => {
                highlighted.push_str(styled);
                highlighted.push_str(text);
                highlighted.push_str(style.reset);
            }
        }
        written = token.end;
    }
    highlighted.push_str(&sql[written..]);
    highlighted
}

/// The style of `token`, whose text is `text`; empty for whitespace and punctuation.
fn token_style(token: &LosslessToken, text: &str, style: &Palette) -> &'static str {
    match token.kind {
        TokenKind::Whitespace => "",
        TokenKind::LineComment | TokenKind::BlockComment => style.comment,
        TokenKind::Significant(kind) => match kind {
            TokenType::String
            | TokenType::DollarString
            | TokenType::TripleDoubleQuotedString
            | TokenType::TripleSingleQuotedString
            | TokenType::BitString
            | TokenType::HexString
            | TokenType::ByteString
            | TokenType::NationalString
            | TokenType::EscapeString
            | TokenType::RawString
            | TokenType::HeredocString
            | TokenType::HeredocStringAlternative
            | TokenType::UnicodeString => style.string,
            TokenType::Number | TokenType::HexNumber => style.number,
            TokenType::Identifier | TokenType::QuotedIdentifier | TokenType::Var => {
                style.identifier
            }
            _ if is_word(text) => style.keyword,
            _ => "",
        },
    }
}

/// Whether `text` is a word, as every keyword and type name is, rather than punctuation.
fn is_word(text: &str) -> bool {
    text.starts_with(|character: char| character.is_ascii_alphabetic())
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}
