//! Type aliases and traits of guard.

use crate::tokens::models::Span;
use crate::tokens::models::TokenType;

/// Enforce token and pre-parse nesting limits.
pub(crate) trait GuardToken {
    fn token_type(&self) -> TokenType;
    fn span(&self) -> Span;
}
