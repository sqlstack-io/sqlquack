//! `scan`: the tokens the grammar reads, ending in a fault token where scanning stopped.

use sqlquack_syntax::tokens::models::Span;

use crate::scan::_helpers::locations::locate;
use crate::scan::_helpers::scanner::{Scanned, Scanner};
use crate::scan::models::{Token, TokenKind};

/// Every token of `sql`; a lexical fault ends the list as a `Fault` token the parser reaches in turn.
pub fn scan(sql: &str) -> Vec<Token> {
    let Scanned {
        mut tokens, fault, ..
    } = Scanner::new(sql).scan();
    if let Some(fault) = fault {
        tokens.push(Token {
            kind: TokenKind::Fault(fault.code),
            value: fault.message,
            start: fault.position,
            end: fault.position,
            span: Span::default(),
        });
    }
    locate(sql, &mut tokens);
    tokens
}
