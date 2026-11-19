//! `has_function_call`.

use crate::guard::_helpers::tokens::{is_function_call_name_token, is_trivia_token};
use crate::tokens::models::{Token, TokenType};

/// Whether `tokens` hold a call: a name directly before an opening parenthesis, as the call-depth guard counts one.
pub fn has_function_call(tokens: &[Token]) -> bool {
    let significant: Vec<TokenType> = tokens
        .iter()
        .map(|token| token.token_type)
        .filter(|token_type| !is_trivia_token(*token_type))
        .collect();
    significant
        .windows(2)
        .any(|pair| is_function_call_name_token(pair[0]) && pair[1] == TokenType::LParen)
}
