//! `scan`: the significant tokens the parser reads, up to and including the first lexical error.

use crate::scan::main::lex::lex;
use crate::scan::models::Token;

/// The tokens of `sql` the parser reads: no blanks or comments, ending at the first error token.
pub fn scan(sql: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    for token in lex(sql) {
        if !token.kind.is_significant() {
            continue;
        }
        let error: bool = token.kind.is_error();
        tokens.push(token);
        if error {
            break;
        }
    }
    tokens
}
