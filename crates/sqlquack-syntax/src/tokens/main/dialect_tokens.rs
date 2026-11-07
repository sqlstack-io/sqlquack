//! `dialect_tokens`: the tokens the parser reads for `sql`, each with the comments attached to it.

use crate::tokens::models::Dialect;
use crate::tokens::models::Token;
use crate::tokens::types::Result;

/// The tokens the parser reads for `sql`, each with the comments attached to it.
pub fn dialect_tokens(sql: &str, dialect: Dialect) -> Result<Vec<Token>> {
    crate::parser::models::Dialect::get(dialect.dialect_type())
        .tokenizer
        .tokenize(sql)
}
