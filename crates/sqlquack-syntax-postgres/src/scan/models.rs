//! Tokens as PostgreSQL's scanner and its lookahead filter hand them to the grammar.

use crate::keywords::models::Keyword;

/// What a token is, as gram.y's terminals distinguish them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// An unquoted identifier that is no keyword; its value is folded to lower case.
    Ident,
    /// A keyword; its value is the folded keyword.
    Keyword(Keyword),
    /// A `"quoted"` or `U&"quoted"` identifier; its value is unquoted.
    QuotedIdent,
    /// A `'...'`, `U&'...'` or `N'...'` string; its value is the decoded text.
    String,
    /// An `E'...'` string; its value is the decoded text.
    EscapeString,
    /// A `$tag$...$tag$` string; its value is the text between the delimiters.
    DollarString,
    /// A `B'...'` bit string; its value is the text between the quotes.
    BitString,
    /// An `X'...'` hexadecimal string; its value is the text between the quotes.
    HexString,
    /// An integer fitting in 32 bits (`ICONST`).
    Integer,
    /// Any other number (`FCONST`).
    Numeric,
    /// `$1`; its value is the digits.
    Param,
    /// A multi-character or user-defined operator (`Op`).
    Op,
    /// `::`.
    Typecast,
    /// `..`.
    DotDot,
    /// `:=`.
    ColonEquals,
    /// `=>`.
    EqualsGreater,
    /// `<=`.
    LessEquals,
    /// `>=`.
    GreaterEquals,
    /// `<>` or `!=`.
    NotEquals,
    /// One character the grammar names itself, such as `(` or `+`, or any other single character.
    Char(char),
    /// Where scanning stopped; its value is the message, and whether it is a syntax error (42601).
    Fault(bool),
}

/// One token: its kind, its value and its byte range in the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// The folded name, decoded string or operator text.
    pub value: String,
    pub start: usize,
    pub end: usize,
}

/// A syntax error, which PostgreSQL reports with SQLSTATE 42601.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    pub message: String,
    /// Byte offset of the token it is at or near; the input's length at its end.
    pub position: usize,
}
