//! Tokens as DuckDB 2.0.0's PEG tokenizer hands them to the grammar.

use sqlquack_syntax::tokens::models::Span;

/// The letter before a string's opening quote, which changes how its body is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringPrefix {
    /// `'...'`: a doubled quote stands for one quote, a backslash is itself.
    Plain,
    /// `E'...'`: backslash escapes are decoded.
    Escape,
    /// `N'...'`: a national string, read as a plain one.
    National,
    /// `X'...'`: a blob literal of hexadecimal digits; its value is the body as written.
    Hex,
    /// `B'...'`: a bit-string literal; its value is the body as written.
    Bit,
}

/// What a token is, as the tokenizer's token types and the grammar's literals distinguish them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// An unquoted word, keyword or identifier; its value is as written.
    Word,
    /// A `"quoted"` (or `` `quoted` ``) identifier; its value is unquoted.
    Quoted,
    /// A quoted string; its value is the decoded text.
    String(StringPrefix),
    /// A `$tag$...$tag$` string; its value is the body.
    DollarString,
    /// A number of digits, points, separators and an exponent (`1.2.3` too); its value is as written.
    Number,
    /// A run of operator characters such as `<=`, `->>` or `**`, or `::` and `:=`.
    Op,
    /// One character the grammar names itself: `( ) { } [ ] , : ? $ #`, or `;`.
    Char(char),
    /// Where scanning stopped, its value the message; whether DuckDB raises it before parsing.
    Fault(bool),
}

/// One token: its kind, its value and its byte range in the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub start: usize,
    pub end: usize,
    /// The range in characters, with the line and column after the token, as the shared tree records it.
    pub span: Span,
}
