//! The significant tokens and comments of SQL, read from the lossless tokenizer.

use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{TokenKind, tokenize};

use crate::format::_helpers::bigquery::lex_bigquery;
use crate::format::_helpers::sqlserver::lex_sqlserver;
use crate::format::constants::{TOKENIZE_FAILURE, UNSUPPORTED_DIALECT_FAILURE};
use crate::format::models::FormatDialect;

/// One significant token with its byte range and source text.
#[derive(Debug, Clone)]
pub(crate) struct Lexeme {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: TokenType,
    pub(crate) raw: String,
}

/// A comment between significant tokens.
#[derive(Debug, Clone)]
pub(crate) struct Comment {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: String,
    /// A comment that runs to the end of its line.
    pub(crate) line: bool,
    /// Only whitespace precedes the comment on its line.
    pub(crate) leading: bool,
}

/// The significant tokens and the comments of `sql`.
pub(crate) fn lex(
    sql: &str,
    dialect: FormatDialect,
) -> Result<(Vec<Lexeme>, Vec<Comment>), String> {
    let Some(shared) = dialect.shared() else {
        return match dialect {
            FormatDialect::BigQuery => lex_bigquery(sql),
            FormatDialect::SqlServer => lex_sqlserver(sql),
            _ => Err(UNSUPPORTED_DIALECT_FAILURE.to_string()),
        };
    };
    let tokens = tokenize(sql, shared).map_err(|error| format!("{TOKENIZE_FAILURE}: {error}"))?;
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
    Ok((lexemes, comments))
}

/// The comment of `sql` from `start` to `end`; `line` when it runs to the end of its line.
pub(crate) fn comment(sql: &str, start: usize, end: usize, line: bool) -> Comment {
    let line_start = sql[..start].rfind('\n').map_or(0, |at| at + 1);
    Comment {
        start,
        end,
        text: sql[start..end].trim_end_matches(['\r', '\n']).to_string(),
        line,
        leading: sql[line_start..start].trim().is_empty(),
    }
}

/// Whether `text` is an unquoted ASCII word whose letter case carries no meaning.
pub(crate) fn is_unquoted_word(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '$'))
}

/// Whether the tokenizer types this unquoted word as a keyword rather than a name.
pub(crate) fn is_keyword(lexeme: &Lexeme) -> bool {
    is_unquoted_word(&lexeme.raw)
        && !matches!(
            lexeme.kind,
            TokenType::Var | TokenType::Identifier | TokenType::QuotedIdentifier
        )
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
