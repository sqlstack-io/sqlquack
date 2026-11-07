//! The spacing guard: whitespace changes between tokens are checked on the bytes, not the tokenizer.

use sqlquack_syntax::Dialect;

use crate::format::_helpers::lexemes::{Lexeme, is_keyword};
use crate::format::_helpers::token_layout::AuthoredSql;
use crate::format::constants::{
    GUARD_OPERATORS, JOIN_FAILURE, OPERATOR_BYTES, SEPARABLE_KEYWORDS, SEPARABLE_OPERATORS,
    SEPARATION_FAILURE, UNSEPARABLE_NEIGHBOURS,
};

/// Refuse output that separates touching tokens, or joins separated ones, where that could change their reading.
pub(crate) fn verify_touching_tokens(
    before: &AuthoredSql<'_>,
    after: &AuthoredSql<'_>,
    binary_operators: &[bool],
    dialect: Dialect,
) -> Result<(), String> {
    let pairs = before.lexemes.windows(2).zip(after.lexemes.windows(2));
    for (index, (source, printed)) in pairs.enumerate() {
        let touched = source[0].end == source[1].start;
        let touches = printed[0].end == printed[1].start;
        let operator = |offset: usize| {
            binary_operators[index + offset]
                && separable_operator(before.sql, &source[offset], &source[1 - offset])
        };
        if touched && !touches && !separable(&source[0], &source[1]) && !operator(0) && !operator(1)
        {
            return Err(SEPARATION_FAILURE.to_string());
        }
        if !touched && touches && could_fuse(&source[0].raw, &source[1].raw, dialect) {
            return Err(JOIN_FAILURE.to_string());
        }
    }
    Ok(())
}

/// Whether whitespace may go between touching tokens: after a comma, just inside a bracket, around a parenthesis, or beside a clause keyword.
fn separable(previous: &Lexeme, next: &Lexeme) -> bool {
    let clause = |lexeme: &Lexeme| {
        is_keyword(lexeme)
            && SEPARABLE_KEYWORDS
                .iter()
                .any(|keyword| lexeme.raw.eq_ignore_ascii_case(keyword))
    };
    let numeric = [previous, next]
        .iter()
        .any(|lexeme| lexeme.raw.starts_with(|first: char| first.is_ascii_digit()));
    matches!(previous.raw.as_str(), "," | ";" | "(" | ")" | "[" | "{")
        || matches!(next.raw.as_str(), "(" | ")" | "]" | "}")
        || (!numeric && (clause(previous) || clause(next)))
}

/// Whether the parse tree's binary `operator` may be spaced from `neighbour`: the guard's own lexing of the source finds exactly it, and neither side is a path or name accessor.
fn separable_operator(sql: &str, operator: &Lexeme, neighbour: &Lexeme) -> bool {
    let bytes = sql.as_bytes();
    let is_operator_byte = |byte: &u8| OPERATOR_BYTES.as_bytes().contains(byte);
    let start = bytes[..operator.start]
        .iter()
        .rposition(|byte| !is_operator_byte(byte))
        .map_or(0, |at| at + 1);
    let end = bytes[operator.end..]
        .iter()
        .position(|byte| !is_operator_byte(byte))
        .map_or(sql.len(), |at| operator.end + at);
    let exponent = matches!(operator.raw.as_str(), "+" | "-")
        && start == operator.start
        && matches!(&bytes[..start], [.., digit, b'e' | b'E'] if digit.is_ascii_digit() || *digit == b'.');
    let accessor = UNSEPARABLE_NEIGHBOURS.contains(&neighbour.raw.as_str())
        || neighbour.raw.starts_with(['@', '$']);
    SEPARABLE_OPERATORS.contains(&operator.raw.as_str())
        && !exponent
        && !accessor
        && !sql[start..end].contains(':')
        && operator_pieces(&sql[start..end], start).contains(&(operator.start, operator.end))
}

/// The byte ranges of a run of operator characters, read left to right taking the longest operator each time.
fn operator_pieces(run: &str, offset: usize) -> Vec<(usize, usize)> {
    let mut pieces: Vec<(usize, usize)> = Vec::new();
    let mut at = 0;
    while at < run.len() {
        let width = GUARD_OPERATORS
            .iter()
            .filter(|operator| run[at..].starts_with(*operator))
            .map(|operator| operator.len())
            .max()
            .unwrap_or(1);
        pieces.push((offset + at, offset + at + width));
        at += width;
    }
    pieces
}

/// Whether printing `previous` and `next` with no space between them could read as different tokens; DuckDB lexes `@` as an operator character, Snowflake as part of a stage name.
fn could_fuse(previous: &str, next: &str, dialect: Dialect) -> bool {
    let (Some(left), Some(right)) = (previous.chars().last(), next.chars().next()) else {
        return false;
    };
    let at_operator = dialect == Dialect::DuckDB;
    let word = |character: char| {
        character.is_alphanumeric()
            || matches!(character, '_' | '$' | '\'' | '"' | '`')
            || (character == '@' && !at_operator)
    };
    let operator =
        |character: char| OPERATOR_BYTES.contains(character) || (character == '@' && at_operator);
    (word(left) && word(right)) || (operator(left) && operator(right))
}
