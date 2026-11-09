//! Type aliases and traits of tokens.

use std::ops::Range;

use crate::tokens::errors::Error;
use crate::tokens::models::Span;
use crate::tokens::models::TokenSource;
use crate::tokens::models::TokenType;

/// The result type for polyglot operations
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) trait TokenOutput: Sized {
    fn from_source(
        token_type: TokenType,
        source: TokenSource<'_>,
        text: Range<usize>,
        span: Span,
    ) -> Self;
    fn from_owned(token_type: TokenType, text: String, span: Span) -> Self;
    fn token_type(&self) -> TokenType;
    fn text<'a>(&'a self, source: &'a str) -> &'a str;
    fn comments_mut(&mut self) -> &mut Vec<String>;
    fn trailing_comments_mut(&mut self) -> &mut Vec<String>;
}

pub(crate) trait TokenizerCursor {
    fn len(&self) -> usize;
    fn char_at(&self, index: usize) -> char;
    fn text_from_range(&self, source: &str, start: usize, end: usize) -> String;

    fn source_range<'a>(&self, _source: &'a str, _start: usize, _end: usize) -> Option<&'a str> {
        None
    }

    fn range_contains(&self, start: usize, needle: char) -> bool {
        (start..self.len()).any(|index| self.char_at(index) == needle)
    }
}

/// SQLQuack: a node field holding its source range, read and filled by located parses.
pub(crate) trait SpanField {
    fn recorded(&self) -> Option<Span>;
    /// Records `span` unless one is recorded; the parser's own name spans are never changed.
    fn fill(&mut self, span: Span);
}
