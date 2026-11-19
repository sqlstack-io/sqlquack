//! `enforce_lexed_tokens`.

use crate::guard::_helpers::tokens::enforce_token_slice;
use crate::guard::models::ComplexityGuardOptions;
use crate::tokens::models::Token;
use crate::tokens::types::Result;

/// Refuses `tokens` past `options`' token, parenthesis and function-call limits, before any parse.
pub fn enforce_lexed_tokens(tokens: &[Token], options: &ComplexityGuardOptions) -> Result<()> {
    enforce_token_slice(tokens, options)
}
