//! The scanner: SqlBaseLexer.g4's rules, longest match first, over the input's characters.

use sqlquack_syntax::tokens::models::Span;

use crate::scan::_helpers::strings::{decode_quoted, quoted_end};
use crate::scan::constants::{MULTI_CHAR_OPERATORS, UNCLOSED_COMMENT, UNICODE_BLANKS};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// A lexical fault: its message and where it is. Every lexical fault is a syntax error.
pub(crate) struct Fault {
    pub(crate) message: String,
    pub(crate) position: usize,
}

/// `IDENTIFIER`'s characters: a Unicode letter, a digit or `_`.
pub(crate) fn is_word_char(character: char) -> bool {
    character.is_alphabetic() || character.is_ascii_digit() || character == '_'
}

fn is_blank(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000C}')
        || UNICODE_BLANKS.contains(&character)
}

/// Whether the byte after a decimal token lets it stand (`isValidDecimal`).
fn valid_decimal_end(next: Option<u8>) -> bool {
    !next.is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub(crate) struct Scanner<'sql> {
    sql: &'sql str,
    bytes: &'sql [u8],
    at: usize,
    /// The ranges no token covers but a blank or comment does.
    pub(crate) skipped: Vec<(usize, usize)>,
}

impl<'sql> Scanner<'sql> {
    pub(crate) fn new(sql: &'sql str) -> Self {
        Self {
            sql,
            bytes: sql.as_bytes(),
            at: 0,
            skipped: Vec::new(),
        }
    }

    fn byte_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.at + offset).copied()
    }

    fn char_at(&self, position: usize) -> Option<char> {
        self.sql
            .get(position..)
            .and_then(|rest| rest.chars().next())
    }

    fn token(&self, kind: TokenKind, value: String, start: usize) -> Token {
        Token {
            kind,
            value,
            start,
            end: self.at,
            span: Span::default(),
        }
    }

    /// Every token up to the end or the first fault.
    pub(crate) fn scan(mut self) -> (Vec<Token>, Option<Fault>, Vec<(usize, usize)>) {
        let mut tokens: Vec<Token> = Vec::new();
        loop {
            match self.skip_blanks() {
                Ok(()) => {}
                Err(fault) => return (tokens, Some(fault), self.skipped),
            }
            if self.at >= self.bytes.len() {
                return (tokens, None, self.skipped);
            }
            match self.next_token() {
                Ok(token) => tokens.push(token),
                Err(fault) => return (tokens, Some(fault), self.skipped),
            }
        }
    }

    /// Skips blanks and comments (`WS`, `SIMPLE_COMMENT`, `BRACKETED_COMMENT`).
    fn skip_blanks(&mut self) -> Result<(), Fault> {
        loop {
            let start: usize = self.at;
            match self.char_at(self.at) {
                Some(character) if is_blank(character) => {
                    self.at += character.len_utf8();
                }
                Some('-') if self.byte_at(1) == Some(b'-') => {
                    self.at += 2;
                    while let Some(byte) = self.byte_at(0) {
                        if byte == b'\\' && self.byte_at(1) == Some(b'\n') {
                            self.at += 2;
                        } else if byte == b'\r' || byte == b'\n' {
                            break;
                        } else {
                            self.at += 1;
                        }
                    }
                }
                Some('/') if self.byte_at(1) == Some(b'*') && self.byte_at(2) != Some(b'+') => {
                    self.bracketed_comment()?;
                }
                _ => return Ok(()),
            }
            self.skipped.push((start, self.at));
        }
    }

    /// A nested `/* ... */` comment; an unclosed one is a syntax error.
    fn bracketed_comment(&mut self) -> Result<(), Fault> {
        let start: usize = self.at;
        let mut depth: usize = 0;
        while self.at < self.bytes.len() {
            if self.byte_at(0) == Some(b'/') && self.byte_at(1) == Some(b'*') {
                depth += 1;
                self.at += 2;
            } else if self.byte_at(0) == Some(b'*') && self.byte_at(1) == Some(b'/') {
                depth -= 1;
                self.at += 2;
                if depth == 0 {
                    return Ok(());
                }
            } else {
                self.at += 1;
            }
        }
        Err(Fault {
            message: UNCLOSED_COMMENT.to_owned(),
            position: start,
        })
    }

    fn next_token(&mut self) -> Result<Token, Fault> {
        let start: usize = self.at;
        let Some(character) = self.char_at(start) else {
            return Err(Fault {
                message: "invalid input".to_owned(),
                position: start,
            });
        };
        if (character == 'r' || character == 'R') && matches!(self.byte_at(1), Some(b'\'' | b'"')) {
            let quote: u8 = self.bytes[start + 1];
            return self.raw_string(start, quote);
        }
        if character == '\'' || character == '"' {
            return self.string(start, character as u8);
        }
        if character == '`' {
            return self.quoted_identifier(start);
        }
        if (character.is_ascii_digit()
            || (character == '.' && self.byte_at(1).is_some_and(|byte| byte.is_ascii_digit())))
            && let Some(token) = self.number(start)
        {
            return Ok(token);
        }
        if is_word_char(character) {
            while let Some(next) = self.char_at(self.at) {
                if !is_word_char(next) {
                    break;
                }
                self.at += next.len_utf8();
            }
            return Ok(self.token(TokenKind::Word, self.sql[start..self.at].to_owned(), start));
        }
        if character == '$'
            && let Some(token) = self.dollar_string(start)?
        {
            return Ok(token);
        }
        if self.sql[start..].starts_with("/*+") {
            self.at += 3;
            return Ok(self.token(TokenKind::HintStart, "/*+".to_owned(), start));
        }
        if self.sql[start..].starts_with("*/") {
            self.at += 2;
            return Ok(self.token(TokenKind::HintEnd, "*/".to_owned(), start));
        }
        for operator in MULTI_CHAR_OPERATORS {
            if self.sql[start..].starts_with(operator) {
                self.at += operator.len();
                return Ok(self.token(TokenKind::Op, operator.to_owned(), start));
            }
        }
        self.at += character.len_utf8();
        Ok(self.token(TokenKind::Char(character), character.to_string(), start))
    }

    fn string(&mut self, start: usize, quote: u8) -> Result<Token, Fault> {
        match quoted_end(self.bytes, start, quote, true) {
            Some(end) => {
                self.at = end;
                let value: String = decode_quoted(&self.sql[start + 1..end - 1], quote as char);
                Ok(self.token(TokenKind::String { raw: false }, value, start))
            }
            None => Err(Fault {
                message: "unclosed string literal".to_owned(),
                position: start,
            }),
        }
    }

    fn raw_string(&mut self, start: usize, quote: u8) -> Result<Token, Fault> {
        let body: usize = start + 2;
        match self.bytes[body..].iter().position(|byte| *byte == quote) {
            Some(length) => {
                self.at = body + length + 1;
                let value: String = self.sql[body..body + length].to_owned();
                Ok(self.token(TokenKind::String { raw: true }, value, start))
            }
            None => Err(Fault {
                message: "unclosed string literal".to_owned(),
                position: start,
            }),
        }
    }

    fn quoted_identifier(&mut self, start: usize) -> Result<Token, Fault> {
        match quoted_end(self.bytes, start, b'`', false) {
            Some(end) => {
                self.at = end;
                let value: String = self.sql[start + 1..end - 1].replace("``", "`");
                Ok(self.token(TokenKind::Quoted, value, start))
            }
            None => Err(Fault {
                message: "unclosed quoted identifier".to_owned(),
                position: start,
            }),
        }
    }

    /// `$tag$ ... $tag$`; `None` when `$` starts no tag, so it is a lone character.
    fn dollar_string(&mut self, start: usize) -> Result<Option<Token>, Fault> {
        let letters: usize = self.bytes[start + 1..]
            .iter()
            .take_while(|byte| byte.is_ascii_alphabetic())
            .count();
        if self.bytes.get(start + 1 + letters) != Some(&b'$') {
            return Ok(None);
        }
        let tag: &str = &self.sql[start..start + letters + 2];
        let body: usize = start + tag.len();
        match self.sql[body..].find(tag) {
            Some(length) => {
                self.at = body + length + tag.len();
                let value: String = self.sql[body..body + length].to_owned();
                Ok(Some(self.token(TokenKind::DollarString, value, start)))
            }
            None => Err(Fault {
                message: "unclosed dollar-quoted string".to_owned(),
                position: start,
            }),
        }
    }

    /// The longest numeric literal at `start`, unless `IDENTIFIER` matches more.
    fn number(&mut self, start: usize) -> Option<Token> {
        let bytes: &[u8] = self.bytes;
        let digits_from = |position: usize| -> usize {
            bytes[position..]
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count()
        };
        let exponent_at = |position: usize| -> Option<usize> {
            if !matches!(bytes.get(position), Some(b'e' | b'E')) {
                return None;
            }
            let signed: usize =
                position + 1 + usize::from(matches!(bytes.get(position + 1), Some(b'+' | b'-')));
            let count: usize = digits_from(signed);
            (count > 0).then_some(signed + count)
        };
        let suffix_at = |position: usize| -> Option<(usize, NumberKind)> {
            match bytes.get(position).map(u8::to_ascii_uppercase) {
                Some(b'B') if matches!(bytes.get(position + 1), Some(b'd' | b'D')) => {
                    Some((position + 2, NumberKind::BigDecimal))
                }
                Some(b'D') => Some((position + 1, NumberKind::Double)),
                Some(b'F') => Some((position + 1, NumberKind::Float)),
                _ => None,
            }
        };
        let integer: usize = digits_from(start);
        let mut best: Option<(usize, NumberKind)> = None;
        let mut consider = |end: usize, kind: NumberKind, checked: bool| {
            if checked && !valid_decimal_end(bytes.get(end).copied()) {
                return;
            }
            if best.is_none_or(|(longest, _)| end > longest) {
                best = Some((end, kind));
            }
        };
        if integer > 0 {
            let end: usize = start + integer;
            consider(end, NumberKind::Integer, false);
            match bytes.get(end).map(u8::to_ascii_uppercase) {
                Some(b'L') => consider(end + 1, NumberKind::BigInt, false),
                Some(b'S') => consider(end + 1, NumberKind::SmallInt, false),
                Some(b'Y') => consider(end + 1, NumberKind::TinyInt, false),
                _ => {}
            }
            if let Some((suffixed, kind)) = suffix_at(end) {
                consider(suffixed, kind, false);
            }
            if let Some(exponent) = exponent_at(end) {
                consider(exponent, NumberKind::Exponent, false);
                if let Some((suffixed, kind)) = suffix_at(exponent) {
                    consider(suffixed, kind, false);
                }
            }
        }
        if bytes.get(start + integer) == Some(&b'.') {
            let fraction: usize = digits_from(start + integer + 1);
            if integer > 0 || fraction > 0 {
                let end: usize = start + integer + 1 + fraction;
                consider(end, NumberKind::Decimal, true);
                if let Some((suffixed, kind)) = suffix_at(end) {
                    consider(suffixed, kind, true);
                }
                if let Some(exponent) = exponent_at(end) {
                    consider(exponent, NumberKind::Exponent, true);
                    if let Some((suffixed, kind)) = suffix_at(exponent) {
                        consider(suffixed, kind, true);
                    }
                }
            }
        }
        let (end, kind) = best?;
        let mut word_end: usize = start;
        while let Some(next) = self.char_at(word_end) {
            if !is_word_char(next) {
                break;
            }
            word_end += next.len_utf8();
        }
        if word_end > end {
            return None;
        }
        self.at = end;
        Some(self.token(
            TokenKind::Number(kind),
            self.sql[start..end].to_owned(),
            start,
        ))
    }
}
