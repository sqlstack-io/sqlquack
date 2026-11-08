//! `tokenize`: lossless tokens of a Transact-SQL input, typed as the shared tokenizer types them.

use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{LosslessToken, TokenKind as LosslessKind};

use crate::scan::_helpers::scanner::{Scanned, Scanner, TriviaKind};
use crate::scan::errors::ScanError;
use crate::scan::models::{NumberKind, Token, TokenKind};

/// Every byte of `sql` as a token, blank run or comment, in order; the lexer's fault when a run never closes.
pub fn tokenize(sql: &str) -> Result<Vec<LosslessToken>, ScanError> {
    let Scanned {
        tokens,
        fault,
        trivia,
    } = Scanner::new(sql).scan();
    if let Some(fault) = fault {
        return Err(fault);
    }
    let mut out: Vec<LosslessToken> = tokens
        .iter()
        .map(|token| LosslessToken {
            kind: LosslessKind::Significant(token_type(token)),
            start: token.start,
            end: token.end,
        })
        .chain(trivia.iter().map(|trivium| LosslessToken {
            kind: match trivium.kind {
                TriviaKind::Blank => LosslessKind::Whitespace,
                TriviaKind::LineComment => LosslessKind::LineComment,
                TriviaKind::BlockComment => LosslessKind::BlockComment,
            },
            start: trivium.start,
            end: trivium.end,
        }))
        .collect();
    out.sort_unstable_by_key(|token| token.start);
    Ok(out)
}

/// The shared token type of a scanned token.
fn token_type(token: &Token) -> TokenType {
    match token.kind {
        TokenKind::Word => TokenType::Var,
        TokenKind::Variable => TokenType::Parameter,
        TokenKind::Quoted(_) => TokenType::QuotedIdentifier,
        TokenKind::String { national: false } => TokenType::String,
        TokenKind::String { national: true } => TokenType::NationalString,
        TokenKind::Number(NumberKind::Binary) => TokenType::HexString,
        TokenKind::Number(_) => TokenType::Number,
        TokenKind::Op => match token.value.as_str() {
            "<=" => TokenType::Lte,
            ">=" => TokenType::Gte,
            "<>" | "!=" => TokenType::Neq,
            "::" => TokenType::DColon,
            _ => TokenType::Operator,
        },
        TokenKind::Char(c) => match c {
            '(' => TokenType::LParen,
            ')' => TokenType::RParen,
            ',' => TokenType::Comma,
            ';' => TokenType::Semicolon,
            '.' => TokenType::Dot,
            '+' => TokenType::Plus,
            '-' => TokenType::Dash,
            '*' => TokenType::Star,
            '/' => TokenType::Slash,
            '%' => TokenType::Percent,
            '&' => TokenType::Amp,
            '|' => TokenType::Pipe,
            '^' => TokenType::Caret,
            '~' => TokenType::Tilde,
            '=' => TokenType::Eq,
            '<' => TokenType::Lt,
            '>' => TokenType::Gt,
            '!' => TokenType::Exclamation,
            '{' => TokenType::LBrace,
            '}' => TokenType::RBrace,
            ':' => TokenType::Colon,
            _ => TokenType::Unknown,
        },
        TokenKind::Fault(_) => TokenType::Unknown,
    }
}
