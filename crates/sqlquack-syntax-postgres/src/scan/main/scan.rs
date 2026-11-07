//! `scan`: the tokens the grammar reads, ending in a fault token where scanning stopped.

use sqlquack_syntax::tokens::models::Span;

use crate::scan::_helpers::locations::locate;
use crate::scan::_helpers::scanner::{Fault, RawToken, Scanner, decode_unicode_token};
use crate::scan::constants::{UESCAPE, UESCAPE_FORBIDDEN};
use crate::scan::models::{Token, TokenKind};

fn fault_token(fault: Fault) -> Token {
    Token {
        kind: TokenKind::Fault(fault.syntax),
        value: fault.message,
        start: fault.position,
        end: fault.position,
        span: Span::default(),
    }
}

/// The escape character a `UESCAPE` string names, or the syntax error it is.
fn escape_character(raw: &RawToken) -> Result<char, Fault> {
    let simple: bool = raw.token.kind == TokenKind::String && raw.unicode.is_none();
    if !simple {
        return Err(Fault::syntax(
            "UESCAPE must be followed by a simple string literal",
            raw.token.start,
        ));
    }
    let mut characters = raw.token.value.chars();
    match (characters.next(), characters.next()) {
        (Some(escape), None) if !UESCAPE_FORBIDDEN.contains(escape) => Ok(escape),
        _ => Err(Fault::syntax(
            "invalid Unicode escape character",
            raw.token.start,
        )),
    }
}

/// Every token of `sql`; a lexical fault ends the list as a `Fault` token the parser reaches in turn.
pub fn scan(sql: &str) -> Vec<Token> {
    let (raws, mut fault) = Scanner::new(sql).scan();
    let mut tokens: Vec<Token> = Vec::with_capacity(raws.len() + 1);
    let mut raws = raws.into_iter().peekable();
    while let Some(raw) = raws.next() {
        if raw.unicode.is_none() {
            tokens.push(raw.token);
            continue;
        }
        let uescape: bool = raws.peek().is_some_and(|next| {
            next.token.kind != TokenKind::QuotedIdent && next.token.value == UESCAPE
        });
        let escape: Result<char, Fault> = match uescape {
            false => Ok('\\'),
            true => {
                raws.next();
                match raws.next() {
                    Some(string) => escape_character(&string),
                    None => Err(Fault::syntax(
                        "UESCAPE must be followed by a simple string literal",
                        sql.len(),
                    )),
                }
            }
        };
        match escape.and_then(|escape| decode_unicode_token(raw, escape)) {
            Ok(token) => tokens.push(token),
            Err(decoding) => {
                fault = Some(decoding);
                break;
            }
        }
    }
    if let Some(fault) = fault {
        tokens.push(fault_token(fault));
    }
    locate(sql, &mut tokens);
    tokens
}
