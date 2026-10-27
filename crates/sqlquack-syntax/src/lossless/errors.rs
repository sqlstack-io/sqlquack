//! Errors of lossless.

use crate::tokens::errors::Error;

/// Why SQL could not be split into lossless tokens.
#[derive(Debug, thiserror::Error)]
pub enum TokenizeFailure {
    /// The tokenizer itself rejected the input.
    #[error(transparent)]
    Tokenizer(#[from] Error),
    /// Text between significant tokens that is neither whitespace nor a comment.
    #[error("text at bytes {start}..{end} belongs to no token")]
    Uncovered {
        /// Byte offset where the uncovered text starts.
        start: usize,
        /// Byte offset where it ends.
        end: usize,
    },
    /// A significant token that starts before the previous one ends.
    #[error("the token at bytes {start}..{end} overlaps the previous token")]
    Overlap {
        /// Byte offset where the token starts.
        start: usize,
        /// Byte offset where it ends.
        end: usize,
    },
}
