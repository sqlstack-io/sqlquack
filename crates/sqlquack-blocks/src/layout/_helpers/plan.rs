//! What the layout knows before it starts: matching parentheses and `END`s, and widths.

use crate::layout::_helpers::tokens::{Token, TokenKind};
use crate::layout::constants::{QUERY_OPENERS, SET_OPERATIONS};

/// Facts about the tokens, each found in one pass over them.
pub(crate) struct Plan {
    /// For each opening parenthesis, the index of its closing one.
    closes: Vec<usize>,
    /// For each `CASE`, the index of its `END`.
    ends: Vec<usize>,
    /// For each opening parenthesis, whether a set operation stands directly inside it.
    set_operations: Vec<bool>,
    /// Characters other than whitespace before each token, and in all of them at the end.
    widths: Vec<usize>,
}

impl Plan {
    /// The facts about `tokens`; an unmatched parenthesis or `CASE` reaches the last token.
    pub(crate) fn new(tokens: &[Token<'_>]) -> Self {
        let last = tokens.len().saturating_sub(1);
        let (mut closes, mut ends) = (vec![last; tokens.len()], vec![last; tokens.len()]);
        let mut set_operations = vec![false; tokens.len()];
        let (mut parentheses, mut cases): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
        let mut widths = Vec::with_capacity(tokens.len() + 1);
        widths.push(0);
        for (index, token) in tokens.iter().enumerate() {
            widths.push(widths[index] + token.text.chars().count());
            match token {
                token if token.is_symbol("(") => parentheses.push(index),
                token if token.is_symbol(")") => {
                    if let Some(open) = parentheses.pop() {
                        closes[open] = index;
                    }
                }
                token if token.is("CASE") => cases.push(index),
                token if token.is("END") => {
                    if let Some(open) = cases.pop() {
                        ends[open] = index;
                    }
                }
                token if SET_OPERATIONS.iter().any(|word| token.is(word)) => {
                    if let Some(open) = parentheses.last() {
                        set_operations[*open] = true;
                    }
                }
                _ => {}
            }
        }
        Self {
            closes,
            ends,
            set_operations,
            widths,
        }
    }

    /// The index of the parenthesis closing the one at `open`.
    pub(crate) fn close(&self, open: usize) -> usize {
        self.closes[open]
    }

    /// The index of the `END` closing the `CASE` at `open`.
    pub(crate) fn case_end(&self, open: usize) -> usize {
        self.ends[open]
    }

    /// Characters other than whitespace from token `first` through token `last`.
    pub(crate) fn width(&self, first: usize, last: usize) -> usize {
        self.widths
            .get(last + 1)
            .map_or(0, |end| end.saturating_sub(self.widths[first]))
    }

    /// Whether the parenthesis at `open` holds a query or a set operation of queries.
    pub(crate) fn opens_query(&self, tokens: &[Token<'_>], open: usize) -> bool {
        let first = tokens[open + 1..]
            .iter()
            .find(|token| !matches!(token.kind, TokenKind::LineComment | TokenKind::BlockComment));
        self.set_operations[open]
            || first.is_some_and(|token| QUERY_OPENERS.iter().any(|word| token.is(word)))
    }
}
