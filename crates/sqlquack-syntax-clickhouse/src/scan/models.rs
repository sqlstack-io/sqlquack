//! Tokens as ClickHouse's lexer (`src/Parsers/Lexer.cpp`) hands them to its parsers.

use sqlquack_syntax::tokens::models::Span;

/// What a token is: ClickHouse's `TokenType`, one variant per type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A run of blanks, ASCII or the Unicode ones the lexer skips.
    Whitespace,
    /// A `--`, `//`, `# `, `#!` or `/* ... */` comment.
    Comment,
    /// An unquoted word: a name or a keyword; its value is as written.
    BareWord,
    /// A number as written: decimal, `0x` hex, `0b` binary, with a point or an exponent.
    Number,
    /// A `'...'`, `‘...’`, `x'...'` or `b'...'` string; its value is the text as written.
    StringLiteral,
    /// A `` `...` ``, `"..."` or `“...”` name; its value is the text as written.
    QuotedIdentifier,
    /// A `$tag$...$tag$` string; its value is the text as written.
    HereDoc,
    OpeningRoundBracket,
    ClosingRoundBracket,
    OpeningSquareBracket,
    ClosingSquareBracket,
    OpeningCurlyBrace,
    ClosingCurlyBrace,
    Comma,
    Semicolon,
    /// `\G`.
    VerticalDelimiter,
    Dot,
    Asterisk,
    DollarSign,
    Plus,
    /// `-` or the Unicode minus sign `−`.
    Minus,
    Slash,
    Percent,
    Arrow,
    QuestionMark,
    Colon,
    Caret,
    DoubleColon,
    /// `=` or `==`.
    Equals,
    /// `!=` or `<>`.
    NotEquals,
    Less,
    Greater,
    LessOrEquals,
    GreaterOrEquals,
    /// `<=>`.
    Spaceship,
    PipeMark,
    Concatenation,
    At,
    DoubleAt,
    /// A character no rule takes, or a `#` not starting a comment.
    Error,
    ErrorMultilineCommentIsNotClosed,
    ErrorSingleQuoteIsNotClosed,
    ErrorDoubleQuoteIsNotClosed,
    ErrorBackQuoteIsNotClosed,
    ErrorSingleExclamationMark,
    ErrorWrongNumber,
}

impl TokenKind {
    /// Whether the parser reads the token, unlike blanks and comments.
    pub fn is_significant(self) -> bool {
        !matches!(self, Self::Whitespace | Self::Comment)
    }

    /// Whether the lexer failed at the token.
    pub fn is_error(self) -> bool {
        matches!(
            self,
            Self::Error
                | Self::ErrorMultilineCommentIsNotClosed
                | Self::ErrorSingleQuoteIsNotClosed
                | Self::ErrorDoubleQuoteIsNotClosed
                | Self::ErrorBackQuoteIsNotClosed
                | Self::ErrorSingleExclamationMark
                | Self::ErrorWrongNumber
        )
    }

    /// The message ClickHouse gives a lexical error (`getErrorTokenDescription`).
    pub fn error_description(self) -> &'static str {
        match self {
            Self::Error => "Unrecognized token",
            Self::ErrorMultilineCommentIsNotClosed => "Multiline comment is not closed",
            Self::ErrorSingleQuoteIsNotClosed => "Single quoted string is not closed",
            Self::ErrorDoubleQuoteIsNotClosed => "Double quoted string is not closed",
            Self::ErrorBackQuoteIsNotClosed => "Back quoted string is not closed",
            Self::ErrorSingleExclamationMark => "Exclamation mark can only occur in != operator",
            Self::ErrorWrongNumber => "Wrong number",
            _ => "Not an error",
        }
    }
}

/// One token: its kind, its text and its byte range in the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// The token's text as written.
    pub value: String,
    pub start: usize,
    pub end: usize,
    /// The range in characters, with the line and column after the token, as the shared tree records it.
    pub span: Span,
}
