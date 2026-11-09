//! `tokenize_lossless`: lossless tokens of `sql`, with a tokenizer configuration deciding the significant tokens.

use crate::lossless::errors::TokenizeFailure;
use crate::lossless::main::lossless_tokens::lossless_tokens;
use crate::lossless::models::LosslessToken;
use crate::tokens::models::TokenizerConfig;
use crate::tokens::models::{Span, TokenType, Tokenizer};

/// Lossless tokens of `sql`, with the tokenizer `config` builds deciding the significant tokens.
pub fn tokenize_lossless(
    sql: &str,
    config: TokenizerConfig,
) -> Result<Vec<LosslessToken>, TokenizeFailure> {
    let nested = config.nested_comments;
    let tokens = Tokenizer::new(config).tokenize(sql)?;
    let significant: Vec<(Span, TokenType)> = tokens
        .iter()
        .map(|token| (token.span, token.token_type))
        .collect();
    lossless_tokens(sql, &significant, nested)
}
