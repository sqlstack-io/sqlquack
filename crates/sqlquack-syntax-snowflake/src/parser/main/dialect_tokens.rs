//! `dialect_tokens`: the tokens the parser reads for `sql`, each with the comments attached to it.

use sqlquack_syntax::tokens::models::Dialect;
use sqlquack_syntax::tokens::models::Token;
use sqlquack_syntax::tokens::types::Result;

use crate::parser::_helpers::support::dialects::dialect_type;

/// The tokens the parser reads for `sql`, each with the comments attached to it.
pub fn dialect_tokens(sql: &str, dialect: Dialect) -> Result<Vec<Token>> {
    crate::parser::models::Dialect::get(dialect_type(dialect)?)
        .tokenizer
        .tokenize(sql)
}
