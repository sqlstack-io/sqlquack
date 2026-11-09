//! Tokens as SqlBaseLexer.g4 hands them to the grammar.

use sqlquack_syntax::tokens::models::Span;

/// A numeric literal's lexer rule, which its suffix (if any) names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberKind {
    /// `INTEGER_VALUE`: digits.
    Integer,
    /// `DECIMAL_VALUE`: digits with a point.
    Decimal,
    /// `EXPONENT_VALUE`: digits with an exponent.
    Exponent,
    /// `BIGINT_LITERAL`: digits and `L`.
    BigInt,
    /// `SMALLINT_LITERAL`: digits and `S`.
    SmallInt,
    /// `TINYINT_LITERAL`: digits and `Y`.
    TinyInt,
    /// `DOUBLE_LITERAL`: a number and `D`.
    Double,
    /// `FLOAT_LITERAL`: a number and `F`.
    Float,
    /// `BIGDECIMAL_LITERAL`: a number and `BD`.
    BigDecimal,
}

/// What a token is, as the grammar's terminals distinguish them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// An unquoted identifier or keyword (`IDENTIFIER` or any keyword token); its value is as written.
    Word,
    /// A `` `quoted` `` identifier; its value is unquoted.
    Quoted,
    /// A `'...'` or `"..."` string; its value is the decoded text. `raw` for `r'...'`.
    String { raw: bool },
    /// A `$tag$...$tag$` string; its value is the body.
    DollarString,
    /// A number; its value is the text as written, suffix included.
    Number(NumberKind),
    /// An operator of more than one character, such as `::` or `<=>`; its value is the text.
    Op,
    /// One character the grammar names itself, such as `(` or `+`, or any other character.
    Char(char),
    /// `/*+`, which opens a hint.
    HintStart,
    /// `*/`, which closes a hint.
    HintEnd,
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
