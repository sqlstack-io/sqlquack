//! Type aliases and traits of tokens.

use crate::tokens::errors::Error;
use crate::tokens::models::Span;

/// The result type for polyglot operations
pub type Result<T> = std::result::Result<T, Error>;

/// SQLQuack: a node field holding its source range, read and filled by located parses.
pub(crate) trait SpanField {
    fn recorded(&self) -> Option<Span>;
    /// Records `span` unless one is recorded; the parser's own name spans are never changed.
    fn fill(&mut self, span: Span);
}
