//! Tokens as SQL Server 2022's Transact-SQL lexer hands them to the grammar.

use sqlquack_syntax::tokens::models::Span;

/// How a delimited identifier was quoted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuoteStyle {
    /// `[name]`: a doubled `]` stands for one.
    Bracket,
    /// `"name"`, as `QUOTED_IDENTIFIER ON` reads it: a doubled `"` stands for one.
    Double,
}

/// What a numeric literal is, as the lexer types it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberKind {
    /// Digits only: `int`, `bigint` or `numeric` by size.
    Integer,
    /// Digits with a point: `numeric`.
    Decimal,
    /// Digits with an exponent, `1e5` or `1.5E-3`: `float`.
    Float,
    /// A currency symbol and an optional number, such as `$12.50` or a bare `$`: `money`.
    Money,
    /// `0x` and hexadecimal digits, possibly none: `varbinary`.
    Binary,
}

/// What a token is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// An unquoted word: a keyword or a name, `#temp` and `$action` included; its value as written.
    Word,
    /// `@name` or `@@name`; its value as written.
    Variable,
    /// A delimited identifier; its value without the delimiters, doubled closers undoubled.
    Quoted(QuoteStyle),
    /// A `'string'`, or with `national` an `N'string'`; its value the decoded text.
    String { national: bool },
    /// A numeric literal; its value as written.
    Number(NumberKind),
    /// An operator of more than one character, such as `<=`, `!<`, `+=` or `::`.
    Op,
    /// One character on its own: `( ) , ; . + - * / % & | ^ ~ = < > ! { } :`, or one the grammar never reads.
    Char(char),
    /// Where scanning stopped, its value the engine's message and its error number.
    Fault(u32),
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

/// Whether a word is one of SQL Server's keywords, and whether it is reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeywordStatus {
    /// One of the engine's reserved keywords; it names nothing unless delimited.
    Reserved,
    /// A word the grammar reads as a keyword in some position but that may still name things.
    Unreserved,
}
