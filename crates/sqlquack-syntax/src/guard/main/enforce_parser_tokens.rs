//! `enforce_parser_tokens`.

use crate::guard::_helpers::tokens::enforce_token_slice;
use crate::guard::models::ComplexityGuardOptions;
use crate::tokens::models::ParserToken;
use crate::tokens::types::Result;

pub fn enforce_parser_tokens(
    tokens: &[ParserToken],
    options: &ComplexityGuardOptions,
) -> Result<()> {
    enforce_token_slice(tokens, options)
}
