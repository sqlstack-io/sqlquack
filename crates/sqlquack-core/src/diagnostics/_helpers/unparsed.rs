//! A parse failure in SQL's words: what was expected, which bracket is open, and templates.

use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, LosslessToken, TokenKind, tokenize};

use crate::diagnostics::constants::{
    CLOSING_PAREN, END_OF_INPUT, EXPECTED_PREFIX, FOUND_SEPARATOR, NEAR_SUFFIX, TEMPLATE_HELP,
    TEMPLATE_MARKERS, TOKEN_TEXTS, UNCLOSED_PAREN_HELP, UNEXPECTED_TOKEN,
};
use crate::parsing::models::{SourceLines, SourceLocation};

/// What a parse failure's diagnostic says: its label, where it points and the likely fix.
pub(crate) struct Unparsed {
    pub(crate) label: String,
    pub(crate) location: Option<SourceLocation>,
    pub(crate) help: Option<String>,
}

/// The parser's `message` at `location` in `source`, worded for the reader.
pub(crate) fn unparsed(message: &str, location: Option<SourceLocation>, source: &str) -> Unparsed {
    let found = location.map(|at| found_text(source, at));
    let help = TEMPLATE_MARKERS
        .iter()
        .any(|marker| source.contains(marker))
        .then(|| TEMPLATE_HELP.to_owned());
    let Some((expected, got)) = message
        .strip_prefix(EXPECTED_PREFIX)
        .and_then(|rest| rest.split_once(FOUND_SEPARATOR))
    else {
        let label = match (message.contains(UNEXPECTED_TOKEN), found) {
            (true, Some(found)) if found.is_empty() => "unexpected end of the input".to_owned(),
            (true, Some(found)) => format!("unexpected `{found}`"),
            _ => message
                .split(NEAR_SUFFIX)
                .next()
                .unwrap_or(message)
                .to_owned(),
        };
        return Unparsed {
            label,
            location,
            help,
        };
    };
    let expected = token_text(expected);
    let at_end = got.starts_with(END_OF_INPUT) || found.as_ref().is_some_and(String::is_empty);
    if expected == CLOSING_PAREN
        && at_end
        && let Some(open) = unclosed_paren(source)
    {
        return Unparsed {
            label: "this `(` is never closed".to_owned(),
            location: Some(SourceLines::new(source).location(open, open + 1)),
            help: help.or_else(|| Some(UNCLOSED_PAREN_HELP.to_owned())),
        };
    }
    let found = match (at_end, found) {
        (false, Some(found)) if !found.is_empty() => format!("`{found}`"),
        _ => "the end of the input".to_owned(),
    };
    Unparsed {
        label: format!("expected {expected}, found {found}"),
        location,
        help,
    }
}

/// An expected token as SQL writes it: its text for a symbol or keyword, else the parser's words.
fn token_text(expected: &str) -> String {
    if let Some((_, text)) = TOKEN_TEXTS.iter().find(|(name, _)| *name == expected) {
        return format!("`{text}`");
    }
    match expected.chars().all(char::is_alphanumeric) && expected.starts_with(char::is_uppercase) {
        true => {
            let mut keyword = String::new();
            for character in expected.chars() {
                if character.is_uppercase() && !keyword.is_empty() {
                    keyword.push(' ');
                }
                keyword.push(character.to_ascii_uppercase());
            }
            format!("`{keyword}`")
        }
        false => expected.to_owned(),
    }
}

/// The source's characters at `location`.
fn found_text(source: &str, location: SourceLocation) -> String {
    source
        .chars()
        .skip(location.start)
        .take(location.end.saturating_sub(location.start))
        .collect()
}

/// The character offset of the last `(` the source never closes, outside strings and comments.
fn unclosed_paren(source: &str) -> Option<usize> {
    let tokens: Vec<LosslessToken> = tokenize(source, Dialect::Snowflake).unwrap_or_default();
    let mut open: Vec<usize> = Vec::new();
    for token in tokens {
        match token.kind {
            TokenKind::Significant(TokenType::LParen) => open.push(token.start),
            TokenKind::Significant(TokenType::RParen) => {
                open.pop();
            }
            _ => {}
        }
    }
    open.last().map(|byte| source[..*byte].chars().count())
}
