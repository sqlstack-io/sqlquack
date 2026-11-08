//! Tokens as SQLite 3.53's tokenizer hands them to the grammar.

use sqlquack_syntax::tokens::models::Span;

/// How a quoted identifier was delimited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quote {
    /// `"name"`, a doubled `"` standing for one.
    Double,
    /// `` `name` ``, a doubled `` ` `` standing for one.
    Backtick,
    /// `[name]`, ending at the first `]`.
    Bracket,
}

/// Which number token the tokenizer made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberKind {
    /// `INTEGER`: digits, or `0x` and hexadecimal digits.
    Integer,
    /// `FLOAT`: digits with a point or an exponent.
    Float,
    /// `QNUMBER`: a number written with `_` digit separators; its value has them removed.
    Separated,
    /// `QNUMBER` with a separator not between two digits, refused when the grammar reduces it.
    Misseparated,
}

/// What a token is, as the grammar's terminals distinguish them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// An unquoted word that is no keyword, or `WINDOW`, `OVER` or `FILTER` read as a name.
    Ident,
    /// A keyword, by its grammar token (`SELECT`, `JOIN_KW`, `LIKE_KW`, `CTIME_KW`, ...).
    Keyword(&'static str),
    /// A quoted identifier (`ID`); its value is unquoted.
    Quoted(Quote),
    /// A `'string'` (`STRING`); its value is decoded.
    String,
    /// A number; its value is as written, without digit separators.
    Number(NumberKind),
    /// An `x'hex'` blob literal (`BLOB`); its value is the hexadecimal digits.
    Blob,
    /// A parameter (`VARIABLE`): `?`, `?NNN`, `:name`, `@name`, `$name` or `#name`; as written.
    Variable,
    /// Punctuation or an operator, by its grammar token, such as `LP`, `CONCAT` or `PTR`.
    Op(&'static str),
    /// Text the tokenizer cannot read (`ILLEGAL`); its value is the message reported at it.
    Fault,
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
