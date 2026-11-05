//! The words of the SQL, and how its brackets, lists and conditions divide them.

use sqlquack_syntax::lossless::models::{LosslessToken, TokenKind};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, tokenize};

use crate::layout::models::{Word, Words};

/// Every token but whitespace, whether whitespace preceded it, and where each bracket closes.
pub(crate) fn words<'sql>(sql: &'sql str, tokens: &[LosslessToken]) -> Words<'sql> {
    let mut words: Vec<Word<'sql>> = Vec::with_capacity(tokens.len());
    let mut spaced = false;
    for token in tokens {
        let (kind, line_comment) = match token.kind {
            TokenKind::Whitespace => {
                spaced = true;
                continue;
            }
            TokenKind::Significant(kind) => (Some(kind), false),
            TokenKind::LineComment => (None, true),
            TokenKind::BlockComment => (None, false),
        };
        words.push(Word {
            text: token.text(sql),
            kind,
            line_comment,
            spaced,
        });
        spaced = false;
    }
    let mut closer: Vec<Option<usize>> = vec![None; words.len()];
    let (mut brackets, mut cases): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    for (index, word) in words.iter().enumerate() {
        match word.kind {
            Some(TokenType::LParen) => brackets.push(index),
            Some(TokenType::Case) => cases.push(index),
            Some(TokenType::RParen) => {
                if let Some(open) = brackets.pop() {
                    closer[open] = Some(index);
                }
            }
            Some(TokenType::End) => {
                if let Some(open) = cases.pop() {
                    closer[open] = Some(index);
                }
            }
            _ => {}
        }
    }
    Words { words, closer }
}

/// The word after `index` at the same level: past the bracket or CASE that opens at `index`.
pub(crate) fn step(words: &Words<'_>, index: usize) -> usize {
    words.closer.get(index).copied().flatten().unwrap_or(index) + 1
}

/// The kind of the first word from `index` that is not a comment.
pub(crate) fn kind_at(words: &Words<'_>, index: usize) -> Option<TokenType> {
    words
        .words
        .get(index..)
        .unwrap_or_default()
        .iter()
        .find_map(|word| word.kind)
}

/// Whether `text` is the word at `index`, ignoring case.
pub(crate) fn text_is(words: &Words<'_>, index: usize, text: &str) -> bool {
    words
        .words
        .get(index)
        .is_some_and(|word| word.text.eq_ignore_ascii_case(text))
}

/// Whether the bracket at `open` holds a query, itself perhaps in brackets.
pub(crate) fn opens_query(words: &Words<'_>, open: usize) -> bool {
    match kind_at(words, open + 1) {
        Some(TokenType::Select | TokenType::With) => true,
        Some(TokenType::LParen) => opens_query(words, open + 1),
        _ => false,
    }
}

/// The ranges of `from..to` between its commas at this level, and each comma after a range.
pub(crate) fn items(
    words: &Words<'_>,
    from: usize,
    to: usize,
) -> Vec<(usize, usize, Option<usize>)> {
    let mut items: Vec<(usize, usize, Option<usize>)> = Vec::new();
    let (mut start, mut index) = (from, from);
    while index < to {
        if words.words[index].kind == Some(TokenType::Comma) {
            items.push((start, index, Some(index)));
            start = index + 1;
        }
        index = step(words, index);
    }
    if start < to {
        items.push((start, to, None));
    }
    items
}

/// Each `AND` or `OR` joining two conditions of `from..to` at this level, not `BETWEEN`'s `AND`.
pub(crate) fn conditions(words: &Words<'_>, from: usize, to: usize) -> Vec<usize> {
    let mut joins: Vec<usize> = Vec::new();
    let mut between = false;
    let mut index = from;
    while index < to {
        match words.words[index].kind {
            Some(TokenType::Between) => between = true,
            Some(TokenType::And) if between => between = false,
            Some(TokenType::And | TokenType::Or) if index > from => joins.push(index),
            _ => {}
        }
        index = step(words, index);
    }
    joins
}

/// Whether `from..=to` must break over lines: it holds a query or a line comment.
pub(crate) fn forced(words: &Words<'_>, from: usize, to: usize) -> bool {
    (from..=to).any(|index| {
        let word = &words.words[index];
        word.line_comment || (word.kind == Some(TokenType::LParen) && opens_query(words, index))
    })
}

/// How wide `from..=to` is on one line, with a space after every comma.
pub(crate) fn width(words: &Words<'_>, from: usize, to: usize) -> usize {
    (from..=to)
        .map(|index| {
            let spaced = index > from && spaced(words, index);
            words.words[index].text.chars().count() + usize::from(spaced)
        })
        .sum()
}

/// Whether the word at `index` takes a space on a shared line: after whitespace, a comma or `/`.
pub(crate) fn spaced(words: &Words<'_>, index: usize) -> bool {
    let word = &words.words[index];
    let previous = index
        .checked_sub(1)
        .and_then(|previous| words.words[previous].kind);
    word.spaced
        || word.kind == Some(TokenType::Slash)
        || matches!(previous, Some(TokenType::Comma | TokenType::Slash))
}

/// Whether `sql` has exactly the words `expected` holds.
pub(crate) fn same_words(sql: &str, dialect: Dialect, expected: &Words<'_>) -> bool {
    let Ok(tokens) = tokenize(sql, dialect) else {
        return false;
    };
    let found = words(sql, &tokens);
    found.words.len() == expected.words.len()
        && found
            .words
            .iter()
            .zip(&expected.words)
            .all(|(found, expected)| found.text == expected.text && found.kind == expected.kind)
}

/// Where a window specification `from..to` starts its `ORDER BY`, after a `PARTITION BY`.
pub(crate) fn window_orders(words: &Words<'_>, from: usize, to: usize) -> Vec<usize> {
    let mut index = from + 1;
    while index < to {
        if words.words[index].kind == Some(TokenType::Order) {
            return vec![index];
        }
        index = step(words, index);
    }
    Vec::new()
}
