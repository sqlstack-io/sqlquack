//! The lexer: tokens, blanks and comments of a Transact-SQL input, as SQL Server 2022 splits them.

use sqlquack_syntax::tokens::models::Span;

use crate::scan::constants::{
    CURRENCY_BLOCK, CURRENCY_SYMBOLS, MISSING_COMMENT_END, MISSING_COMMENT_END_CODE,
    TWO_CHARACTER_OPERATORS, UNCLOSED_QUOTATION, UNCLOSED_QUOTATION_CODE,
};
use crate::scan::errors::ScanError;
use crate::scan::models::{NumberKind, QuoteStyle, Token, TokenKind};

/// What the scanner read: tokens, the fault that stopped it, and the blank and comment ranges.
pub(crate) struct Scanned {
    pub(crate) tokens: Vec<Token>,
    pub(crate) fault: Option<ScanError>,
    /// Byte ranges of blanks and comments, with whether each is a comment and of which kind.
    pub(crate) trivia: Vec<Trivia>,
}

/// A blank run or a comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Trivia {
    pub(crate) kind: TriviaKind,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TriviaKind {
    Blank,
    LineComment,
    BlockComment,
}

/// Whether the lexer skips `c` between tokens: control characters, Unicode blanks and zero-width characters.
pub(crate) fn is_blank(c: char) -> bool {
    (c <= ' ' || c.is_whitespace() || matches!(c, '\u{200b}' | '\u{2060}' | '\u{feff}'))
        && c != '\u{7f}'
}

/// Whether `c` starts a `money` literal.
fn is_currency(c: char) -> bool {
    CURRENCY_SYMBOLS.contains(c) || (CURRENCY_BLOCK.0..=CURRENCY_BLOCK.1).contains(&c)
}

/// Whether `c` starts an unquoted name: a letter, `_`, `#` or `@`.
fn starts_word(c: char) -> bool {
    c.is_alphabetic() || matches!(c, '_' | '#' | '@')
}

/// Whether `c` continues an unquoted name.
fn continues_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '#' | '@' | '$')
}

pub(crate) struct Scanner<'sql> {
    sql: &'sql str,
    at: usize,
    tokens: Vec<Token>,
    trivia: Vec<Trivia>,
}

impl<'sql> Scanner<'sql> {
    pub(crate) fn new(sql: &'sql str) -> Self {
        Self {
            sql,
            at: 0,
            tokens: Vec::new(),
            trivia: Vec::new(),
        }
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.sql[self.at..].chars().nth(offset)
    }

    fn peek(&self) -> Option<char> {
        self.sql[self.at..].chars().next()
    }

    fn bump(&mut self) {
        if let Some(c) = self.peek() {
            self.at += c.len_utf8();
        }
    }

    fn bump_while(&mut self, test: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&test) {
            self.bump();
        }
    }

    fn rest(&self) -> &'sql str {
        &self.sql[self.at..]
    }

    fn push(&mut self, kind: TokenKind, value: String, start: usize) {
        self.tokens.push(Token {
            kind,
            value,
            start,
            end: self.at,
            span: Span::default(),
        });
    }

    /// Every token, then the fault that stopped the scan, if any, and the blanks and comments.
    pub(crate) fn scan(mut self) -> Scanned {
        let fault: Option<ScanError> = loop {
            let Some(c) = self.peek() else { break None };
            let start: usize = self.at;
            let result: Result<(), ScanError> = self.token(c, start);
            if let Err(fault) = result {
                break Some(fault);
            }
        };
        Scanned {
            tokens: self.tokens,
            fault,
            trivia: self.trivia,
        }
    }

    /// Reads the token, blank run or comment starting with `c` at `start`.
    fn token(&mut self, c: char, start: usize) -> Result<(), ScanError> {
        if is_blank(c) {
            self.bump_while(is_blank);
            return self.trivium(TriviaKind::Blank, start);
        }
        if self.rest().starts_with("--") {
            self.bump_while(|c| !matches!(c, '\n' | '\r'));
            return self.trivium(TriviaKind::LineComment, start);
        }
        if self.rest().starts_with("/*") {
            return self.block_comment(start);
        }
        match c {
            '\'' => self.string(start, false),
            'N' | 'n' if self.peek_at(1) == Some('\'') => {
                self.bump();
                self.string(start, true)
            }
            '[' => self.delimited(start, ']', QuoteStyle::Bracket),
            '"' => self.delimited(start, '"', QuoteStyle::Double),
            '0' if matches!(self.peek_at(1), Some('x' | 'X')) => {
                self.bump();
                self.bump();
                self.bump_while(|c| c.is_ascii_hexdigit());
                self.push_text(TokenKind::Number(NumberKind::Binary), start);
                Ok(())
            }
            '0'..='9' => {
                let kind: NumberKind = self.number();
                self.push_text(TokenKind::Number(kind), start);
                Ok(())
            }
            '.' if self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) => {
                let kind: NumberKind = self.number();
                self.push_text(TokenKind::Number(kind), start);
                Ok(())
            }
            '$' if self
                .peek_at(1)
                .is_some_and(|c| c.is_alphabetic() || c == '_') =>
            {
                self.bump();
                self.bump_while(continues_word);
                self.push_text(TokenKind::Word, start);
                Ok(())
            }
            c if is_currency(c) => {
                self.money();
                self.push_text(TokenKind::Number(NumberKind::Money), start);
                Ok(())
            }
            c if starts_word(c) => {
                self.bump();
                self.bump_while(continues_word);
                let kind: TokenKind = match c {
                    '@' => TokenKind::Variable,
                    _ => TokenKind::Word,
                };
                self.push_text(kind, start);
                Ok(())
            }
            _ => {
                match TWO_CHARACTER_OPERATORS
                    .iter()
                    .find(|operator| self.rest().starts_with(**operator))
                {
                    Some(operator) => {
                        self.at += operator.len();
                        self.push_text(TokenKind::Op, start);
                    }
                    None => {
                        self.bump();
                        self.push_text(TokenKind::Char(c), start);
                    }
                }
                Ok(())
            }
        }
    }

    fn push_text(&mut self, kind: TokenKind, start: usize) {
        let text: String = self.sql[start..self.at].to_owned();
        self.push(kind, text, start);
    }

    fn trivium(&mut self, kind: TriviaKind, start: usize) -> Result<(), ScanError> {
        self.trivia.push(Trivia {
            kind,
            start,
            end: self.at,
        });
        Ok(())
    }

    /// `/* ... */`, nesting: each `/*` inside needs its own `*/`.
    fn block_comment(&mut self, start: usize) -> Result<(), ScanError> {
        let mut depth: usize = 0;
        loop {
            let rest: &str = self.rest();
            if rest.is_empty() {
                return Err(ScanError {
                    message: MISSING_COMMENT_END.to_owned(),
                    code: MISSING_COMMENT_END_CODE,
                    position: start,
                });
            }
            if rest.starts_with("/*") {
                depth += 1;
                self.at += 2;
            } else if rest.starts_with("*/") {
                depth -= 1;
                self.at += 2;
                if depth == 0 {
                    return self.trivium(TriviaKind::BlockComment, start);
                }
            } else {
                self.bump();
            }
        }
    }

    /// The body of a quoted run up to `close`, a doubled `close` standing for one.
    fn quoted_body(&mut self, close: char) -> Result<String, String> {
        let mut value: String = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(value);
            };
            self.bump();
            if c == close {
                if self.peek() == Some(close) {
                    self.bump();
                } else {
                    return Ok(value);
                }
            }
            value.push(c);
        }
    }

    /// The fault for a quoted run that never closes, as the engine words it with its read body.
    fn unclosed(start: usize, body: &str) -> ScanError {
        ScanError {
            message: format!("{UNCLOSED_QUOTATION} '{body}'."),
            code: UNCLOSED_QUOTATION_CODE,
            position: start,
        }
    }

    fn string(&mut self, start: usize, national: bool) -> Result<(), ScanError> {
        self.bump();
        match self.quoted_body('\'') {
            Ok(value) => {
                self.push(TokenKind::String { national }, value, start);
                Ok(())
            }
            Err(body) => Err(Self::unclosed(start, &body)),
        }
    }

    fn delimited(&mut self, start: usize, close: char, style: QuoteStyle) -> Result<(), ScanError> {
        self.bump();
        match self.quoted_body(close) {
            Ok(value) => {
                self.push(TokenKind::Quoted(style), value, start);
                Ok(())
            }
            Err(body) => Err(Self::unclosed(start, &body)),
        }
    }

    /// Digits, a point and digits, then an exponent whose sign and digits may be missing.
    fn number(&mut self) -> NumberKind {
        let mut kind: NumberKind = NumberKind::Integer;
        self.bump_while(|c| c.is_ascii_digit());
        if self.peek() == Some('.') {
            kind = NumberKind::Decimal;
            self.bump();
            self.bump_while(|c| c.is_ascii_digit());
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            kind = NumberKind::Float;
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            self.bump_while(|c| c.is_ascii_digit());
        }
        kind
    }

    /// A currency symbol with any blanks, one sign and the digits and point after it.
    fn money(&mut self) {
        self.bump();
        let symbol_end: usize = self.at;
        self.bump_while(is_blank);
        let signed: bool = matches!(self.peek(), Some('+' | '-'));
        if signed {
            self.bump();
        }
        let digits: bool = match self.peek() {
            Some(c) if c.is_ascii_digit() => true,
            Some('.') => self.peek_at(1).is_some_and(|c| c.is_ascii_digit()),
            _ => false,
        };
        if !digits {
            if !signed {
                self.at = symbol_end;
            }
            return;
        }
        self.bump_while(|c| c.is_ascii_digit());
        if self.peek() == Some('.') {
            self.bump();
            self.bump_while(|c| c.is_ascii_digit());
        }
    }
}
