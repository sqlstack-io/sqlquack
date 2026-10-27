//! `enforce_parser_token_stats`.

use crate::guard::_helpers::codes::parse_guard_error;
use crate::guard::models::ComplexityGuardOptions;
use crate::guard::models::TokenGuardStats;
use crate::tokens::models::ParserToken;
use crate::tokens::types::Result;

pub(crate) fn enforce_parser_token_stats(
    tokens: &[ParserToken],
    stats: &TokenGuardStats,
    options: &ComplexityGuardOptions,
) -> Result<()> {
    if let Some(max) = options.max_tokens {
        if stats.token_count > max {
            let span = tokens
                .get(max)
                .or_else(|| tokens.last())
                .map(|token| token.span);
            return Err(parse_guard_error(
                "E_GUARD_TOKEN_BUDGET_EXCEEDED",
                stats.token_count,
                max,
                span,
            ));
        }
    }

    let parenthesis_error = options.max_parenthesis_depth.and_then(|max| {
        stats
            .parenthesis_depth_spans
            .get(max)
            .copied()
            .map(|span| (span, max))
    });
    let function_error = options.max_function_call_depth.and_then(|max| {
        stats
            .function_depth_spans
            .get(max)
            .copied()
            .map(|span| (span, max))
    });

    match (parenthesis_error, function_error) {
        (Some((paren_span, _paren_max)), Some((function_span, function_max)))
            if function_span.start < paren_span.start =>
        {
            Err(parse_guard_error(
                "E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED",
                function_max + 1,
                function_max,
                Some(function_span),
            ))
        }
        (Some((span, max)), _) => Err(parse_guard_error(
            "E_GUARD_NESTING_DEPTH_EXCEEDED",
            max + 1,
            max,
            Some(span),
        )),
        (None, Some((span, max))) => Err(parse_guard_error(
            "E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED",
            max + 1,
            max,
            Some(span),
        )),
        (None, None) => Ok(()),
    }
}
