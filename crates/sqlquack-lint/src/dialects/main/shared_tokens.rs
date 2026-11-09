//! `shared_tokens`: the tokens of SQL as the shared tokenizer reads a dialect.

use sqlquack_parse::dialect_tokens;
use sqlquack_syntax::Dialect;
use sqlquack_syntax::tokens::models::Token;

/// The tokens of `sql` in `dialect`, or why the shared tokenizer stopped.
pub(crate) fn shared_tokens(sql: &str, dialect: Dialect) -> Result<Vec<Token>, String> {
    dialect_tokens(sql, dialect).map_err(|error| error.to_string())
}
