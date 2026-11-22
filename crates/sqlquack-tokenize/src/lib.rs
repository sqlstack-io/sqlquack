//! SQLQuack's tokenizer and lossless tokenizer, apart from the syntax tree so that tree readers need not reach them.

#![forbid(unsafe_code)]

pub mod lossless;
pub mod tokens;

pub use crate::lossless::errors::TokenizeFailure;
pub use crate::lossless::models::LosslessToken;
pub use crate::lossless::models::TokenKind;
pub use crate::tokens::models::{Tokenizer, TokenizerConfig};
