//! `scan`: the tokens the grammar reads, a fault token where the tokenizer cannot read the text.

use crate::scan::_helpers::locations::locate;
use crate::scan::_helpers::scanner::Scanner;
use crate::scan::models::Token;

/// Every token of `sql` up to its first NUL byte; text SQLite cannot read is a `Fault` token.
pub fn scan(sql: &str) -> Vec<Token> {
    let (mut tokens, _) = Scanner::new(sql).scan();
    locate(sql, &mut tokens);
    tokens
}
