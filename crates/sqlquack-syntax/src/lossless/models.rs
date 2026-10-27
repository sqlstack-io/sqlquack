//! Data models of lossless.

use crate::tokens::models::Span;
use crate::tokens::models::TokenType;

/// What a lossless token holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// A token the parser reads.
    Significant(TokenType),
    /// A run of whitespace.
    Whitespace,
    /// A comment that runs to the end of the line (`--`, or `//` in Snowflake).
    LineComment,
    /// A `/* ... */` comment.
    BlockComment,
}

/// One token of the input, with its byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessToken {
    /// What the token holds.
    pub kind: TokenKind,
    /// Byte offset of the token's first byte.
    pub start: usize,
    /// Byte offset just past the token's last byte.
    pub end: usize,
}

impl LosslessToken {
    /// The token's text in `sql`, the input it was produced from.
    pub fn text<'sql>(&self, sql: &'sql str) -> &'sql str {
        &sql[self.start..self.end]
    }
}

/// Byte offsets for the tokenizer's character offsets.
#[derive(Debug)]
pub(crate) struct CharOffsets {
    pub(crate) bytes: Option<Vec<usize>>,
    pub(crate) len: usize,
}

impl CharOffsets {
    pub(crate) fn new(sql: &str) -> Self {
        let bytes = (!sql.is_ascii()).then(|| sql.char_indices().map(|(byte, _)| byte).collect());
        Self {
            bytes,
            len: sql.len(),
        }
    }

    pub(crate) fn byte(&self, char_offset: usize) -> usize {
        match &self.bytes {
            None => char_offset.min(self.len),
            Some(bytes) => bytes.get(char_offset).copied().unwrap_or(self.len),
        }
    }

    pub(crate) fn bytes(&self, span: Span) -> (usize, usize) {
        (self.byte(span.start), self.byte(span.end))
    }
}
