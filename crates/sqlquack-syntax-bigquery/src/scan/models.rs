//! Tokens as GoogleSQL's lexer (googlesql.tm, `:: lexer`) hands them to the grammar.

use sqlquack_syntax::tokens::models::Span;

/// A numeric literal's lexer rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberKind {
    /// `DECIMAL_INTEGER_LITERAL`: digits.
    Integer,
    /// `HEX_INTEGER_LITERAL`: `0x` and hexadecimal digits.
    Hex,
    /// `FLOATING_POINT_LITERAL`: digits with a point or an exponent.
    Float,
}

/// What a token is, as the grammar's terminals distinguish them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// An unquoted identifier or keyword; its value is as written.
    Word,
    /// A `` `quoted` `` identifier; its value is unescaped.
    Quoted,
    /// A string or bytes literal; its value is the decoded text.
    String { raw: bool, bytes: bool },
    /// A number; its value is the text as written.
    Number(NumberKind),
    /// An operator of more than one character, such as `::` or `|>`; its value is the text.
    Op,
    /// One character the grammar names itself, such as `(` or `+`, or any other character.
    Char(char),
    /// Where scanning stopped; its value is the message, and whether it is a syntax error.
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
