//! Statements a parser reports where it read them, as token ranges for the layout guide.

use crate::dialects::models::Lexeme;
use crate::dialects::models::StatementSpan;

/// One statement's tokens, from its first to its last, a `;` written right after it included, and how many statements enclose it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StatementTokens {
    pub(crate) first: usize,
    pub(crate) last: usize,
    pub(crate) depth: usize,
}

/// The token range of each statement in `spans`, among `lexemes`.
pub(crate) fn statement_tokens(
    spans: &[StatementSpan],
    lexemes: &[Lexeme],
) -> Vec<StatementTokens> {
    spans
        .iter()
        .filter_map(|span| {
            let first = lexemes.partition_point(|lexeme| lexeme.start < span.start);
            let last = lexemes
                .partition_point(|lexeme| lexeme.end <= span.end)
                .checked_sub(1)?;
            (first <= last).then_some(StatementTokens {
                first,
                last,
                depth: span.depth,
            })
        })
        .collect()
}
