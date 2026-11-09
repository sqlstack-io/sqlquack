//! `lex`: every token of the input in order, blanks, comments and lexical errors included.

use crate::scan::_helpers::lexer::Lexer;
use crate::scan::_helpers::locations::locate;
use crate::scan::models::Token;

/// Every token of `sql`, which together cover every byte; a byte no rule takes is an error token.
pub fn lex(sql: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Lexer::new(sql).tokens();
    locate(sql, &mut tokens);
    tokens
}
