//! `tokenize`: splits SQL into tokens covering every byte: significant tokens, whitespace and comments.

use sqlquack_syntax::tokens::models::Dialect;
use sqlquack_tokenize::lossless::errors::TokenizeFailure;
use sqlquack_tokenize::lossless::main::tokenize_lossless::tokenize_lossless;
use sqlquack_tokenize::lossless::models::LosslessToken;

use crate::parser::_helpers::support::dialects::tokenizer_config;

/// Splits SQL into tokens covering every byte: significant tokens, whitespace and comments.
pub fn tokenize(
    sql: &str,
    dialect: Dialect,
) -> std::result::Result<Vec<LosslessToken>, TokenizeFailure> {
    tokenize_lossless(sql, tokenizer_config(dialect)?)
}
