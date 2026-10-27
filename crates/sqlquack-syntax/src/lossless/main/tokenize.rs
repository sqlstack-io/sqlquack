//! `tokenize`: splits SQL into tokens covering every byte: significant tokens, whitespace and comments.

use crate::lossless::errors::TokenizeFailure;
use crate::lossless::models::LosslessToken;
use crate::tokens::models::Dialect;

/// Splits SQL into tokens covering every byte: significant tokens, whitespace and comments.
pub fn tokenize(
    sql: &str,
    dialect: Dialect,
) -> std::result::Result<Vec<LosslessToken>, TokenizeFailure> {
    crate::lossless::main::tokenize_lossless::tokenize_lossless(sql, dialect)
}
