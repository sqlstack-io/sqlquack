//! Errors of modes.

use sqlquack_syntax::lossless::errors::TokenizeFailure;
use sqlquack_syntax::tokens::errors::Error;

/// Why strict parsing refused the input.
#[derive(Debug, thiserror::Error)]
pub enum StrictFailure {
    /// The parser rejected the input.
    #[error(transparent)]
    Parse(#[from] Error),
    /// The input does not split into lossless tokens.
    #[error(transparent)]
    Tokens(#[from] TokenizeFailure),
}
