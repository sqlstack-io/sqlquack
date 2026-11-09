//! ClickHouse's lexer (`Lexer::nextTokenImpl`), rule for rule, over the input's bytes.

use sqlquack_syntax::tokens::models::Span;

use crate::scan::constants::{
    UNICODE_BLANKS, UNICODE_DOUBLE_QUOTES, UNICODE_MINUS, UNICODE_QUOTED_MIN_BYTES,
    UNICODE_SINGLE_QUOTES,
};
use crate::scan::models::{Token, TokenKind};

/// `isWordCharASCII`: an ASCII letter, digit or `_`.
pub(crate) fn is_word_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `isWhitespaceASCII`.
fn is_blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0B' | b'\x0C')
}

/// `isNumberSeparator`: a `_` inside a block of digits, followed by a digit of the same base.
fn is_number_separator(start_of_block: bool, hex: bool, bytes: &[u8], at: usize) -> bool {
    if bytes.get(at) != Some(&b'_') || start_of_block {
        return false;
    }
    bytes.get(at + 1).is_some_and(|next| match hex {
        true => next.is_ascii_hexdigit(),
        false => next.is_ascii_digit(),
    })
}

/// Whether `byte` is a digit of the base a number is in.
fn is_digit(byte: u8, hex: bool) -> bool {
    match hex {
        true => byte.is_ascii_hexdigit(),
        false => byte.is_ascii_digit(),
    }
}

pub(crate) struct Lexer<'sql> {
    sql: &'sql str,
    bytes: &'sql [u8],
    at: usize,
    previous_significant: Option<TokenKind>,
}

impl<'sql> Lexer<'sql> {
    pub(crate) fn new(sql: &'sql str) -> Self {
        Self {
            sql,
            bytes: sql.as_bytes(),
            at: 0,
            previous_significant: None,
        }
    }

    /// Every token of the input, blanks and comments included, in order; together they cover it.
    pub(crate) fn tokens(mut self) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        while self.at < self.bytes.len() {
            let start: usize = self.at;
            let kind: TokenKind = self.next_kind();
            let mut end: usize = self.at.max(start + 1).min(self.bytes.len());
            while !self.sql.is_char_boundary(end) {
                end += 1;
            }
            self.at = end;
            if kind.is_significant() {
                self.previous_significant = Some(kind);
            }
            tokens.push(Token {
                kind,
                value: self.sql[start..end].to_owned(),
                start,
                end,
                span: Span::default(),
            });
        }
        tokens
    }

    fn byte(&self, at: usize) -> Option<u8> {
        self.bytes.get(at).copied()
    }

    /// Consumes `next` when it comes next, and says whether it did.
    fn eat(&mut self, next: u8) -> bool {
        let found: bool = self.byte(self.at) == Some(next);
        self.at += usize::from(found);
        found
    }

    fn single(&mut self, kind: TokenKind) -> TokenKind {
        self.at += 1;
        kind
    }

    fn comment_to_line_end(&mut self) -> TokenKind {
        while self.at < self.bytes.len() && self.bytes[self.at] != b'\n' {
            self.at += 1;
        }
        TokenKind::Comment
    }

    fn next_kind(&mut self) -> TokenKind {
        let start: usize = self.at;
        match self.bytes[start] {
            byte if is_blank(byte) => {
                while self.at < self.bytes.len() && is_blank(self.bytes[self.at]) {
                    self.at += 1;
                }
                TokenKind::Whitespace
            }
            b'0'..=b'9' => self.number(start),
            b'\'' => self.quoted(
                b'\'',
                TokenKind::StringLiteral,
                TokenKind::ErrorSingleQuoteIsNotClosed,
            ),
            b'"' => self.quoted(
                b'"',
                TokenKind::QuotedIdentifier,
                TokenKind::ErrorDoubleQuoteIsNotClosed,
            ),
            b'`' => self.quoted(
                b'`',
                TokenKind::QuotedIdentifier,
                TokenKind::ErrorBackQuoteIsNotClosed,
            ),
            b'(' => self.single(TokenKind::OpeningRoundBracket),
            b')' => self.single(TokenKind::ClosingRoundBracket),
            b'[' => self.single(TokenKind::OpeningSquareBracket),
            b']' => self.single(TokenKind::ClosingSquareBracket),
            b'{' => self.single(TokenKind::OpeningCurlyBrace),
            b'}' => self.single(TokenKind::ClosingCurlyBrace),
            b',' => self.single(TokenKind::Comma),
            b';' => self.single(TokenKind::Semicolon),
            b'.' => self.dot(start),
            b'+' => self.single(TokenKind::Plus),
            b'*' => self.single(TokenKind::Asterisk),
            b'%' => self.single(TokenKind::Percent),
            b'?' => self.single(TokenKind::QuestionMark),
            b'^' => self.single(TokenKind::Caret),
            _ => self.operator(start),
        }
    }

    /// The operators of more than one possible character, comments, and the remaining rules.
    fn operator(&mut self, start: usize) -> TokenKind {
        let first: u8 = self.bytes[start];
        if first.is_ascii() && !matches!(first, b'$' | b'x' | b'X' | b'b' | b'B') {
            self.at += 1;
        }
        match first {
            b'-' if self.eat(b'>') => TokenKind::Arrow,
            b'-' if self.eat(b'-') => self.comment_to_line_end(),
            b'-' => TokenKind::Minus,
            b'/' if self.eat(b'/') => self.comment_to_line_end(),
            b'/' if self.eat(b'*') => self.block_comment(),
            b'/' => TokenKind::Slash,
            b'#' if matches!(self.byte(self.at), Some(b' ' | b'!')) => self.comment_to_line_end(),
            b'=' => {
                self.eat(b'=');
                TokenKind::Equals
            }
            b'!' if self.eat(b'=') => TokenKind::NotEquals,
            b'!' => TokenKind::ErrorSingleExclamationMark,
            b'<' if self.bytes[self.at..].starts_with(b"=>") => {
                self.at += 2;
                TokenKind::Spaceship
            }
            b'<' if self.eat(b'=') => TokenKind::LessOrEquals,
            b'<' if self.eat(b'>') => TokenKind::NotEquals,
            b'<' => TokenKind::Less,
            b'>' if self.eat(b'=') => TokenKind::GreaterOrEquals,
            b'>' => TokenKind::Greater,
            b':' if self.eat(b':') => TokenKind::DoubleColon,
            b':' => TokenKind::Colon,
            b'|' if self.eat(b'|') => TokenKind::Concatenation,
            b'|' => TokenKind::PipeMark,
            b'@' if self.eat(b'@') => TokenKind::DoubleAt,
            b'@' => TokenKind::At,
            b'\\' if self.eat(b'G') => TokenKind::VerticalDelimiter,
            byte if byte.is_ascii() && !matches!(byte, b'$' | b'x' | b'X' | b'b' | b'B') => {
                match is_word_char(byte) {
                    true => self.word(),
                    false => TokenKind::Error,
                }
            }
            _ => self.other(start),
        }
    }

    /// A nested `/* ... */` comment after its `/*`; an unclosed one runs to the end as an error.
    fn block_comment(&mut self) -> TokenKind {
        let bytes: &[u8] = self.bytes;
        let mut depth: usize = 1;
        while self.at + 2 <= bytes.len() {
            if bytes[self.at..].starts_with(b"/*") {
                self.at += 2;
                depth += 1;
            } else if bytes[self.at..].starts_with(b"*/") {
                self.at += 2;
                depth -= 1;
                if depth == 0 {
                    return TokenKind::Comment;
                }
            } else {
                self.at += 1;
            }
        }
        self.at = bytes.len();
        TokenKind::ErrorMultilineCommentIsNotClosed
    }

    /// `quotedString<quote>`: backslash escapes and a doubled quote stay inside.
    fn quoted(&mut self, quote: u8, success: TokenKind, error: TokenKind) -> TokenKind {
        let bytes: &[u8] = self.bytes;
        let end: usize = bytes.len();
        self.at += 1;
        loop {
            while self.at < end && bytes[self.at] != quote && bytes[self.at] != b'\\' {
                self.at += 1;
            }
            if self.at >= end {
                return error;
            }
            self.at += 1;
            if bytes[self.at - 1] == quote {
                if self.eat(quote) {
                    continue;
                }
                return success;
            }
            if self.at >= end {
                return error;
            }
            self.at += 1;
        }
    }

    /// `quotedStringWithUnicodeQuotes`: from after the opening quote up to the closing one.
    fn unicode_quoted(&mut self, closing: char, success: TokenKind, error: TokenKind) -> TokenKind {
        match self.sql[self.at..].find(closing) {
            Some(offset) => {
                self.at += offset + closing.len_utf8();
                success
            }
            None => {
                self.at = self.bytes.len();
                error
            }
        }
    }

    /// The digits of one block, with `_` separators between them.
    fn digits(&mut self, mut start_of_block: bool, hex: bool) {
        while self.at < self.bytes.len()
            && (is_digit(self.bytes[self.at], hex)
                || is_number_separator(start_of_block, hex, self.bytes, self.at))
        {
            self.at += 1;
            start_of_block = false;
        }
    }

    /// An exponent marker, an optional sign and the exponent's digits, when the marker is next.
    fn exponent(&mut self, markers: [u8; 2]) {
        let end: usize = self.bytes.len();
        if self.at + 1 < end && markers.contains(&self.bytes[self.at]) {
            self.at += 1;
            if self.at + 1 < end && matches!(self.bytes[self.at], b'-' | b'+') {
                self.at += 1;
            }
            self.digits(true, false);
        }
    }

    /// A number, or a word starting with digits such as `1abc`.
    fn number(&mut self, start: usize) -> TokenKind {
        let bytes: &[u8] = self.bytes;
        if self.previous_significant == Some(TokenKind::Dot) {
            self.at += 1;
            self.digits(false, false);
        } else {
            let mut hex: bool = false;
            let mut start_of_block: bool = false;
            match (bytes.get(start + 1), bytes.get(start + 2)) {
                (Some(b'x' | b'X'), Some(next)) if bytes[start] == b'0' => {
                    hex = next.is_ascii_hexdigit();
                    start_of_block = hex;
                }
                (Some(b'b' | b'B'), Some(b'0' | b'1')) if bytes[start] == b'0' => {
                    start_of_block = true;
                }
                _ => {}
            }
            self.at += match start_of_block {
                true => 2,
                false => 1,
            };
            self.digits(start_of_block, hex);
            if self.eat(b'.') {
                self.digits(true, hex);
            }
            self.exponent(match hex {
                true => *b"pP",
                false => *b"eE",
            });
        }
        if self.at < bytes.len() && is_word_char(bytes[self.at]) {
            self.at += 1;
            while self.at < bytes.len() && is_word_char(bytes[self.at]) {
                self.at += 1;
            }
            return match bytes[start..self.at]
                .iter()
                .all(|byte| is_word_char(*byte) || *byte == b'$')
            {
                true => TokenKind::BareWord,
                false => TokenKind::ErrorWrongNumber,
            };
        }
        TokenKind::Number
    }

    /// `.`: a dot after a name, a closing bracket or a number, or before a non-digit; else a number.
    fn dot(&mut self, start: usize) -> TokenKind {
        let digit_next: bool = self
            .byte(start + 1)
            .is_some_and(|byte| byte.is_ascii_digit());
        let after_operand: bool = matches!(
            self.previous_significant,
            Some(
                TokenKind::ClosingRoundBracket
                    | TokenKind::ClosingSquareBracket
                    | TokenKind::BareWord
                    | TokenKind::QuotedIdentifier
                    | TokenKind::Number
            )
        );
        self.at += 1;
        if start > 0 && (!digit_next || after_operand) {
            return TokenKind::Dot;
        }
        self.digits(true, false);
        self.exponent(*b"eE");
        TokenKind::Number
    }

    /// An unquoted word: word characters and `$`.
    fn word(&mut self) -> TokenKind {
        while self.at < self.bytes.len()
            && (is_word_char(self.bytes[self.at]) || self.bytes[self.at] == b'$')
        {
            self.at += 1;
        }
        TokenKind::BareWord
    }

    /// Heredocs, `$`, `x'..'` and `b'..'` strings, words, and the rules for non-ASCII characters.
    fn other(&mut self, start: usize) -> TokenKind {
        let bytes: &[u8] = self.bytes;
        if bytes[start] == b'$' {
            if let Some(kind) = self.heredoc(start) {
                return kind;
            }
            if bytes.get(start + 1).is_none_or(|next| !is_word_char(*next)) {
                return self.single(TokenKind::DollarSign);
            }
        }
        if bytes.get(start + 1) == Some(&b'\'') && start + 2 < bytes.len() {
            match bytes[start] {
                b'x' | b'X' => return self.bit_string(start, true),
                b'b' | b'B' => return self.bit_string(start, false),
                _ => {}
            }
        }
        if bytes[start].is_ascii() {
            self.at += 1;
            return self.word();
        }
        self.unicode(start)
    }

    /// `$tag$ ... $tag$`, when a tag of word characters opens it and the same tag closes it.
    fn heredoc(&mut self, start: usize) -> Option<TokenKind> {
        let bytes: &[u8] = self.bytes;
        let tag_end: usize =
            start + 1 + bytes[start + 1..].iter().position(|byte| *byte == b'$')?;
        if !bytes[start + 1..tag_end]
            .iter()
            .all(|byte| is_word_char(*byte))
        {
            return None;
        }
        let tag: &[u8] = &bytes[start..=tag_end];
        let found: usize = bytes[tag_end + 1..]
            .windows(tag.len())
            .position(|window| window == tag)?;
        self.at = tag_end + 1 + found + tag.len();
        Some(TokenKind::HereDoc)
    }

    /// `x'...'` or `b'...'`: hex or binary digits, then the closing quote.
    fn bit_string(&mut self, start: usize, hex: bool) -> TokenKind {
        self.at = start + 2;
        while self.at < self.bytes.len()
            && match hex {
                true => self.bytes[self.at].is_ascii_hexdigit(),
                false => matches!(self.bytes[self.at], b'0' | b'1'),
            }
        {
            self.at += 1;
        }
        if !self.eat(b'\'') {
            self.at = self.bytes.len();
            return TokenKind::ErrorSingleQuoteIsNotClosed;
        }
        TokenKind::StringLiteral
    }

    /// A non-ASCII character: the minus sign, a Unicode-quoted token, blanks, or an error.
    fn unicode(&mut self, start: usize) -> TokenKind {
        let rest: &str = &self.sql[start..];
        let Some(character) = rest.chars().next() else {
            return self.single(TokenKind::Error);
        };
        self.at += character.len_utf8();
        if character == UNICODE_MINUS {
            return TokenKind::Minus;
        }
        if rest.len() >= UNICODE_QUOTED_MIN_BYTES {
            if character == UNICODE_SINGLE_QUOTES.0 {
                return self.unicode_quoted(
                    UNICODE_SINGLE_QUOTES.1,
                    TokenKind::StringLiteral,
                    TokenKind::ErrorSingleQuoteIsNotClosed,
                );
            }
            if character == UNICODE_DOUBLE_QUOTES.0 {
                return self.unicode_quoted(
                    UNICODE_DOUBLE_QUOTES.1,
                    TokenKind::QuotedIdentifier,
                    TokenKind::ErrorDoubleQuoteIsNotClosed,
                );
            }
        }
        if !UNICODE_BLANKS.contains(&character) {
            return TokenKind::Error;
        }
        while let Some(next) = self.sql[self.at..].chars().next() {
            if next.is_ascii() && is_blank(next as u8) || UNICODE_BLANKS.contains(&next) {
                self.at += next.len_utf8();
            } else {
                break;
            }
        }
        TokenKind::Whitespace
    }
}
