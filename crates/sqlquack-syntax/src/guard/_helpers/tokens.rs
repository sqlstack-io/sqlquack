//! Guard: tokens.

use crate::guard::_helpers::codes::parse_guard_error;
use crate::guard::models::ComplexityGuardOptions;
use crate::guard::types::GuardToken;
use crate::tokens::models::ParserToken;
use crate::tokens::models::Span;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl GuardToken for Token {
    fn token_type(&self) -> TokenType {
        self.token_type
    }

    fn span(&self) -> Span {
        self.span
    }
}

impl GuardToken for ParserToken {
    fn token_type(&self) -> TokenType {
        self.token_type
    }

    fn span(&self) -> Span {
        self.span
    }
}

pub(crate) fn enforce_token_slice<T: GuardToken>(
    tokens: &[T],
    options: &ComplexityGuardOptions,
) -> Result<()> {
    if let Some(max) = options.max_tokens {
        let token_count = tokens.len();
        if token_count > max {
            let span = tokens
                .get(max)
                .or_else(|| tokens.last())
                .map(GuardToken::span);
            return Err(parse_guard_error(
                "E_GUARD_TOKEN_BUDGET_EXCEEDED",
                token_count,
                max,
                span,
            ));
        }
    }

    if options.max_parenthesis_depth.is_some() || options.max_function_call_depth.is_some() {
        let mut paren_depth = 0usize;
        let mut function_depth = 0usize;
        let mut paren_stack: Vec<bool> = Vec::new();
        let mut previous_significant: Option<TokenType> = None;

        for token in tokens {
            match token.token_type() {
                TokenType::LParen => {
                    paren_depth += 1;
                    if let Some(max) = options.max_parenthesis_depth {
                        if paren_depth > max {
                            return Err(parse_guard_error(
                                "E_GUARD_NESTING_DEPTH_EXCEEDED",
                                paren_depth,
                                max,
                                Some(token.span()),
                            ));
                        }
                    }

                    let is_function_call = previous_significant
                        .map(is_function_call_name_token)
                        .unwrap_or(false);
                    paren_stack.push(is_function_call);
                    if is_function_call {
                        function_depth += 1;
                        if let Some(max) = options.max_function_call_depth {
                            if function_depth > max {
                                return Err(parse_guard_error(
                                    "E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED",
                                    function_depth,
                                    max,
                                    Some(token.span()),
                                ));
                            }
                        }
                    }
                }
                TokenType::RParen => {
                    paren_depth = paren_depth.saturating_sub(1);
                    if paren_stack.pop().unwrap_or(false) {
                        function_depth = function_depth.saturating_sub(1);
                    }
                }
                _ => {}
            }

            if !is_trivia_token(token.token_type()) {
                previous_significant = Some(token.token_type());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn enforce_tokens(tokens: &[Token], options: &ComplexityGuardOptions) -> Result<()> {
    enforce_token_slice(tokens, options)
}

pub(crate) fn is_trivia_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::Space | TokenType::Break | TokenType::LineComment | TokenType::BlockComment
    )
}

pub(crate) fn is_function_call_name_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::Identifier
            | TokenType::Var
            | TokenType::QuotedIdentifier
            | TokenType::CurrentDate
            | TokenType::CurrentDateTime
            | TokenType::CurrentTime
            | TokenType::CurrentTimestamp
            | TokenType::CurrentUser
            | TokenType::If
            | TokenType::Index
            | TokenType::Insert
            | TokenType::Left
            | TokenType::Replace
            | TokenType::Right
            | TokenType::Row
    )
}
